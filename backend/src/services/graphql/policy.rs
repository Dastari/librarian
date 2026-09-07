//! GraphQL ORM policy hooks for Librarian.

use async_graphql::ErrorExtensions;
use futures::future::BoxFuture;
use graphql_orm::graphql::orm::{EntityAccessKind, EntityAccessSurface, EntityPolicy, FieldPolicy};

use crate::db::Database;
use crate::graphql::entities::{
    Album, Audiobook, Chapter, Episode, Library, MediaFile, Movie, Show, Track,
};
use crate::services::graphql::auth::{AuthUser, Role};

#[derive(Debug, Default)]
pub struct AppEntityPolicy;

impl AppEntityPolicy {
    fn deny(message: impl Into<String>) -> async_graphql::Error {
        async_graphql::Error::new(message.into()).extend_with(|_, e| e.set("code", "FORBIDDEN"))
    }

    fn role_allowed(user: Option<&AuthUser>, required: Role) -> async_graphql::Result<bool> {
        let Some(user) = user else {
            return Ok(false);
        };
        Ok(user.has_role(required))
    }
}

impl EntityPolicy for AppEntityPolicy {
    fn can_access_entity<'a>(
        &'a self,
        ctx: Option<&'a async_graphql::Context<'_>>,
        _db: &'a Database,
        entity_name: &'static str,
        policy_key: Option<&'static str>,
        kind: EntityAccessKind,
        surface: EntityAccessSurface,
    ) -> BoxFuture<'a, async_graphql::Result<bool>> {
        Box::pin(async move {
            if ctx.is_none() || surface == EntityAccessSurface::Repository {
                return Ok(true);
            }

            let user = ctx.and_then(|ctx| ctx.data_opt::<AuthUser>());
            let required = match policy_key {
                Some("admin.read" | "admin.write") => Role::Admin,
                Some("member.read" | "member.write") | None => Role::Member,
                Some(other) => {
                    return Err(Self::deny(format!(
                        "Unknown policy '{other}' for {entity_name} {kind:?} on {surface:?}"
                    )));
                }
            };

            Self::role_allowed(user, required)
        })
    }
}

#[derive(Debug, Default)]
pub struct AppFieldPolicy;

impl AppFieldPolicy {
    fn string_value(value: Option<&(dyn std::any::Any + Send + Sync)>) -> Option<&str> {
        let value = value?;
        value
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| {
                value
                    .downcast_ref::<Option<String>>()
                    .and_then(Option::as_deref)
            })
    }

    pub(crate) async fn owns_link(
        db: &Database,
        user_id: &str,
        field_name: &str,
        value: &str,
    ) -> async_graphql::Result<bool> {
        macro_rules! read {
            ($future:expr) => {
                $future
                    .await
                    .map_err(|error| async_graphql::Error::new(error.to_string()))?
            };
        }

        let owned = match field_name {
            "libraryId" => read!(Library::get(db.pool(), &value.to_string()))
                .is_some_and(|row| row.user_id == user_id),
            "movieId" => read!(Movie::get(db.pool(), &value.to_string()))
                .is_some_and(|row| row.user_id == user_id),
            "showId" => read!(Show::get(db.pool(), &value.to_string()))
                .is_some_and(|row| row.user_id == user_id),
            "albumId" => read!(Album::get(db.pool(), &value.to_string()))
                .is_some_and(|row| row.user_id == user_id),
            "audiobookId" => read!(Audiobook::get(db.pool(), &value.to_string()))
                .is_some_and(|row| row.user_id == user_id),
            "mediaFileId" => {
                let media = read!(MediaFile::get(db.pool(), &value.to_string()));
                match media.and_then(|row| row.library_id) {
                    Some(library_id) => read!(Library::get(db.pool(), &library_id))
                        .is_some_and(|row| row.user_id == user_id),
                    None => false,
                }
            }
            "episodeId" => {
                let episode = read!(Episode::get(db.pool(), &value.to_string()));
                match episode {
                    Some(row) => read!(Show::get(db.pool(), &row.show_id))
                        .is_some_and(|show| show.user_id == user_id),
                    None => false,
                }
            }
            "trackId" => {
                let track = read!(Track::get(db.pool(), &value.to_string()));
                match track {
                    Some(row) => read!(Library::get(db.pool(), &row.library_id))
                        .is_some_and(|library| library.user_id == user_id),
                    None => false,
                }
            }
            "chapterId" => {
                let chapter = read!(Chapter::get(db.pool(), &value.to_string()));
                match chapter {
                    Some(row) => read!(Audiobook::get(db.pool(), &row.audiobook_id))
                        .is_some_and(|book| book.user_id == user_id),
                    None => false,
                }
            }
            _ => false,
        };
        Ok(owned)
    }
}

impl FieldPolicy for AppFieldPolicy {
    fn can_read_field<'a>(
        &'a self,
        ctx: &'a async_graphql::Context<'_>,
        _db: &'a Database,
        _entity_name: &'static str,
        _field_name: &'static str,
        policy_key: Option<&'static str>,
        _record: Option<&'a (dyn std::any::Any + Send + Sync)>,
    ) -> BoxFuture<'a, async_graphql::Result<bool>> {
        Box::pin(async move {
            let user = ctx.data_opt::<AuthUser>();
            Ok(match policy_key {
                None | Some("member.read") => user.is_some_and(|user| user.has_role(Role::Member)),
                Some("admin.read") => user.is_some_and(AuthUser::is_admin),
                Some(_) => false,
            })
        })
    }

    fn can_write_field<'a>(
        &'a self,
        ctx: &'a async_graphql::Context<'_>,
        db: &'a Database,
        _entity_name: &'static str,
        field_name: &'static str,
        policy_key: Option<&'static str>,
        record: Option<&'a (dyn std::any::Any + Send + Sync)>,
        value: Option<&'a (dyn std::any::Any + Send + Sync)>,
    ) -> BoxFuture<'a, async_graphql::Result<bool>> {
        Box::pin(async move {
            let Some(user) = ctx.data_opt::<AuthUser>() else {
                return Ok(false);
            };
            if user.is_admin() {
                return Ok(true);
            }
            if !user.has_role(Role::Member) {
                return Ok(false);
            }
            match policy_key {
                None | Some("member.write") => Ok(true),
                Some("admin.write") => Ok(false),
                // Ownership can be supplied when a row is created, but it is
                // immutable through public generated updates.
                Some("owner.id") => Ok(
                    record.is_none() && Self::string_value(value) == Some(user.user_id.as_str())
                ),
                // Link fields must point to content the caller owns and are
                // immutable after creation. Service/repository operations use
                // the trusted repository branch below.
                Some("owned.link") => {
                    let Some(value) = Self::string_value(value) else {
                        return Ok(record.is_none());
                    };
                    Ok(record.is_none()
                        && Self::owns_link(db, &user.user_id, field_name, value).await?)
                }
                Some(_) => Ok(false),
            }
        })
    }

    fn can_read_repository_field<'a>(
        &'a self,
        _access: Option<graphql_orm::graphql::auth::AccessContext<'a>>,
        _db: &'a Database,
        _entity_name: &'static str,
        _field_name: &'static str,
        _policy_key: Option<&'static str>,
        _record: Option<&'a (dyn std::any::Any + Send + Sync)>,
    ) -> BoxFuture<'a, async_graphql::Result<bool>> {
        Box::pin(async { Ok(true) })
    }

    fn can_write_repository_field<'a>(
        &'a self,
        _access: Option<graphql_orm::graphql::auth::AccessContext<'a>>,
        _db: &'a Database,
        _entity_name: &'static str,
        _field_name: &'static str,
        _policy_key: Option<&'static str>,
        _record: Option<&'a (dyn std::any::Any + Send + Sync)>,
        _value: Option<&'a (dyn std::any::Any + Send + Sync)>,
    ) -> BoxFuture<'a, async_graphql::Result<bool>> {
        Box::pin(async { Ok(true) })
    }
}
