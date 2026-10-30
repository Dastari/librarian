//! Private personal rows deliberately do not inherit the general administrator
//! override. Validation runs in the originating generated transaction, so a
//! failed link or invariant rolls back writes and invalidations together.

use std::any::Any;

use async_graphql::{Context, Error, ErrorExtensions, Result};
use graphql_orm::DefaultBackend;
use graphql_orm::graphql::filters::StringFilter;
use graphql_orm::graphql::orm::{
    ChangeAction, MutationContext, MutationEvent, MutationPhase, ReadPredicate,
};

use crate::db::Database;
use crate::graphql::AuthUser;
use crate::graphql::entities::*;
use crate::services::graphql::auth::Role;

pub const WATCH_LATER_NAME: &str = "Watch Later";
pub const SAVED_VIEW_VERSION: i32 = 1;

type Row = dyn Any + Send + Sync;

fn denied() -> Error {
    Error::new("Personal record or linked target not found or access denied")
        .extend_with(|_, e| e.set("code", "FORBIDDEN"))
}
fn invalid(message: &str) -> Error {
    Error::new(message).extend_with(|_, e| e.set("code", "BAD_USER_INPUT"))
}
fn db_error(error: impl std::fmt::Display) -> Error {
    Error::new(error.to_string())
}
fn eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_owned()),
        ..Default::default()
    }
}

pub(crate) fn is_personal(entity: &str) -> bool {
    matches!(
        entity,
        "PersonalList" | "ListEntry" | "SavedView" | "HomeShelf" | "LibraryPreference"
    )
}

pub(crate) fn owner<'a>(entity: &str, row: &'a Row) -> Option<&'a str> {
    macro_rules! get {
        ($ty:ty) => {
            row.downcast_ref::<$ty>().map(|row| row.user_id.as_str())
        };
    }
    match entity {
        "PersonalList" => get!(PersonalList),
        "ListEntry" => get!(ListEntry),
        "SavedView" => get!(SavedView),
        "HomeShelf" => get!(HomeShelf),
        "LibraryPreference" => get!(LibraryPreference),
        _ => None,
    }
}

/// A complete database predicate is applied before counts, cursors and offsets.
pub(crate) fn read_predicate(entity: &str, user: &str) -> Result<Option<ReadPredicate>> {
    macro_rules! predicate {
        ($ty:ident) => {
            ReadPredicate::from_filter::<DefaultBackend, _>(&$ty {
                user_id: Some(eq(user)),
                ..Default::default()
            })?
        };
    }
    Ok(Some(match entity {
        "PersonalList" => predicate!(PersonalListWhereInput),
        "ListEntry" => predicate!(ListEntryWhereInput),
        "SavedView" => predicate!(SavedViewWhereInput),
        "HomeShelf" => predicate!(HomeShelfWhereInput),
        "LibraryPreference" => predicate!(LibraryPreferenceWhereInput),
        _ => return Ok(None),
    }))
}

pub(crate) fn prepare_create(
    ctx: Option<&Context<'_>>,
    entity: &str,
    input: &mut Row,
) -> Result<()> {
    if !is_personal(entity) {
        return Ok(());
    }
    let Some(ctx) = ctx else {
        return Ok(());
    };
    let user = ctx
        .data_opt::<AuthUser>()
        .filter(|u| u.has_role(Role::Member))
        .ok_or_else(denied)?;
    macro_rules! assign {
        ($ty:ty) => {
            input.downcast_mut::<$ty>().ok_or_else(denied)?.user_id = user.user_id.clone()
        };
    }
    match entity {
        "PersonalList" => assign!(CreatePersonalListInput),
        "ListEntry" => assign!(CreateListEntryInput),
        "SavedView" => assign!(CreateSavedViewInput),
        "HomeShelf" => assign!(CreateHomeShelfInput),
        "LibraryPreference" => assign!(CreateLibraryPreferenceInput),
        _ => unreachable!(),
    }
    Ok(())
}

pub(crate) fn prepare_update(
    ctx: Option<&Context<'_>>,
    entity: &str,
    existing: Option<&Row>,
    input: &Row,
) -> Result<()> {
    if !is_personal(entity) || ctx.is_none() {
        return Ok(());
    }
    let user = ctx
        .and_then(|c| c.data_opt::<AuthUser>())
        .filter(|u| u.has_role(Role::Member))
        .ok_or_else(denied)?;
    if existing.and_then(|row| owner(entity, row)) != Some(user.user_id.as_str()) {
        return Err(denied());
    }
    // Reject supplied immutable fields explicitly. Returning false from a field
    // policy may silently strip an input, which is unsuitable for associations.
    let immutable = match entity {
        "PersonalList" => {
            let v = input
                .downcast_ref::<UpdatePersonalListInput>()
                .ok_or_else(denied)?;
            v.user_id.is_some() || v.kind.is_some()
        }
        "ListEntry" => {
            let v = input
                .downcast_ref::<UpdateListEntryInput>()
                .ok_or_else(denied)?;
            v.user_id.is_some()
                || v.list_id.is_some()
                || v.movie_id.is_some()
                || v.show_id.is_some()
                || v.album_id.is_some()
                || v.audiobook_id.is_some()
                || v.track_id.is_some()
        }
        "SavedView" => input
            .downcast_ref::<UpdateSavedViewInput>()
            .ok_or_else(denied)?
            .user_id
            .is_some(),
        "HomeShelf" => {
            let v = input
                .downcast_ref::<UpdateHomeShelfInput>()
                .ok_or_else(denied)?;
            v.user_id.is_some() || v.shelf_key.is_some()
        }
        "LibraryPreference" => {
            let v = input
                .downcast_ref::<UpdateLibraryPreferenceInput>()
                .ok_or_else(denied)?;
            v.user_id.is_some() || v.library_id.is_some()
        }
        _ => false,
    };
    if immutable {
        Err(invalid(
            "Personal ownership, list kind and stable associations cannot be changed",
        ))
    } else {
        Ok(())
    }
}

/// Called before AppFieldPolicy's generic administrator override. Every
/// rejection is an error, including nullable links and explicit null updates.
pub(crate) async fn write_field(
    ctx: &Context<'_>,
    _db: &Database,
    entity: &str,
    field: &str,
    policy: Option<&str>,
    row: Option<&Row>,
    value: Option<&Row>,
) -> Result<bool> {
    let user = ctx
        .data_opt::<AuthUser>()
        .filter(|u| u.has_role(Role::Member))
        .ok_or_else(denied)?;
    if row.is_some_and(|row| owner(entity, row) != Some(user.user_id.as_str())) {
        return Err(denied());
    }
    let text = value.and_then(|v| {
        v.downcast_ref::<String>().map(String::as_str).or_else(|| {
            v.downcast_ref::<Option<String>>()
                .and_then(Option::as_deref)
        })
    });
    match policy {
        Some("owner.id") => {
            if row.is_none() && text == Some(user.user_id.as_str()) {
                Ok(true)
            } else {
                Err(denied())
            }
        }
        Some("owned.link") => {
            if row.is_some() && (entity == "ListEntry" || entity == "LibraryPreference") {
                return Err(denied());
            }
            let Some(target) = text else {
                return if value.is_some_and(|v| {
                    v.downcast_ref::<Option<String>>()
                        .is_some_and(Option::is_none)
                }) {
                    Ok(true)
                } else {
                    Err(denied())
                };
            };
            // The generated update already holds a transaction here. Reading
            // a second pool connection can deadlock a single-connection pool
            // and would race the actual write. Validate every target owner in
            // validate_row on that same transaction before events can commit.
            // A hook rejection is a hard error and rolls back the entire write.
            if target.is_empty()
                || !matches!(
                    field,
                    "listId"
                        | "savedViewId"
                        | "libraryId"
                        | "movieId"
                        | "showId"
                        | "albumId"
                        | "audiobookId"
                        | "trackId"
                )
            {
                Err(denied())
            } else {
                Ok(true)
            }
        }
        None | Some("member.write") => Ok(true),
        _ => Err(denied()),
    }
}

fn nonblank(value: &str, max: usize, message: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > max {
        Err(invalid(message))
    } else {
        Ok(())
    }
}
fn position(value: i32) -> Result<()> {
    if value < 0 {
        Err(invalid("Personal ordering position must be nonnegative"))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_saved_view(view: &SavedView) -> Result<()> {
    nonblank(
        &view.name,
        200,
        "Saved view name must contain 1 to 200 bytes",
    )?;
    if view.version != SAVED_VIEW_VERSION {
        return Err(invalid("Unsupported saved view version; expected 1"));
    }
    if view.title_contains.as_ref().is_some_and(|v| v.len() > 500) {
        return Err(invalid("Saved title filter must not exceed 500 bytes"));
    }
    if view.content_type == PersonalContentType::Show
        && view.availability != PersonalAvailability::Any
    {
        return Err(invalid("Show saved views support ANY availability only"));
    }
    Ok(())
}

async fn library_owned(tx: &mut MutationContext<'_>, id: &str, user: &str) -> Result<()> {
    if tx
        .find_by_id::<Library>(&id.to_owned())
        .await
        .map_err(db_error)?
        .is_some_and(|row| row.user_id == user)
    {
        Ok(())
    } else {
        Err(denied())
    }
}

/// Repository reads are trusted. Compare both title ownership and its actual
/// library; a forged redundant user_id on catalogue data is never sufficient.
async fn validate_entry(tx: &mut MutationContext<'_>, row: &ListEntry) -> Result<()> {
    position(row.position)?;
    let count = [
        &row.movie_id,
        &row.show_id,
        &row.album_id,
        &row.audiobook_id,
        &row.track_id,
    ]
    .into_iter()
    .filter(|id| id.is_some())
    .count();
    if count != 1 {
        return Err(invalid(
            "A list entry must reference exactly one movie, show, album, audiobook or track",
        ));
    }
    let list = tx
        .find_by_id::<PersonalList>(&row.list_id)
        .await
        .map_err(db_error)?
        .filter(|v| v.user_id == row.user_id)
        .ok_or_else(denied)?;
    if list.kind == PersonalListKind::Smart {
        return Err(invalid(
            "Smart lists use saved filters and cannot contain manual entries",
        ));
    }
    macro_rules! title {
        ($field:ident, $ty:ty) => {
            if let Some(id) = &row.$field {
                let target = tx
                    .find_by_id::<$ty>(id)
                    .await
                    .map_err(db_error)?
                    .filter(|v| v.user_id == row.user_id)
                    .ok_or_else(denied)?;
                library_owned(tx, &target.library_id, &row.user_id).await?;
            }
        };
    }
    title!(movie_id, Movie);
    title!(show_id, Show);
    title!(album_id, Album);
    title!(audiobook_id, Audiobook);
    if let Some(id) = &row.track_id {
        let track = tx
            .find_by_id::<Track>(id)
            .await
            .map_err(db_error)?
            .ok_or_else(denied)?;
        library_owned(tx, &track.library_id, &row.user_id).await?;
        let album = tx
            .find_by_id::<Album>(&track.album_id)
            .await
            .map_err(db_error)?
            .filter(|album| album.user_id == row.user_id && album.library_id == track.library_id)
            .ok_or_else(denied)?;
        let _ = album;
    }
    Ok(())
}

async fn validate_row(tx: &mut MutationContext<'_>, event: &MutationEvent) -> Result<()> {
    match event.entity_name {
        "PersonalList" => {
            let row = event.after::<PersonalList>()?.ok_or_else(denied)?;
            nonblank(&row.name, 200, "List name must contain 1 to 200 bytes")?;
            if row.name.trim() != row.name {
                return Err(invalid("List names cannot start or end with whitespace"));
            }
            if row.description.as_ref().is_some_and(|v| v.len() > 4000) {
                return Err(invalid("List description must not exceed 4000 bytes"));
            }
            if (row.kind == PersonalListKind::WatchLater) != (row.name == WATCH_LATER_NAME) {
                return Err(invalid("Watch Later uses the reserved name 'Watch Later'"));
            }
            if (row.kind == PersonalListKind::Smart) != row.saved_view_id.is_some() {
                return Err(invalid("Only smart lists must reference a saved view"));
            }
            if let Some(id) = &row.saved_view_id
                && tx
                    .find_by_id::<SavedView>(id)
                    .await
                    .map_err(db_error)?
                    .is_none_or(|v| v.user_id != row.user_id)
            {
                return Err(denied());
            }
            if event
                .before::<PersonalList>()?
                .is_some_and(|old| old.user_id != row.user_id || old.kind != row.kind)
            {
                return Err(invalid("Personal ownership and list kind are immutable"));
            }
        }
        "ListEntry" => {
            let row = event.after::<ListEntry>()?.ok_or_else(denied)?;
            if event.before::<ListEntry>()?.is_some_and(|old| {
                old.user_id != row.user_id
                    || old.list_id != row.list_id
                    || old.movie_id != row.movie_id
                    || old.show_id != row.show_id
                    || old.album_id != row.album_id
                    || old.audiobook_id != row.audiobook_id
                    || old.track_id != row.track_id
            }) {
                return Err(invalid(
                    "List entry ownership and content associations are immutable",
                ));
            }
            validate_entry(tx, row).await?;
        }
        "SavedView" => {
            let row = event.after::<SavedView>()?.ok_or_else(denied)?;
            validate_saved_view(row)?;
            if let Some(id) = &row.library_id {
                library_owned(tx, id, &row.user_id).await?;
            }
            if event
                .before::<SavedView>()?
                .is_some_and(|old| old.user_id != row.user_id)
            {
                return Err(denied());
            }
        }
        "HomeShelf" => {
            let row = event.after::<HomeShelf>()?.ok_or_else(denied)?;
            position(row.position)?;
            nonblank(&row.shelf_key, 160, "Shelf key must contain 1 to 160 bytes")?;
            if !row
                .shelf_key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b':'))
            {
                return Err(invalid(
                    "Shelf key may contain only ASCII letters, digits, underscores, hyphens and colons",
                ));
            }
            if row.list_id.is_some() && row.library_id.is_some() {
                return Err(invalid(
                    "A shelf may reference a list or a library, not both",
                ));
            }
            if let Some(id) = &row.list_id
                && tx
                    .find_by_id::<PersonalList>(id)
                    .await
                    .map_err(db_error)?
                    .is_none_or(|v| v.user_id != row.user_id)
            {
                return Err(denied());
            }
            if let Some(id) = &row.library_id {
                library_owned(tx, id, &row.user_id).await?;
            }
            if event
                .before::<HomeShelf>()?
                .is_some_and(|old| old.user_id != row.user_id || old.shelf_key != row.shelf_key)
            {
                return Err(invalid("Shelf ownership and key are immutable"));
            }
        }
        "LibraryPreference" => {
            let row = event.after::<LibraryPreference>()?.ok_or_else(denied)?;
            position(row.position)?;
            library_owned(tx, &row.library_id, &row.user_id).await?;
            if event
                .before::<LibraryPreference>()?
                .is_some_and(|old| old.user_id != row.user_id || old.library_id != row.library_id)
            {
                return Err(invalid("Preference ownership and library are immutable"));
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) async fn on_mutation(
    _ctx: Option<&Context<'_>>,
    tx: &mut MutationContext<'_>,
    event: &MutationEvent,
) -> Result<()> {
    if event.phase == MutationPhase::Before && event.action == ChangeAction::Deleted {
        cleanup_deleted(tx, event).await?;
    }
    if event.phase != MutationPhase::After {
        return Ok(());
    }
    if event.action != ChangeAction::Deleted && is_personal(event.entity_name) {
        // Input ownership is assigned to every real caller, including admins.
        // Hooks also cover trusted writes and transaction-local bulk mutations.
        validate_row(tx, event).await?;
    }
    // The surviving list must invalidate after a deleted child. Metadata upward
    // propagation handles create/update; this write supplies delete provenance.
    if event.entity_name == "ListEntry"
        && event.action == ChangeAction::Deleted
        && let Some(row) = event.before::<ListEntry>()?
        && tx
            .find_by_id::<PersonalList>(&row.list_id)
            .await
            .map_err(db_error)?
            .is_some()
    {
        tx.update_by_id::<PersonalList>(&row.list_id, UpdatePersonalListInput::default())
            .await
            .map_err(db_error)?;
    }
    Ok(())
}

/// Remove dependants through generated mutations before physical FK actions.
/// This preserves personal deletion events, surviving-parent invalidations and
/// rollback. Deleting a scoped library deletes saved rules rather than silently
/// widening them to every remaining library.
async fn cleanup_deleted(tx: &mut MutationContext<'_>, event: &MutationEvent) -> Result<()> {
    macro_rules! delete {
        ($ty:ty, $filter:ident, $field:ident) => {
            tx.delete_where::<$ty>($filter {
                $field: Some(eq(&event.id)),
                ..Default::default()
            })
            .await
            .map_err(db_error)?;
        };
    }
    match event.entity_name {
        "PersonalList" => {
            delete!(ListEntry, ListEntryWhereInput, list_id);
            tx.update_where::<HomeShelf>(
                HomeShelfWhereInput {
                    list_id: Some(eq(&event.id)),
                    ..Default::default()
                },
                UpdateHomeShelfInput {
                    list_id: Some(None),
                    visible: Some(false),
                    ..Default::default()
                },
            )
            .await
            .map_err(db_error)?;
        }
        "SavedView" => {
            delete!(PersonalList, PersonalListWhereInput, saved_view_id);
        }
        "Library" => {
            delete!(SavedView, SavedViewWhereInput, library_id);
            delete!(LibraryPreference, LibraryPreferenceWhereInput, library_id);
            tx.update_where::<HomeShelf>(
                HomeShelfWhereInput {
                    library_id: Some(eq(&event.id)),
                    ..Default::default()
                },
                UpdateHomeShelfInput {
                    library_id: Some(None),
                    visible: Some(false),
                    ..Default::default()
                },
            )
            .await
            .map_err(db_error)?;
        }
        "Movie" => {
            delete!(ListEntry, ListEntryWhereInput, movie_id);
        }
        "Show" => {
            delete!(ListEntry, ListEntryWhereInput, show_id);
        }
        "Album" => {
            delete!(ListEntry, ListEntryWhereInput, album_id);
        }
        "Audiobook" => {
            delete!(ListEntry, ListEntryWhereInput, audiobook_id);
        }
        "Track" => {
            delete!(ListEntry, ListEntryWhereInput, track_id);
        }
        "User" => {
            delete!(HomeShelf, HomeShelfWhereInput, user_id);
            delete!(LibraryPreference, LibraryPreferenceWhereInput, user_id);
            delete!(PersonalList, PersonalListWhereInput, user_id);
            delete!(SavedView, SavedViewWhereInput, user_id);
            delete!(ListEntry, ListEntryWhereInput, user_id);
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
#[path = "personal_policy_tests.rs"]
mod tests;
