//! Row-level ownership policy for GraphQL entity access.
//!
//! Repository-surface calls are trusted internal service operations. GraphQL
//! member access is scoped either by a row's direct `user_id`, its library, or
//! its nearest owned parent. Administrators retain an explicit override.
//!
//! Generated reads receive complete typed owner predicates before pagination.
//! Subscription dispatch retains callbacks for deleted-preimage authorization;
//! complete SQL visibility deliberately suppresses deleted preimages upstream.

use futures::future::BoxFuture;
use graphql_orm::DefaultBackend;
use graphql_orm::graphql::filters::StringFilter;
use graphql_orm::graphql::orm::{EntityAccessSurface, ReadPredicate, ReadVisibility, RowPolicy};

use crate::db::Database;
use crate::graphql::entities::*;
use crate::services::graphql::auth::{AuthUser, Role};

#[derive(Debug, Default)]
pub struct OwnershipRowPolicy;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ScopeClass {
    Owner,
    Derived,
    Shared,
    Admin,
}

impl ScopeClass {
    #[cfg(test)]
    fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Derived => "derived",
            Self::Shared => "shared",
            Self::Admin => "admin",
        }
    }
}

#[cfg(test)]
fn ownership_allowed(role: Option<Role>, row_user_id: &str, requester_user_id: &str) -> bool {
    match role {
        Some(Role::Admin) => true,
        Some(Role::Member) => row_user_id == requester_user_id,
        None => false,
    }
}

impl OwnershipRowPolicy {
    fn member_read_predicate(
        entity_name: &str,
        owner: &str,
    ) -> async_graphql::Result<Option<ReadPredicate>> {
        let owner_filter = || StringFilter {
            eq: Some(owner.to_owned()),
            ..Default::default()
        };
        macro_rules! direct {
            ($where_type:ident) => {
                ReadPredicate::from_filter::<DefaultBackend, _>(&$where_type {
                    user_id: Some(owner_filter()),
                    ..Default::default()
                })?
            };
        }
        let library = || {
            ReadPredicate::from_filter::<DefaultBackend, _>(&LibraryWhereInput {
                user_id: Some(owner_filter()),
                ..Default::default()
            })
        };
        let file = || {
            ReadPredicate::related::<DefaultBackend, MediaFile, Library>(
                &[("library_id", "id")],
                library()?,
            )
        };
        let predicate = match entity_name {
            "Library" => direct!(LibraryWhereInput),
            "LibraryScanRun" => direct!(LibraryScanRunWhereInput),
            "LibraryScanIssue" => direct!(LibraryScanIssueWhereInput),
            "Movie" => direct!(MovieWhereInput),
            "Collection" => direct!(CollectionWhereInput),
            "Show" => direct!(ShowWhereInput),
            "Artist" => direct!(ArtistWhereInput),
            "Album" => direct!(AlbumWhereInput),
            "Audiobook" => direct!(AudiobookWhereInput),
            "PendingFileMatch" => direct!(PendingFileMatchWhereInput),
            "PlaybackProgress" => direct!(PlaybackProgressWhereInput),
            "PlaybackSession" => direct!(PlaybackSessionWhereInput),
            "Notification" => direct!(NotificationWhereInput),
            "RssFeed" => direct!(RssFeedWhereInput),
            "Torrent" => direct!(TorrentWhereInput),
            "UsenetDownload" => direct!(UsenetDownloadWhereInput),
            "NamingPattern" => direct!(NamingPatternWhereInput),
            "SourcePriorityRule" => direct!(SourcePriorityRuleWhereInput),
            "CastSession" => direct!(CastSessionWhereInput),
            "MediaFile" => file()?,
            "Track" => ReadPredicate::related::<DefaultBackend, Track, Library>(
                &[("library_id", "id")],
                library()?,
            )?,
            "Episode" => ReadPredicate::related::<DefaultBackend, Episode, Show>(
                &[("show_id", "id")],
                direct!(ShowWhereInput),
            )?,
            "Season" => ReadPredicate::related::<DefaultBackend, Season, Show>(
                &[("show_id", "id")],
                direct!(ShowWhereInput),
            )?,
            "Chapter" => ReadPredicate::related::<DefaultBackend, Chapter, Audiobook>(
                &[("audiobook_id", "id")],
                direct!(AudiobookWhereInput),
            )?,
            "TorrentFile" => ReadPredicate::related::<DefaultBackend, TorrentFile, Torrent>(
                &[("torrent_id", "id")],
                direct!(TorrentWhereInput),
            )?,
            "MovieCastCredit" => ReadPredicate::related::<DefaultBackend, MovieCastCredit, Movie>(
                &[("movie_id", "id")],
                direct!(MovieWhereInput),
            )?,
            "RssFeedItem" => ReadPredicate::related::<DefaultBackend, RssFeedItem, RssFeed>(
                &[("feed_id", "id")],
                direct!(RssFeedWhereInput),
            )?,
            "VideoStream" => ReadPredicate::related::<DefaultBackend, VideoStream, MediaFile>(
                &[("media_file_id", "id")],
                file()?,
            )?,
            "AudioStream" => ReadPredicate::related::<DefaultBackend, AudioStream, MediaFile>(
                &[("media_file_id", "id")],
                file()?,
            )?,
            "Subtitle" => ReadPredicate::related::<DefaultBackend, Subtitle, MediaFile>(
                &[("media_file_id", "id")],
                file()?,
            )?,
            "MediaChapter" => ReadPredicate::related::<DefaultBackend, MediaChapter, MediaFile>(
                &[("media_file_id", "id")],
                file()?,
            )?,
            _ => return Ok(None),
        };
        Ok(Some(predicate))
    }

    fn scope_class(entity_name: &str) -> Option<ScopeClass> {
        if super::personal_policy::is_personal(entity_name) {
            return Some(ScopeClass::Owner);
        }
        Some(match entity_name {
            "Library" | "LibraryScanRun" | "LibraryScanIssue" | "Movie" | "Collection" | "Show"
            | "Artist" | "Album" | "Audiobook" | "PendingFileMatch" | "PlaybackProgress"
            | "PlaybackSession" | "Notification" | "RssFeed" | "Torrent" | "UsenetDownload"
            | "NamingPattern" | "SourcePriorityRule" | "CastSession" => ScopeClass::Owner,
            "MovieCastCredit" | "Episode" | "Season" | "MediaFile" | "Track" | "Chapter"
            | "TorrentFile" | "RssFeedItem" | "VideoStream" | "AudioStream" | "Subtitle"
            | "MediaChapter" => ScopeClass::Derived,
            "Person" | "CastDevice" | "CastSetting" | "ScheduleCache" | "TorznabCategory"
            | "QualityProfile" | "ReleaseBlocklist" => ScopeClass::Shared,
            "Source" | "User" | "InviteToken" | "RefreshToken" | "AppSetting" | "AppLog"
            | "UsenetServer" | "ScheduleSyncState" | "MetadataCache" => ScopeClass::Admin,
            _ => return None,
        })
    }

    fn direct_user_id<'a>(
        entity_name: &'static str,
        row: &'a (dyn std::any::Any + Send + Sync),
    ) -> Option<Option<&'a str>> {
        macro_rules! required {
            ($ty:ty) => {
                row.downcast_ref::<$ty>()
                    .map(|record| Some(record.user_id.as_str()))
            };
        }
        match entity_name {
            "Library" => required!(Library),
            "LibraryScanRun" => required!(LibraryScanRun),
            "LibraryScanIssue" => required!(LibraryScanIssue),
            "Movie" => required!(Movie),
            "Show" => required!(Show),
            "Collection" => required!(Collection),
            "Artist" => required!(Artist),
            "Album" => required!(Album),
            "Audiobook" => required!(Audiobook),
            "PendingFileMatch" => required!(PendingFileMatch),
            "PlaybackProgress" => required!(PlaybackProgress),
            "PlaybackSession" => required!(PlaybackSession),
            "Notification" => required!(Notification),
            "RssFeed" => required!(RssFeed),
            "Torrent" => required!(Torrent),
            "UsenetDownload" => required!(UsenetDownload),
            "NamingPattern" => required!(NamingPattern),
            "SourcePriorityRule" => required!(SourcePriorityRule),
            "CastSession" => row
                .downcast_ref::<CastSession>()
                .map(|record| record.user_id.as_deref()),
            _ => None,
        }
    }

    async fn library_owned(
        db: &Database,
        library_id: Option<&str>,
        requester_user_id: &str,
    ) -> async_graphql::Result<bool> {
        let Some(library_id) = library_id else {
            return Ok(false);
        };
        Ok(Library::get(db.pool(), &library_id.to_string())
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?
            .is_some_and(|library| library.user_id == requester_user_id))
    }

    async fn media_file_owned(
        db: &Database,
        media_file_id: &str,
        requester_user_id: &str,
    ) -> async_graphql::Result<bool> {
        let media_file = MediaFile::get(db.pool(), &media_file_id.to_string())
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        Self::library_owned(
            db,
            media_file
                .as_ref()
                .and_then(|file| file.library_id.as_deref()),
            requester_user_id,
        )
        .await
    }

    async fn derived_owned(
        db: &Database,
        entity_name: &'static str,
        row: &(dyn std::any::Any + Send + Sync),
        requester_user_id: &str,
    ) -> async_graphql::Result<Option<bool>> {
        let owned = match entity_name {
            "MediaFile" => {
                let Some(record) = row.downcast_ref::<MediaFile>() else {
                    return Ok(Some(false));
                };
                Self::library_owned(db, record.library_id.as_deref(), requester_user_id).await?
            }
            "Episode" => {
                let Some(record) = row.downcast_ref::<Episode>() else {
                    return Ok(Some(false));
                };
                let parent = Show::get(db.pool(), &record.show_id)
                    .await
                    .map_err(|error| async_graphql::Error::new(error.to_string()))?;
                parent.is_some_and(|show| show.user_id == requester_user_id)
            }
            "Season" => {
                let Some(record) = row.downcast_ref::<Season>() else {
                    return Ok(Some(false));
                };
                let parent = Show::get(db.pool(), &record.show_id)
                    .await
                    .map_err(|error| async_graphql::Error::new(error.to_string()))?;
                parent.is_some_and(|show| show.user_id == requester_user_id)
            }
            "Track" => {
                let Some(record) = row.downcast_ref::<Track>() else {
                    return Ok(Some(false));
                };
                Self::library_owned(db, Some(&record.library_id), requester_user_id).await?
            }
            "Chapter" => {
                let Some(record) = row.downcast_ref::<Chapter>() else {
                    return Ok(Some(false));
                };
                let parent = Audiobook::get(db.pool(), &record.audiobook_id)
                    .await
                    .map_err(|error| async_graphql::Error::new(error.to_string()))?;
                parent.is_some_and(|book| book.user_id == requester_user_id)
            }
            "VideoStream" => {
                let Some(record) = row.downcast_ref::<VideoStream>() else {
                    return Ok(Some(false));
                };
                Self::media_file_owned(db, &record.media_file_id, requester_user_id).await?
            }
            "AudioStream" => {
                let Some(record) = row.downcast_ref::<AudioStream>() else {
                    return Ok(Some(false));
                };
                Self::media_file_owned(db, &record.media_file_id, requester_user_id).await?
            }
            "Subtitle" => {
                let Some(record) = row.downcast_ref::<Subtitle>() else {
                    return Ok(Some(false));
                };
                Self::media_file_owned(db, &record.media_file_id, requester_user_id).await?
            }
            "MediaChapter" => {
                let Some(record) = row.downcast_ref::<MediaChapter>() else {
                    return Ok(Some(false));
                };
                Self::media_file_owned(db, &record.media_file_id, requester_user_id).await?
            }
            "TorrentFile" => {
                let Some(record) = row.downcast_ref::<TorrentFile>() else {
                    return Ok(Some(false));
                };
                let parent = Torrent::get(db.pool(), &record.torrent_id)
                    .await
                    .map_err(|error| async_graphql::Error::new(error.to_string()))?;
                parent.is_some_and(|torrent| torrent.user_id == requester_user_id)
            }
            "MovieCastCredit" => {
                let Some(record) = row.downcast_ref::<MovieCastCredit>() else {
                    return Ok(Some(false));
                };
                let parent = Movie::get(db.pool(), &record.movie_id)
                    .await
                    .map_err(|error| async_graphql::Error::new(error.to_string()))?;
                parent.is_some_and(|movie| movie.user_id == requester_user_id)
            }
            "RssFeedItem" => {
                let Some(record) = row.downcast_ref::<RssFeedItem>() else {
                    return Ok(Some(false));
                };
                let parent = RssFeed::get(db.pool(), &record.feed_id)
                    .await
                    .map_err(|error| async_graphql::Error::new(error.to_string()))?;
                parent.is_some_and(|feed| feed.user_id == requester_user_id)
            }
            _ => return Ok(None),
        };
        Ok(Some(owned))
    }

    async fn decide(
        ctx: Option<&async_graphql::Context<'_>>,
        db: &Database,
        surface: EntityAccessSurface,
        entity_name: &'static str,
        row: &(dyn std::any::Any + Send + Sync),
    ) -> async_graphql::Result<bool> {
        if ctx.is_none() || surface == EntityAccessSurface::Repository {
            return Ok(true);
        }
        let Some(user) = ctx.and_then(|ctx| ctx.data_opt::<AuthUser>()) else {
            return Ok(false);
        };
        if super::personal_policy::is_personal(entity_name) {
            return Ok(user.has_role(Role::Member)
                && super::personal_policy::owner(entity_name, row) == Some(user.user_id.as_str()));
        }
        if user.is_admin() {
            return Ok(true);
        }
        if user.parsed_role() != Some(Role::Member) {
            return Ok(false);
        }

        if let Some(row_user_id) = Self::direct_user_id(entity_name, row) {
            return Ok(row_user_id == Some(user.user_id.as_str()));
        }
        if let Some(owned) = Self::derived_owned(db, entity_name, row, &user.user_id).await? {
            return Ok(owned);
        }
        // These rows are intentionally shared configuration/catalog data.
        // Everything else must be explicitly direct-owned, derived-owned, or
        // admin-only at the entity policy. Default-deny prevents a newly
        // registered entity from silently becoming globally visible.
        Ok(Self::scope_class(entity_name) == Some(ScopeClass::Shared))
    }
}

impl RowPolicy for OwnershipRowPolicy {
    fn read_visibility<'a>(
        &'a self,
        ctx: Option<&'a async_graphql::Context<'_>>,
        _db: &'a Database,
        entity_name: &'static str,
        _policy_key: Option<&'static str>,
        surface: EntityAccessSurface,
    ) -> BoxFuture<'a, async_graphql::Result<ReadVisibility>> {
        Box::pin(async move {
            if ctx.is_none() || surface == EntityAccessSurface::Repository {
                return Ok(ReadVisibility::Unrestricted);
            }
            // Preserve explicit delete-preimage checks on event delivery. Query
            // predicate completeness must not silently disable DELETE notices.
            if surface == EntityAccessSurface::GraphqlSubscription {
                return Ok(ReadVisibility::CallbackOnly);
            }
            let user = ctx
                .and_then(|ctx| ctx.data_opt::<AuthUser>())
                .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;
            if super::personal_policy::is_personal(entity_name) {
                if !user.has_role(Role::Member) {
                    return Err(async_graphql::Error::new("Access denied"));
                }
                return super::personal_policy::read_predicate(entity_name, &user.user_id)?
                    .map(ReadVisibility::Complete)
                    .ok_or_else(|| {
                        async_graphql::Error::new("Missing personal ownership predicate")
                    });
            }
            if user.is_admin() {
                return Ok(ReadVisibility::Unrestricted);
            }
            if user.parsed_role() != Some(Role::Member) {
                return Err(async_graphql::Error::new("Access denied"));
            }
            if Self::scope_class(entity_name) == Some(ScopeClass::Shared) {
                return Ok(ReadVisibility::Unrestricted);
            }
            match Self::member_read_predicate(entity_name, &user.user_id)? {
                Some(predicate) => Ok(ReadVisibility::Complete(predicate)),
                None => Ok(ReadVisibility::CallbackOnly),
            }
        })
    }

    fn can_read_deleted_preimage<'a>(
        &'a self,
        ctx: Option<&'a async_graphql::Context<'_>>,
        db: &'a Database,
        entity_name: &'static str,
        _policy_key: Option<&'static str>,
        surface: EntityAccessSurface,
        row: &'a (dyn std::any::Any + Send + Sync),
    ) -> BoxFuture<'a, async_graphql::Result<bool>> {
        Box::pin(async move {
            if ctx.is_none() || surface != EntityAccessSurface::GraphqlSubscription {
                return Ok(false);
            }
            // SQLite has no separate database-session/RLS policy here. Recheck
            // this application's explicit ownership policy on the trusted
            // generated preimage so deletions can invalidate clients' queries.
            // Derived ownership still requires a currently visible parent.
            Self::decide(ctx, db, surface, entity_name, row).await
        })
    }

    fn can_read_row<'a>(
        &'a self,
        ctx: Option<&'a async_graphql::Context<'_>>,
        db: &'a Database,
        entity_name: &'static str,
        _policy_key: Option<&'static str>,
        surface: EntityAccessSurface,
        row: &'a (dyn std::any::Any + Send + Sync),
    ) -> BoxFuture<'a, async_graphql::Result<bool>> {
        Box::pin(Self::decide(ctx, db, surface, entity_name, row))
    }

    fn can_write_row<'a>(
        &'a self,
        ctx: Option<&'a async_graphql::Context<'_>>,
        db: &'a Database,
        entity_name: &'static str,
        _policy_key: Option<&'static str>,
        surface: EntityAccessSurface,
        row: &'a (dyn std::any::Any + Send + Sync),
    ) -> BoxFuture<'a, async_graphql::Result<bool>> {
        Box::pin(Self::decide(ctx, db, surface, entity_name, row))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    use super::{OwnershipRowPolicy, ownership_allowed};
    use crate::services::graphql::auth::Role;

    #[test]
    fn admin_can_access_any_row() {
        assert!(ownership_allowed(Some(Role::Admin), "other-user", "me"));
        assert!(ownership_allowed(Some(Role::Admin), "me", "me"));
    }

    #[test]
    fn member_can_access_only_own_row() {
        assert!(ownership_allowed(Some(Role::Member), "me", "me"));
        assert!(!ownership_allowed(Some(Role::Member), "other-user", "me"));
    }

    #[test]
    fn unauthenticated_is_denied() {
        assert!(!ownership_allowed(None, "me", "me"));
        assert!(!ownership_allowed(None, "other-user", "me"));
    }

    #[derive(Deserialize)]
    struct MatrixEntry {
        entity: String,
        scope: String,
    }

    #[test]
    fn every_authorization_matrix_entity_has_an_explicit_row_scope() {
        let entries: Vec<MatrixEntry> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/authorization-matrix.json"
        ))
        .expect("authorization matrix should parse");
        let expected: BTreeMap<_, _> = entries
            .into_iter()
            .map(|entry| (entry.entity, entry.scope))
            .collect();
        let actual: BTreeMap<_, _> = expected
            .keys()
            .map(|entity| {
                (
                    entity.clone(),
                    OwnershipRowPolicy::scope_class(entity)
                        .map(|scope| scope.as_str().to_string())
                        .unwrap_or_else(|| "UNDECLARED".to_string()),
                )
            })
            .collect();
        assert_eq!(actual, expected);
    }
}
