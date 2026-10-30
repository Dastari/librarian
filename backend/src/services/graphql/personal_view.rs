//! Versioned, typed saved-filter compiler. Public callers obtain authorized
//! library IDs through generated Library queries; trusted backend callers must
//! establish the same scope before using this helper. Its output is accepted by
//! generated entity queries, including their complete dataset counts/paging.
//! It is intentionally not a custom data-query resolver.

use async_graphql::{Error, Result};
use graphql_orm::graphql::filters::{BoolFilter, StringFilter};
use graphql_orm::graphql::orm::OrderDirection;

use crate::graphql::entities::*;

#[derive(Clone, Debug)]
pub enum PersonalViewQuery {
    Movie(MovieWhereInput, MovieOrderByInput),
    Show(ShowWhereInput, ShowOrderByInput),
    Album(AlbumWhereInput, AlbumOrderByInput),
    Audiobook(AudiobookWhereInput, AudiobookOrderByInput),
    Track(TrackWhereInput, TrackOrderByInput),
}

/// Compile version 1 without accepting arbitrary field names, operators or JSON.
/// `authorized_library_ids` must contain only the caller's own library IDs, even
/// for an administrator. Empty scope is an error, never an omitted IN filter
/// (the generated empty-list filter means no predicate).
pub fn compile_saved_view(
    view: &SavedView,
    caller_user_id: &str,
    authorized_library_ids: &[String],
) -> Result<PersonalViewQuery> {
    super::personal_policy::validate_saved_view(view)?;
    if caller_user_id != view.user_id || caller_user_id.is_empty() {
        return Err(Error::new("Saved view is not owned by the caller"));
    }
    if authorized_library_ids.is_empty() {
        return Err(Error::new("Saved view has no authorized libraries"));
    }
    let library = if let Some(id) = &view.library_id {
        if !authorized_library_ids.contains(id) {
            return Err(Error::new("Saved view library is not owned by the caller"));
        }
        StringFilter {
            eq: Some(id.clone()),
            ..Default::default()
        }
    } else {
        StringFilter {
            in_list: Some(authorized_library_ids.to_vec()),
            ..Default::default()
        }
    };
    let owner = StringFilter {
        eq: Some(caller_user_id.to_owned()),
        ..Default::default()
    };
    let title = view
        .title_contains
        .as_ref()
        .filter(|value| !value.is_empty())
        .map(|value| StringFilter {
            contains: Some(value.clone()),
            ..Default::default()
        });
    let available = match view.availability {
        PersonalAvailability::Any => None,
        PersonalAvailability::Available => Some(BoolFilter {
            eq: Some(true),
            ..Default::default()
        }),
        PersonalAvailability::Missing => Some(BoolFilter {
            eq: Some(false),
            ..Default::default()
        }),
    };
    let direction = if view.descending {
        OrderDirection::Desc
    } else {
        OrderDirection::Asc
    };
    // Media ID ordering is intentionally absent: those entity order inputs do
    // not expose id. Each sort uses its actual generated title/name/date field.
    macro_rules! order {
        ($ty:ident, $title:ident) => {{
            let mut order = $ty::default();
            match view.sort {
                PersonalSort::Title => order.$title = Some(direction),
                PersonalSort::CreatedAt => order.created_at = Some(direction),
                PersonalSort::UpdatedAt => order.updated_at = Some(direction),
            }
            order
        }};
    }
    Ok(match view.content_type {
        PersonalContentType::Movie => PersonalViewQuery::Movie(
            MovieWhereInput {
                user_id: Some(owner),
                library_id: Some(library),
                title,
                has_file: available,
                ..Default::default()
            },
            order!(MovieOrderByInput, title),
        ),
        PersonalContentType::Show => PersonalViewQuery::Show(
            ShowWhereInput {
                user_id: Some(owner),
                library_id: Some(library),
                name: title,
                ..Default::default()
            },
            order!(ShowOrderByInput, name),
        ),
        PersonalContentType::Album => PersonalViewQuery::Album(
            AlbumWhereInput {
                user_id: Some(owner),
                library_id: Some(library),
                name: title,
                has_files: available,
                ..Default::default()
            },
            order!(AlbumOrderByInput, name),
        ),
        PersonalContentType::Audiobook => PersonalViewQuery::Audiobook(
            AudiobookWhereInput {
                user_id: Some(owner),
                library_id: Some(library),
                title,
                has_files: available,
                ..Default::default()
            },
            order!(AudiobookOrderByInput, title),
        ),
        PersonalContentType::Track => PersonalViewQuery::Track(
            TrackWhereInput {
                library_id: Some(library),
                title,
                media_file_id: match view.availability {
                    PersonalAvailability::Any => None,
                    PersonalAvailability::Available => Some(StringFilter {
                        is_null: Some(false),
                        ..Default::default()
                    }),
                    PersonalAvailability::Missing => Some(StringFilter {
                        is_null: Some(true),
                        ..Default::default()
                    }),
                },
                ..Default::default()
            },
            order!(TrackOrderByInput, title),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> SavedView {
        SavedView {
            id: "view".into(),
            user_id: "owner".into(),
            name: "Saved".into(),
            version: 1,
            content_type: PersonalContentType::Movie,
            library_id: Some("library".into()),
            title_contains: Some("Night".into()),
            availability: PersonalAvailability::Available,
            sort: PersonalSort::Title,
            descending: false,
            created_at: String::new(),
            updated_at: String::new(),
            library: None,
        }
    }

    #[test]
    fn compiler_restricts_scope_versions_and_unsupported_show_availability() {
        let mut row = view();
        assert!(compile_saved_view(&row, "foreign", &["library".into()]).is_err());
        assert!(compile_saved_view(&row, "owner", &[]).is_err());
        assert!(compile_saved_view(&row, "owner", &["foreign-library".into()]).is_err());
        row.version = 2;
        assert!(compile_saved_view(&row, "owner", &["library".into()]).is_err());
        row.version = 1;
        row.content_type = PersonalContentType::Show;
        assert!(compile_saved_view(&row, "owner", &["library".into()]).is_err());
    }

    #[test]
    fn compiler_matches_supported_generated_fields_for_every_content_type() {
        let mut row = view();
        let libs = ["library".into()];
        match compile_saved_view(&row, "owner", &libs).unwrap() {
            PersonalViewQuery::Movie(filter, order) => {
                assert_eq!(filter.user_id.unwrap().eq.as_deref(), Some("owner"));
                assert_eq!(filter.library_id.unwrap().eq.as_deref(), Some("library"));
                assert_eq!(filter.title.unwrap().contains.as_deref(), Some("Night"));
                assert_eq!(filter.has_file.unwrap().eq, Some(true));
                assert!(matches!(order.title, Some(OrderDirection::Asc)));
            }
            _ => panic!("expected movie query"),
        }
        row.content_type = PersonalContentType::Show;
        row.availability = PersonalAvailability::Any;
        assert!(
            matches!(compile_saved_view(&row, "owner", &libs).unwrap(), PersonalViewQuery::Show(filter, _) if filter.name.is_some())
        );
        row.content_type = PersonalContentType::Album;
        row.availability = PersonalAvailability::Missing;
        assert!(
            matches!(compile_saved_view(&row, "owner", &libs).unwrap(), PersonalViewQuery::Album(filter, _) if filter.has_files.as_ref().unwrap().eq == Some(false))
        );
        row.content_type = PersonalContentType::Audiobook;
        assert!(
            matches!(compile_saved_view(&row, "owner", &libs).unwrap(), PersonalViewQuery::Audiobook(filter, _) if filter.has_files.as_ref().unwrap().eq == Some(false))
        );
        row.content_type = PersonalContentType::Track;
        row.sort = PersonalSort::UpdatedAt;
        row.descending = true;
        assert!(
            matches!(compile_saved_view(&row, "owner", &libs).unwrap(), PersonalViewQuery::Track(filter, order) if filter.media_file_id.as_ref().unwrap().is_null == Some(true) && matches!(order.updated_at, Some(OrderDirection::Desc)))
        );
    }
}
