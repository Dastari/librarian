//! GraphQL ORM policy hooks for Librarian.

use async_graphql::ErrorExtensions;
use futures::future::BoxFuture;
use graphql_orm::graphql::filters::StringFilter;
use graphql_orm::graphql::orm::{
    EntityAccessKind, EntityAccessSurface, EntityPolicy, FieldPolicy, WriteInputContext,
};

use crate::db::Database;
use crate::graphql::entities::{
    Album, Artist, Audiobook, Chapter, Episode, Library, LibraryScanRun, LibraryWhereInput,
    MediaFile, Movie, PendingFileMatch, RssFeed, Show, Torrent, TorrentWhereInput, Track,
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

    /// Check the field's scope and input shape without acquiring a connection.
    /// Persisted ownership is checked by the transaction-aware write transform.
    fn mutable_torrent_library(
        user: Option<&AuthUser>,
        entity_name: &str,
        field_name: &str,
        record: Option<&(dyn std::any::Any + Send + Sync)>,
        value: Option<&(dyn std::any::Any + Send + Sync)>,
    ) -> async_graphql::Result<bool> {
        let denied = || AppEntityPolicy::deny("Torrent library link not found or access denied");
        if entity_name != "Torrent" || field_name != "libraryId" {
            return Err(denied());
        }
        let user = user.ok_or_else(denied)?;
        // Administrators retain the existing explicit field override. Approved
        // acquisition/ownership binding remains the responsibility of its
        // action boundary; this key does not silently elevate a member.
        if user.is_admin() {
            return Ok(true);
        }
        if !user.has_role(Role::Member) {
            return Err(denied());
        }
        let record = record
            .and_then(|record| record.downcast_ref::<Torrent>())
            .ok_or_else(denied)?;
        if record.user_id != user.user_id {
            return Err(denied());
        }
        // Generated updates unwrap the outer option. An inner None explicitly
        // unlinks; missing/untyped values must not become unlink grants.
        value
            .and_then(|value| value.downcast_ref::<Option<String>>())
            .ok_or_else(denied)?;
        Ok(true)
    }

    /// The one public mutable foreign link: validate fresh persisted torrent
    /// and destination owners through the originating generated transaction.
    /// WriteQuery's trusted reads require explicit owner checks here.
    pub(crate) async fn validate_mutable_torrent_library(
        write_ctx: &mut WriteInputContext<'_, '_>,
        user: Option<&AuthUser>,
        record: Option<&(dyn std::any::Any + Send + Sync)>,
        value: Option<&(dyn std::any::Any + Send + Sync)>,
    ) -> async_graphql::Result<()> {
        Self::mutable_torrent_library(user, write_ctx.entity_name(), "libraryId", record, value)?;
        let denied = || AppEntityPolicy::deny("Torrent library link not found or access denied");
        let user = user.ok_or_else(denied)?;
        if user.is_admin() {
            return Ok(());
        }
        let record = record
            .and_then(|record| record.downcast_ref::<Torrent>())
            .ok_or_else(denied)?;
        let torrent = write_ctx
            .query::<Torrent>()
            .filter(TorrentWhereInput {
                id: Some(StringFilter {
                    eq: Some(record.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_one()
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?
            .ok_or_else(denied)?;
        if torrent.user_id != user.user_id {
            return Err(denied());
        }
        let library_id = value
            .and_then(|value| value.downcast_ref::<Option<String>>())
            .ok_or_else(denied)?;
        if let Some(library_id) = library_id {
            let library = write_ctx
                .query::<Library>()
                .filter(LibraryWhereInput {
                    id: Some(StringFilter {
                        eq: Some(library_id.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .fetch_one()
                .await
                .map_err(|error| async_graphql::Error::new(error.to_string()))?
                .ok_or_else(denied)?;
            if library.user_id != torrent.user_id {
                return Err(denied());
            }
        }
        Ok(())
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
            "showId" | "tvShowId" => read!(Show::get(db.pool(), &value.to_string()))
                .is_some_and(|row| row.user_id == user_id),
            "artistId" => read!(Artist::get(db.pool(), &value.to_string()))
                .is_some_and(|row| row.user_id == user_id),
            "torrentId" => read!(Torrent::get(db.pool(), &value.to_string()))
                .is_some_and(|row| row.user_id == user_id),
            "feedId" => read!(RssFeed::get(db.pool(), &value.to_string()))
                .is_some_and(|row| row.user_id == user_id),
            "scanRunId" => read!(LibraryScanRun::get(db.pool(), &value.to_string()))
                .is_some_and(|row| row.user_id == user_id),
            "pendingMatchId" => read!(PendingFileMatch::get(db.pool(), &value.to_string()))
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
        entity_name: &'static str,
        field_name: &'static str,
        policy_key: Option<&'static str>,
        record: Option<&'a (dyn std::any::Any + Send + Sync)>,
        value: Option<&'a (dyn std::any::Any + Send + Sync)>,
    ) -> BoxFuture<'a, async_graphql::Result<bool>> {
        Box::pin(async move {
            if super::personal_policy::is_personal(entity_name) {
                return super::personal_policy::write_field(
                    ctx,
                    db,
                    entity_name,
                    field_name,
                    policy_key,
                    record,
                    value,
                )
                .await;
            }
            if policy_key == Some("owned.mutable-link") {
                return Self::mutable_torrent_library(
                    ctx.data_opt::<AuthUser>(),
                    entity_name,
                    field_name,
                    record,
                    value,
                );
            }
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
                Some("admin.write") => Err(AppEntityPolicy::deny(
                    "This field requires administrator authority",
                )),
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

#[cfg(test)]
#[path = "policy_mutable_link_tests.rs"]
mod mutable_link_tests;
