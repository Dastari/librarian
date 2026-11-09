//! Database service: wraps the SQLite pool for lifecycle (start/stop/health) and dependencies.
//!
//! Other services that need the database (e.g. logging) should declare `dependencies: ["database"]`.

use std::collections::BTreeSet;
use std::sync::{Arc, Weak};
use std::time::Duration;

use anyhow::{Context, Result};
use async_graphql::ErrorExtensions;
use async_trait::async_trait;
use futures::future::BoxFuture;
use graphql_orm::graphql::filters::StringFilter;
use graphql_orm::graphql::orm::{
    ApplyOptions, AuthorizedScanConfig, ChangeAction, DatabaseFilter, DefaultBackend, Entity,
    FromSqlRow, MigrationRisk, MigrationStep, MutationContext, MutationEvent, MutationHook,
    MutationPhase, PaginationConfig, PlannedMigration, PostCommitErrorHandler, TransactionMode,
    WriteInputTransform,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tracing::{debug, error, info, warn};

use crate::db::{Database, connect_with_retry};
use crate::graphql::AuthUser;
use crate::services::ServicesManager;
use crate::services::bootstrap_defaults::seed_defaults;
use crate::services::graphql::entities::*;
use crate::services::graphql::policy::{AppEntityPolicy, AppFieldPolicy};
use crate::services::graphql::row_policy::OwnershipRowPolicy;
use crate::services::manager::{Service, ServiceHealth};

/// Configuration for the database service (connection URL, timeouts, etc.).
#[derive(Debug, Clone)]
pub struct DatabaseServiceConfig {
    /// SQLite connection URL (e.g. `sqlite:///data/librarian.db` or `sqlite::memory:`).
    pub database_url: String,
    /// How long to retry connecting before giving up.
    pub connect_timeout: Duration,
}

impl Default for DatabaseServiceConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite:librarian.db".to_string(),
            connect_timeout: Duration::from_secs(30),
        }
    }
}

/// Slots in each per-entity `Changed` event channel (the ORM default is 256).
///
/// A subscriber that falls more than this many events behind gets
/// `SUBSCRIPTION_CONTINUITY_LOST` and must refetch, and propagated events share the parent's
/// channel, so one burst makes every subscriber of that entity lag together.
///
/// Sizing, from the live library (16,434 media files, 89 episodes in 6 shows, at most 31
/// episodes in a show, 192 files in a torrent, 39 subtitles on a file): a single commit that
/// deletes or rematches one title writes a few hundred rows and, with the parent writes and
/// propagated events they trigger, about a thousand events on the busiest channel. 16,384
/// slots is more than ten times that, so a title delete or a rematched page never lags a
/// subscriber that is merely busy for a moment. A whole-library delete emits well over 100,000 events on the
/// library and media-file channels; no sensible capacity holds that, and clients already
/// resynchronize on `SUBSCRIPTION_CONTINUITY_LOST`.
///
/// Memory is one pre-allocated slot per event, a 64-byte event plus ring
/// bookkeeping (see `event_channel_slot_size_bounds_memory`), so about 1.5 MB per entity channel that exists,
/// plus the shared payload of the events still held in the ring. Channels are created on first
/// use, so only entity types actually written or watched pay: roughly 40 MB if the web app
/// watches all of the ~25 entity types it does today.
pub(crate) const EVENT_CHANNEL_CAPACITY: usize = 16_384;

/// Set the event channel capacity. Capacity applies to channels created afterwards, so call
/// this before the first subscription or write.
pub(crate) fn apply_event_channel_capacity(pool: &Database) {
    pool.set_event_channel_capacity(EVENT_CHANNEL_CAPACITY);
}

/// Service that owns the database pool and provides start/stop/health.
/// Register this first so that services depending on `"database"` can start after it.
pub struct DatabaseService {
    pool: Database,
}

impl DatabaseService {
    /// Create a new database service with an already-connected pool.
    /// Use [from_config](Self::from_config) to create from URL and timeout.
    pub fn new(mut pool: Database) -> Self {
        Self::install_hooks(&mut pool, None);
        Self { pool }
    }

    /// Create and connect the database service from config. Call this when building
    /// the service manager (e.g. in [ServicesManagerBuilder](crate::services::manager::ServicesManagerBuilder)).
    pub async fn from_config(
        config: DatabaseServiceConfig,
        services: Option<Weak<ServicesManager>>,
    ) -> Result<Self> {
        let mut pool = connect_with_retry(&config.database_url, config.connect_timeout)
            .await
            .context("Database service: connect_with_retry failed")?;
        Self::install_hooks(&mut pool, services);
        // Before any subscription or write creates an event channel. Unit tests build
        // hundreds of in-memory databases through this constructor; they keep the ORM default
        // so the test process does not pre-allocate every channel for each of them.
        #[cfg(not(test))]
        apply_event_channel_capacity(&pool);
        Ok(Self { pool })
    }

    /// Access the pool (e.g. to clone for app state). Valid until [Service::stop] is called.
    pub fn pool(&self) -> &Database {
        &self.pool
    }

    fn install_hooks(pool: &mut Database, services: Option<Weak<ServicesManager>>) {
        // Exact visible offsets/counts with residual ownership checks require an
        // explicit finite budget since ORM 0.46. Scan continuations are local to
        // this process; restarting invalidates them rather than reusing a key.
        *pool = pool
            .clone()
            .with_authorized_scan_config(AuthorizedScanConfig::new(rand::random(), 256, 100_000));
        // Preserve the pre-0.3 ORM page behavior while existing Librarian operations are
        // migrated to bounded pagination. The consolidated ORM defaults to 50/100, while
        // this application previously used and still requests pages up to the legacy 1,000 cap.
        pool.set_pagination_config(PaginationConfig::legacy());
        pool.set_entity_policy(AppEntityPolicy);
        pool.set_field_policy(AppFieldPolicy);
        pool.set_row_policy(OwnershipRowPolicy);
        pool.set_write_input_transform(AppWriteInputTransform {
            services: services.clone(),
        });
        pool.set_mutation_hook(AppMutationHook { services });
        pool.set_post_commit_error_handler(AppPostCommitErrorHandler);
    }
}

/// All entities that make up the live schema (used for schema bootstrap/migration and, via
/// [crate::services::backup], for full logical database backup/restore). This is the single
/// source of truth for "every real table the app owns" — do not hand-copy a second list.
pub(crate) fn entity_metadata() -> Vec<&'static graphql_orm::graphql::orm::EntityMetadata> {
    vec![
        <Library as Entity>::metadata(),
        <LibraryScanRun as Entity>::metadata(),
        <LibraryScanIssue as Entity>::metadata(),
        <Movie as Entity>::metadata(),
        <Person as Entity>::metadata(),
        <MovieCastCredit as Entity>::metadata(),
        <Collection as Entity>::metadata(),
        <Show as Entity>::metadata(),
        <Episode as Entity>::metadata(),
        <Season as Entity>::metadata(),
        <MediaFile as Entity>::metadata(),
        <Artist as Entity>::metadata(),
        <Album as Entity>::metadata(),
        <Track as Entity>::metadata(),
        <Audiobook as Entity>::metadata(),
        <Chapter as Entity>::metadata(),
        <Torrent as Entity>::metadata(),
        <TorrentFile as Entity>::metadata(),
        <RssFeed as Entity>::metadata(),
        <RssFeedItem as Entity>::metadata(),
        <PendingFileMatch as Entity>::metadata(),
        <Source as Entity>::metadata(),
        <User as Entity>::metadata(),
        <InviteToken as Entity>::metadata(),
        <RefreshToken as Entity>::metadata(),
        <AppSetting as Entity>::metadata(),
        <AppLog as Entity>::metadata(),
        <VideoStream as Entity>::metadata(),
        <AudioStream as Entity>::metadata(),
        <Subtitle as Entity>::metadata(),
        <MediaChapter as Entity>::metadata(),
        <PlaybackSession as Entity>::metadata(),
        <PlaybackProgress as Entity>::metadata(),
        <PersonalList as Entity>::metadata(),
        <ListEntry as Entity>::metadata(),
        <SavedView as Entity>::metadata(),
        <HomeShelf as Entity>::metadata(),
        <LibraryPreference as Entity>::metadata(),
        <CastDevice as Entity>::metadata(),
        <CastSession as Entity>::metadata(),
        <CastSetting as Entity>::metadata(),
        <UsenetServer as Entity>::metadata(),
        <MatchingAssistant as Entity>::metadata(),
        <IdentificationEvent as Entity>::metadata(),
        <IdentifiedItem as Entity>::metadata(),
        <UsenetDownload as Entity>::metadata(),
        <ScheduleCache as Entity>::metadata(),
        <ScheduleSyncState as Entity>::metadata(),
        <NamingPattern as Entity>::metadata(),
        <MetadataCache as Entity>::metadata(),
        <SourcePriorityRule as Entity>::metadata(),
        <Notification as Entity>::metadata(),
        <StorageObject as Entity>::metadata(),
        <ArtworkCache as Entity>::metadata(),
        <TorznabCategory as Entity>::metadata(),
        <QualityProfile as Entity>::metadata(),
        <ReleaseBlocklist as Entity>::metadata(),
    ]
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum SchemaChangeClass {
    Additive,
    DataTransforming,
    Destructive,
}

impl SchemaChangeClass {
    fn label(self) -> &'static str {
        match self {
            Self::Additive => "additive",
            Self::DataTransforming => "data-transforming",
            Self::Destructive => "destructive",
        }
    }
}

/// Proof that a full backup snapshot was checksum-verified and was produced from the exact
/// source schema in a migration plan. The fields are private so callers cannot bypass backup
/// verification by constructing a value themselves.
#[derive(Clone, Debug)]
pub(crate) struct VerifiedSchemaBackup {
    snapshot_id: String,
    source_schema_hash: String,
}

impl VerifiedSchemaBackup {
    pub(crate) fn new(snapshot_id: String, source_schema_hash: String) -> Self {
        Self {
            snapshot_id,
            source_schema_hash,
        }
    }

    pub(crate) fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }
}

/// Librarian deliberately uses a stricter classification than the ORM's portable baseline.
/// In particular, any SQL type conversion is destructive until migration-specific transform
/// code proves how values are preserved. A field rename naturally plans as add+drop, so the
/// drop half also makes the overall plan destructive.
fn effective_step_class(
    step: &graphql_orm::graphql::orm::PlannedMigrationStep,
) -> SchemaChangeClass {
    if step.risk == MigrationRisk::Destructive {
        return SchemaChangeClass::Destructive;
    }

    if matches!(
        &step.step,
        MigrationStep::AlterColumn { before, after, .. } if before.sql_type != after.sql_type
    ) {
        return SchemaChangeClass::Destructive;
    }

    match step.risk {
        MigrationRisk::Additive => SchemaChangeClass::Additive,
        MigrationRisk::Compatible | MigrationRisk::Risky => SchemaChangeClass::DataTransforming,
        MigrationRisk::Destructive => SchemaChangeClass::Destructive,
    }
}

pub(crate) fn schema_plan_class(plan: &PlannedMigration) -> SchemaChangeClass {
    plan.steps
        .iter()
        .map(effective_step_class)
        .max()
        .unwrap_or(SchemaChangeClass::Additive)
}

fn sanitized_step_description(step: &MigrationStep) -> String {
    match step {
        MigrationStep::EnableExtension { name } => format!("enable extension {name}"),
        MigrationStep::CreateTable(table) => format!("create table {}", table.table_name),
        MigrationStep::DropTable { table_name } => format!("drop table {table_name}"),
        MigrationStep::AddColumn { table_name, column } => {
            format!("add column {table_name}.{}", column.name)
        }
        MigrationStep::DropColumn {
            table_name,
            column_name,
        } => format!("drop column {table_name}.{column_name}"),
        MigrationStep::AlterColumn {
            table_name,
            before,
            after,
        } => format!(
            "alter column {table_name}.{} ({} -> {})",
            before.name, before.sql_type, after.sql_type
        ),
        MigrationStep::CreateIndex { table_name, index } => {
            format!("create index {} on {table_name}", index.name)
        }
        MigrationStep::DropIndex {
            table_name,
            index_name,
        } => format!("drop index {index_name} on {table_name}"),
        MigrationStep::CreateSearchIndex { table_name, .. } => {
            format!("create search index on {table_name}")
        }
        MigrationStep::DropSearchIndex {
            table_name,
            index_name,
        } => format!("drop search index {index_name} on {table_name}"),
        MigrationStep::AlterSearchIndex { table_name, .. } => {
            format!("alter search index on {table_name}")
        }
        MigrationStep::AddForeignKey {
            table_name,
            foreign_key,
        } => format!(
            "add foreign key {table_name}.{} -> {}.{}",
            foreign_key
                .column_pairs
                .iter()
                .map(|pair| pair.source_column.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            foreign_key.target_table,
            foreign_key
                .column_pairs
                .iter()
                .map(|pair| pair.target_column.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        MigrationStep::DropForeignKey {
            table_name,
            foreign_key,
        } => format!(
            "drop foreign key {table_name}.{} -> {}.{}",
            foreign_key
                .column_pairs
                .iter()
                .map(|pair| pair.source_column.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            foreign_key.target_table,
            foreign_key
                .column_pairs
                .iter()
                .map(|pair| pair.target_column.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        MigrationStep::SetAppendOnly {
            table_name,
            enabled,
            ..
        } => {
            format!("set append-only={enabled} on {table_name}")
        }
        MigrationStep::SetCheckConstraints { table_name, .. } => {
            format!("change check constraints on {table_name}")
        }
    }
}

pub(crate) fn format_schema_plan(plan: &PlannedMigration) -> String {
    let mut lines = vec![
        format!("planHash: {}", plan.plan_hash),
        format!(
            "sourceSchemaHash: {}",
            plan.source_schema_hash.as_deref().unwrap_or("none")
        ),
        format!("targetSchemaHash: {}", plan.target_schema_hash),
        format!("classification: {}", schema_plan_class(plan).label()),
        format!("steps: {}", plan.steps.len()),
    ];
    for (index, step) in plan.steps.iter().enumerate() {
        lines.push(format!(
            "  {}. [{}] {}",
            index + 1,
            effective_step_class(step).label(),
            sanitized_step_description(&step.step)
        ));
    }
    lines.join("\n")
}

async fn plan_schema_for_entities(
    pool: &Database,
    entities: &[&'static graphql_orm::graphql::orm::EntityMetadata],
) -> Result<Option<PlannedMigration>> {
    let schema = pool.schema();

    let validation = schema
        .validate_against_entities(entities)
        .await
        .context("GraphQL ORM schema validation failed")?;

    if !validation.has_errors() {
        info!(
            service = "database",
            "Live schema already matches GraphQL ORM entity metadata; no migration needed"
        );
        return Ok(None);
    }

    for diagnostic in &validation.diagnostics {
        info!(
            service = "database",
            severity = ?diagnostic.severity,
            kind = ?diagnostic.kind,
            table = diagnostic.table.as_deref().unwrap_or(""),
            column = diagnostic.column.as_deref().unwrap_or(""),
            message = %diagnostic.message,
            "GraphQL ORM schema drift detected"
        );
    }

    let version = format!(
        "auto-{}",
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339())
    );
    let plan = schema
        .plan_migration_to_entities(version, "sync GraphQL ORM entity schema", entities)
        .await
        .context("GraphQL ORM schema migration planning failed")?;
    Ok(Some(plan))
}

pub(crate) async fn plan_schema(pool: &Database) -> Result<Option<PlannedMigration>> {
    plan_schema_for_entities(pool, &entity_metadata()).await
}

/// The plan behind `librarian schema plan`/`apply`: a full diff of the introspected schema.
/// Startup validation does not report every constraint (a missing `UNIQUE`, for one), so the
/// reviewed path never skips planning on a clean validation; it is `None` only when the diff
/// has no steps.
pub(crate) async fn plan_reviewed_schema(pool: &Database) -> Result<Option<PlannedMigration>> {
    let version = format!(
        "reviewed-{}",
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339())
    );
    let plan = pool
        .schema()
        .plan_migration_to_entities(
            version,
            "sync GraphQL ORM entity schema",
            &entity_metadata(),
        )
        .await
        .context("GraphQL ORM schema migration planning failed")?;
    Ok((!plan.steps.is_empty()).then_some(plan))
}

async fn bootstrap_schema_for_entities(
    pool: &Database,
    entities: &[&'static graphql_orm::graphql::orm::EntityMetadata],
) -> Result<()> {
    let Some(plan) = plan_schema_for_entities(pool, entities).await? else {
        return Ok(());
    };
    apply_startup_schema_plan(pool, &plan).await
}

async fn apply_startup_schema_plan(pool: &Database, plan: &PlannedMigration) -> Result<()> {
    let classification = schema_plan_class(plan);

    if classification != SchemaChangeClass::Additive {
        for step in &plan.steps {
            warn!(
                service = "database",
                plan_hash = %plan.plan_hash,
                classification = effective_step_class(step).label(),
                reason = %step.reason,
                step = %sanitized_step_description(&step.step),
                "Non-additive GraphQL ORM schema migration step blocked at startup"
            );
        }
        anyhow::bail!(
            "Database schema changes require explicit review; ordinary startup only applies additive \
             changes.\n{}\nCreate and verify a full backup from the current version, then run:\n  \
             librarian schema apply --plan-hash {} --backup-snapshot <SNAPSHOT_ID>",
            format_schema_plan(plan),
            plan.plan_hash
        );
    }

    pool.schema()
        .apply_migration(
            plan,
            ApplyOptions {
                additive_only: true,
                expected_current_schema_hash: plan.source_schema_hash.clone(),
                ..ApplyOptions::default()
            },
        )
        .await
        .context("GraphQL ORM schema migration application failed")?;
    info!(
        service = "database",
        version = %plan.version,
        plan_hash = %plan.plan_hash,
        "Applied additive GraphQL ORM schema migration"
    );
    Ok(())
}

/// Validate the live schema against the current entity metadata and auto-apply only changes that
/// cannot transform or discard existing row data.
pub(crate) async fn bootstrap_schema(pool: &Database) -> Result<()> {
    bootstrap_schema_for_entities(pool, &entity_metadata()).await
}

/// Apply a previously reviewed plan only after a verified full backup proves that the current
/// source schema is recoverable. The caller must provide the exact plan hash printed by
/// `librarian schema plan`; replanning prevents approving one plan and applying another.
pub(crate) async fn apply_reviewed_schema_plan(
    pool: &Database,
    expected_plan_hash: &str,
    backup: &VerifiedSchemaBackup,
) -> Result<Option<graphql_orm::graphql::orm::AppliedMigrationReport>> {
    let Some(plan) = plan_reviewed_schema(pool).await? else {
        return Ok(None);
    };
    if plan.plan_hash != expected_plan_hash {
        anyhow::bail!(
            "Schema plan hash changed: reviewed {}, current {}. Run `librarian schema plan` again.",
            expected_plan_hash,
            plan.plan_hash
        );
    }
    let source_schema_hash = plan
        .source_schema_hash
        .as_deref()
        .context("Schema plan does not contain a source schema hash")?;
    if backup.source_schema_hash != source_schema_hash {
        anyhow::bail!(
            "Verified backup {} belongs to schema {}, but the current migration source is {}",
            backup.snapshot_id,
            backup.source_schema_hash,
            source_schema_hash
        );
    }

    let report = pool
        .schema()
        .apply_migration(
            &plan,
            ApplyOptions {
                allow_destructive: true,
                additive_only: false,
                expected_current_schema_hash: plan.source_schema_hash.clone(),
                ..ApplyOptions::default()
            },
        )
        .await
        .context("Reviewed GraphQL ORM schema migration application failed")?;
    info!(
        service = "database",
        version = %plan.version,
        plan_hash = %plan.plan_hash,
        backup_snapshot_id = %backup.snapshot_id,
        classification = schema_plan_class(&plan).label(),
        "Applied reviewed GraphQL ORM schema migration"
    );
    Ok(Some(report))
}

#[derive(Debug, Default)]
struct AppWriteInputTransform {
    services: Option<Weak<ServicesManager>>,
}

impl AppWriteInputTransform {
    fn services_from_ctx(
        &self,
        ctx: Option<&async_graphql::Context<'_>>,
    ) -> Option<Arc<ServicesManager>> {
        ctx.and_then(|ctx| ctx.data::<Arc<ServicesManager>>().ok().cloned())
            .or_else(|| self.services.as_ref().and_then(Weak::upgrade))
    }

    fn credentials_look_encrypted(value: &str) -> bool {
        crate::services::sources::encryption::CredentialEncryption::looks_like_envelope(value)
    }

    fn authenticated_member<'a>(
        ctx: Option<&'a async_graphql::Context<'_>>,
    ) -> Option<&'a AuthUser> {
        ctx.and_then(|ctx| ctx.data_opt::<AuthUser>())
            .filter(|user| !user.is_admin())
    }

    /// A library root on a network share is normalised (`smb://host/share/folder`, host and share
    /// in lower case) and must be reachable and a folder: a library that cannot be read from the
    /// start would only ever report itself unavailable. A local path is kept exactly as typed.
    async fn checked_library_root(path: &str) -> async_graphql::Result<String> {
        use crate::services::library_storage::{StorageKind, StorageLocation, library_storage};
        let location = match StorageLocation::parse(path) {
            Ok(location) if location.kind() == StorageKind::Smb => location,
            _ => return Ok(path.to_string()),
        };
        let root = location.root_path();
        let meta = library_storage().stat(&root).await.map_err(|error| {
            async_graphql::Error::new(format!(
                "The library folder on the share cannot be read: {error}"
            ))
        })?;
        if !meta.is_dir() {
            return Err(async_graphql::Error::new(
                "The library location on the share is not a folder",
            ));
        }
        Ok(location.to_path_string())
    }

    fn assign_create_owner(
        ctx: Option<&async_graphql::Context<'_>>,
        entity_name: &str,
        input: &mut (dyn std::any::Any + Send + Sync),
    ) {
        let Some(user) = Self::authenticated_member(ctx) else {
            return;
        };
        macro_rules! assign {
            ($name:literal, $ty:ty) => {
                if entity_name == $name {
                    if let Some(record) = input.downcast_mut::<$ty>() {
                        record.user_id = user.user_id.clone();
                    }
                    return;
                }
            };
        }
        assign!("Library", CreateLibraryInput);
        assign!("LibraryScanRun", CreateLibraryScanRunInput);
        assign!("LibraryScanIssue", CreateLibraryScanIssueInput);
        assign!("Movie", CreateMovieInput);
        assign!("Show", CreateShowInput);
        assign!("Collection", CreateCollectionInput);
        assign!("Artist", CreateArtistInput);
        assign!("Album", CreateAlbumInput);
        assign!("Audiobook", CreateAudiobookInput);
        assign!("PendingFileMatch", CreatePendingFileMatchInput);
        assign!("PlaybackProgress", CreatePlaybackProgressInput);
        assign!("PlaybackSession", CreatePlaybackSessionInput);
        assign!("Notification", CreateNotificationInput);
        assign!("RssFeed", CreateRssFeedInput);
        assign!("Torrent", CreateTorrentInput);
        assign!("UsenetDownload", CreateUsenetDownloadInput);
        assign!("NamingPattern", CreateNamingPatternInput);
        assign!("SourcePriorityRule", CreateSourcePriorityRuleInput);
        if entity_name == "CastSession"
            && let Some(record) = input.downcast_mut::<CreateCastSessionInput>()
        {
            record.user_id = Some(user.user_id.clone());
        }
    }

    fn reject_owner_update(
        ctx: Option<&async_graphql::Context<'_>>,
        entity_name: &str,
        input: &(dyn std::any::Any + Send + Sync),
    ) -> async_graphql::Result<()> {
        if Self::authenticated_member(ctx).is_none() {
            return Ok(());
        }
        macro_rules! supplied {
            ($name:literal, $ty:ty) => {
                if entity_name == $name {
                    return if input
                        .downcast_ref::<$ty>()
                        .is_some_and(|record| record.user_id.is_some())
                    {
                        Err(
                            async_graphql::Error::new("Ownership cannot be changed by a member")
                                .extend_with(|_, extensions| extensions.set("code", "FORBIDDEN")),
                        )
                    } else {
                        Ok(())
                    };
                }
            };
        }
        supplied!("Library", UpdateLibraryInput);
        supplied!("LibraryScanRun", UpdateLibraryScanRunInput);
        supplied!("LibraryScanIssue", UpdateLibraryScanIssueInput);
        supplied!("Movie", UpdateMovieInput);
        supplied!("Show", UpdateShowInput);
        supplied!("Collection", UpdateCollectionInput);
        supplied!("Artist", UpdateArtistInput);
        supplied!("Album", UpdateAlbumInput);
        supplied!("Audiobook", UpdateAudiobookInput);
        supplied!("PendingFileMatch", UpdatePendingFileMatchInput);
        supplied!("PlaybackProgress", UpdatePlaybackProgressInput);
        supplied!("PlaybackSession", UpdatePlaybackSessionInput);
        supplied!("Notification", UpdateNotificationInput);
        supplied!("RssFeed", UpdateRssFeedInput);
        supplied!("Torrent", UpdateTorrentInput);
        supplied!("UsenetDownload", UpdateUsenetDownloadInput);
        supplied!("NamingPattern", UpdateNamingPatternInput);
        supplied!("SourcePriorityRule", UpdateSourcePriorityRuleInput);
        supplied!("CastSession", UpdateCastSessionInput);
        Ok(())
    }

    async fn validate_upsert_link_workaround(
        ctx: Option<&async_graphql::Context<'_>>,
        db: &Database,
        entity_name: &str,
        input: &(dyn std::any::Any + Send + Sync),
        creating: bool,
    ) -> async_graphql::Result<()> {
        let Some(user) = Self::authenticated_member(ctx) else {
            return Ok(());
        };

        // graphql-orm's pinned upsert generator currently cannot compile
        // field-policy annotations on the two upsert-enabled entities. Apply
        // the same immutable/owned-link contract in the write transform until
        // the pinned revision gains that support.
        let valid = match entity_name {
            "Show" if creating => {
                let Some(input) = input.downcast_ref::<CreateShowInput>() else {
                    return Ok(());
                };
                crate::services::graphql::policy::AppFieldPolicy::owns_link(
                    db,
                    &user.user_id,
                    "libraryId",
                    &input.library_id,
                )
                .await?
            }
            "Episode" if creating => {
                let Some(input) = input.downcast_ref::<CreateEpisodeInput>() else {
                    return Ok(());
                };
                crate::services::graphql::policy::AppFieldPolicy::owns_link(
                    db,
                    &user.user_id,
                    "showId",
                    &input.show_id,
                )
                .await?
                    && match input.media_file_id.as_deref() {
                        Some(media_file_id) => {
                            crate::services::graphql::policy::AppFieldPolicy::owns_link(
                                db,
                                &user.user_id,
                                "mediaFileId",
                                media_file_id,
                            )
                            .await?
                        }
                        None => true,
                    }
            }
            "Show" => input
                .downcast_ref::<UpdateShowInput>()
                .is_none_or(|input| input.library_id.is_none()),
            "Episode" => input
                .downcast_ref::<UpdateEpisodeInput>()
                .is_none_or(|input| input.show_id.is_none() && input.media_file_id.is_none()),
            _ => true,
        };
        if valid {
            Ok(())
        } else {
            Err(
                async_graphql::Error::new("Ownership links cannot be changed by a member")
                    .extend_with(|_, extensions| extensions.set("code", "FORBIDDEN")),
            )
        }
    }

    fn reject_member_artwork_source_write(
        ctx: Option<&async_graphql::Context<'_>>,
        entity_name: &str,
        input: &(dyn std::any::Any + Send + Sync),
    ) -> async_graphql::Result<()> {
        let is_member = ctx
            .and_then(|ctx| ctx.data_opt::<AuthUser>())
            .is_some_and(|user| !user.is_admin());
        if !is_member {
            return Ok(());
        }

        let contains_artwork_source = match entity_name {
            "Movie" => {
                input
                    .downcast_ref::<CreateMovieInput>()
                    .is_some_and(|input| input.collection_poster_url.is_some())
                    || input
                        .downcast_ref::<UpdateMovieInput>()
                        .is_some_and(|input| input.collection_poster_url.is_some())
            }
            "Show" => {
                input
                    .downcast_ref::<CreateShowInput>()
                    .is_some_and(|input| input.poster_url.is_some() || input.backdrop_url.is_some())
                    || input
                        .downcast_ref::<UpdateShowInput>()
                        .is_some_and(|input| {
                            input.poster_url.is_some() || input.backdrop_url.is_some()
                        })
            }
            "Collection" => {
                input
                    .downcast_ref::<CreateCollectionInput>()
                    .is_some_and(|input| input.poster_url.is_some() || input.backdrop_url.is_some())
                    || input
                        .downcast_ref::<UpdateCollectionInput>()
                        .is_some_and(|input| {
                            input.poster_url.is_some() || input.backdrop_url.is_some()
                        })
            }
            "Album" => {
                input
                    .downcast_ref::<CreateAlbumInput>()
                    .is_some_and(|input| input.cover_url.is_some())
                    || input
                        .downcast_ref::<UpdateAlbumInput>()
                        .is_some_and(|input| input.cover_url.is_some())
            }
            "Audiobook" => {
                input
                    .downcast_ref::<CreateAudiobookInput>()
                    .is_some_and(|input| input.cover_url.is_some())
                    || input
                        .downcast_ref::<UpdateAudiobookInput>()
                        .is_some_and(|input| input.cover_url.is_some())
            }
            // A still URL is a download source like a poster: only metadata sync sets it.
            "Episode" => {
                input
                    .downcast_ref::<CreateEpisodeInput>()
                    .is_some_and(|input| input.still_url.is_some())
                    || input
                        .downcast_ref::<UpdateEpisodeInput>()
                        .is_some_and(|input| input.still_url.is_some())
            }
            "Artist" => {
                input
                    .downcast_ref::<CreateArtistInput>()
                    .is_some_and(|input| input.image_url.is_some())
                    || input
                        .downcast_ref::<UpdateArtistInput>()
                        .is_some_and(|input| input.image_url.is_some())
            }
            _ => false,
        };

        if contains_artwork_source {
            return Err(async_graphql::Error::new(
                "Artwork source URLs are managed by administrators and metadata providers",
            )
            .extend_with(|_, extensions| extensions.set("code", "FORBIDDEN")));
        }
        Ok(())
    }

    /// Cast content rules live here, not in a hand-written twin of the generated
    /// mutation, so every write path is held to them. See `cast::write_rules`.
    fn prepare_cast_create(
        entity_name: &str,
        input: &mut (dyn std::any::Any + Send + Sync),
    ) -> async_graphql::Result<()> {
        use crate::services::cast::write_rules;
        match entity_name {
            "CastDevice" => input
                .downcast_mut::<CreateCastDeviceInput>()
                .map_or(Ok(()), write_rules::prepare_device_create),
            "CastSetting" => input
                .downcast_mut::<CreateCastSettingInput>()
                .map_or(Ok(()), write_rules::prepare_setting_create),
            _ => Ok(()),
        }
    }

    fn prepare_cast_update(
        entity_name: &str,
        existing_row: Option<&(dyn std::any::Any + Send + Sync)>,
        input: &mut (dyn std::any::Any + Send + Sync),
    ) -> async_graphql::Result<()> {
        use crate::services::cast::write_rules;
        match entity_name {
            "CastDevice" => input
                .downcast_mut::<UpdateCastDeviceInput>()
                .map_or(Ok(()), |input| {
                    write_rules::prepare_device_update(
                        existing_row.and_then(|row| row.downcast_ref::<CastDevice>()),
                        input,
                    )
                }),
            "CastSetting" => input
                .downcast_mut::<UpdateCastSettingInput>()
                .map_or(Ok(()), write_rules::prepare_setting_update),
            _ => Ok(()),
        }
    }

    /// Content rules for the entities outside Cast. See `services::write_rules`.
    fn prepare_content_create(
        entity_name: &str,
        input: &mut (dyn std::any::Any + Send + Sync),
    ) -> async_graphql::Result<()> {
        use crate::services::write_rules;
        // Everything with a title sorts the same way, however it was added.
        macro_rules! sort_key {
            ($ty:ty, $title:ident, $key:ident) => {
                if let Some(input) = input.downcast_mut::<$ty>() {
                    input.$key = write_rules::sort_key_for_new(&input.$title, input.$key.take());
                }
            };
        }
        match entity_name {
            "User" => {
                return input
                    .downcast_mut::<CreateUserInput>()
                    .map_or(Ok(()), write_rules::prepare_user_create);
            }
            "RssFeed" => {
                return input
                    .downcast_mut::<CreateRssFeedInput>()
                    .map_or(Ok(()), write_rules::prepare_feed_create);
            }
            "Movie" => sort_key!(CreateMovieInput, title, sort_title),
            "Audiobook" => sort_key!(CreateAudiobookInput, title, sort_title),
            "Show" => sort_key!(CreateShowInput, name, sort_name),
            "Album" => sort_key!(CreateAlbumInput, name, sort_name),
            "Artist" => sort_key!(CreateArtistInput, name, sort_name),
            "Collection" => sort_key!(CreateCollectionInput, name, sort_name),
            _ => {}
        }
        Ok(())
    }

    fn prepare_content_update(
        ctx: Option<&async_graphql::Context<'_>>,
        entity_name: &str,
        existing_row: Option<&(dyn std::any::Any + Send + Sync)>,
        input: &mut (dyn std::any::Any + Send + Sync),
    ) -> async_graphql::Result<()> {
        use crate::services::write_rules;
        // A rename moves the sort key along, unless someone set their own or is setting it now.
        macro_rules! sort_key {
            ($input:ty, $row:ty, $title:ident, $key:ident) => {
                if let (Some(input), Some(row)) = (
                    input.downcast_mut::<$input>(),
                    existing_row.and_then(|row| row.downcast_ref::<$row>()),
                ) {
                    let cleared = matches!(
                        &input.$key,
                        Some(key) if key.as_deref().is_none_or(|key| key.trim().is_empty())
                    );
                    if cleared {
                        // Blanking it hands the sort order back to the title.
                        let title = input.$title.as_deref().unwrap_or(&row.$title);
                        input.$key = Some(Some(write_rules::sort_key(title)));
                    } else if input.$key.is_none()
                        && let Some(new_title) = input.$title.as_deref()
                    {
                        input.$key = write_rules::sort_key_after_rename(
                            &row.$title,
                            row.$key.as_deref(),
                            new_title,
                        )
                        .map(Some);
                    }
                }
            };
        }
        match entity_name {
            "Movie" => sort_key!(UpdateMovieInput, Movie, title, sort_title),
            "Audiobook" => sort_key!(UpdateAudiobookInput, Audiobook, title, sort_title),
            "Show" => sort_key!(UpdateShowInput, Show, name, sort_name),
            "Album" => sort_key!(UpdateAlbumInput, Album, name, sort_name),
            "Artist" => sort_key!(UpdateArtistInput, Artist, name, sort_name),
            "Collection" => sort_key!(UpdateCollectionInput, Collection, name, sort_name),
            _ => {}
        }
        match entity_name {
            "User" => input
                .downcast_mut::<UpdateUserInput>()
                .map_or(Ok(()), write_rules::prepare_user_update),
            "RssFeed" => input
                .downcast_mut::<UpdateRssFeedInput>()
                .map_or(Ok(()), write_rules::prepare_feed_update),
            "LibraryScanIssue" => {
                input
                    .downcast_mut::<UpdateLibraryScanIssueInput>()
                    .map_or(Ok(()), |input| {
                        write_rules::prepare_scan_issue_update(
                            existing_row.and_then(|row| row.downcast_ref::<LibraryScanIssue>()),
                            input,
                        )
                    })
            }
            "MediaFile" => {
                if let Some(input) = input.downcast_mut::<UpdateMediaFileInput>() {
                    write_rules::prepare_media_file_update(
                        ctx.and_then(|ctx| ctx.data_opt::<AuthUser>()),
                        existing_row.and_then(|row| row.downcast_ref::<MediaFile>()),
                        input,
                        &AppMutationHook::now_rfc3339(),
                    );
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn encrypt_source_credentials(
        &self,
        ctx: Option<&async_graphql::Context<'_>>,
        plaintext: String,
    ) -> async_graphql::Result<String> {
        if plaintext.is_empty() || Self::credentials_look_encrypted(&plaintext) {
            return Ok(plaintext);
        }

        if let Some(ctx) = ctx {
            return crate::services::sources::encryption::encrypt_credentials_ctx(ctx, plaintext)
                .await;
        }

        let services = self.services_from_ctx(None).ok_or_else(|| {
            async_graphql::Error::new(
                "ServicesManager not available for source credential transform",
            )
        })?;
        let sources_svc = services
            .get_sources()
            .await
            .ok_or_else(|| async_graphql::Error::new("Sources service not available"))?;
        let sources_manager = sources_svc
            .get_manager()
            .await
            .ok_or_else(|| async_graphql::Error::new("Sources manager not initialized"))?;
        sources_manager
            .encryption()
            .encrypt_envelope(&plaintext)
            .map_err(|_| async_graphql::Error::new("Credential encryption failed"))
    }
}

impl WriteInputTransform for AppWriteInputTransform {
    fn before_update_with_context<'a>(
        &'a self,
        write_ctx: &'a mut graphql_orm::graphql::orm::WriteInputContext<'_, '_>,
        existing_row: Option<&'a (dyn std::any::Any + Send + Sync)>,
        input: &'a mut (dyn std::any::Any + Send + Sync),
    ) -> BoxFuture<'a, async_graphql::Result<()>> {
        Box::pin(async move {
            self.before_update(
                write_ctx.graphql_ctx(),
                write_ctx.database(),
                write_ctx.entity_name(),
                existing_row,
                input,
            )
            .await?;

            // Generated update holds its MutationContext before field policies
            // run. Validate this mutable link through WriteQuery, which uses
            // that transaction, rather than re-entering the connection pool.
            if write_ctx.entity_name() == "Torrent" && write_ctx.graphql_ctx().is_some() {
                let torrent_input =
                    input.downcast_ref::<UpdateTorrentInput>().ok_or_else(|| {
                        async_graphql::Error::new("Invalid torrent update input")
                            .extend_with(|_, extensions| extensions.set("code", "FORBIDDEN"))
                    })?;
                if let Some(library_id) = &torrent_input.library_id {
                    let user = write_ctx.actor::<AuthUser>();
                    AppFieldPolicy::validate_mutable_torrent_library(
                        write_ctx,
                        user.as_ref(),
                        existing_row,
                        Some(library_id),
                    )
                    .await?;
                }
            }
            Ok(())
        })
    }

    fn before_create<'a>(
        &'a self,
        ctx: Option<&'a async_graphql::Context<'_>>,
        db: &'a Database,
        entity_name: &'static str,
        input: &'a mut (dyn std::any::Any + Send + Sync),
    ) -> BoxFuture<'a, async_graphql::Result<()>> {
        Box::pin(async move {
            crate::services::graphql::personal_policy::prepare_create(ctx, entity_name, input)?;
            Self::assign_create_owner(ctx, entity_name, input);
            Self::validate_upsert_link_workaround(ctx, db, entity_name, input, true).await?;
            Self::reject_member_artwork_source_write(ctx, entity_name, input)?;
            Self::prepare_cast_create(entity_name, input)?;
            Self::prepare_content_create(entity_name, input)?;
            if entity_name == "Library"
                && let Some(input) = input.downcast_mut::<CreateLibraryInput>()
            {
                input.path = Self::checked_library_root(&input.path).await?;
            }
            if entity_name == "Source"
                && let Some(input) = input.downcast_mut::<CreateSourceInput>()
            {
                input.credentials = self
                    .encrypt_source_credentials(ctx, input.credentials.clone())
                    .await?;
            }
            if entity_name == "MatchingAssistant"
                && let Some(input) = input.downcast_mut::<CreateMatchingAssistantInput>()
            {
                input.has_api_key = !input.api_key.is_empty();
                input.api_key = self
                    .encrypt_source_credentials(ctx, std::mem::take(&mut input.api_key))
                    .await?;
            }
            Ok(())
        })
    }

    fn before_update<'a>(
        &'a self,
        ctx: Option<&'a async_graphql::Context<'_>>,
        db: &'a Database,
        entity_name: &'static str,
        existing_row: Option<&'a (dyn std::any::Any + Send + Sync)>,
        input: &'a mut (dyn std::any::Any + Send + Sync),
    ) -> BoxFuture<'a, async_graphql::Result<()>> {
        Box::pin(async move {
            crate::services::graphql::personal_policy::prepare_update(
                ctx,
                entity_name,
                existing_row,
                input,
            )?;
            Self::reject_owner_update(ctx, entity_name, input)?;
            Self::validate_upsert_link_workaround(ctx, db, entity_name, input, false).await?;
            Self::reject_member_artwork_source_write(ctx, entity_name, input)?;
            Self::prepare_cast_update(entity_name, existing_row, input)?;
            Self::prepare_content_update(ctx, entity_name, existing_row, input)?;
            if entity_name == "Library"
                && let Some(input) = input.downcast_mut::<UpdateLibraryInput>()
                && let Some(path) = input.path.as_mut()
            {
                // Saving other settings sends the path again; only a changed path is checked, so
                // a share that is down does not stop a person changing a scan interval.
                let unchanged = existing_row
                    .and_then(|row| row.downcast_ref::<Library>())
                    .is_some_and(|library| library.path == *path);
                if !unchanged {
                    *path = Self::checked_library_root(path).await?;
                }
            }
            if entity_name == "Source"
                && let Some(input) = input.downcast_mut::<UpdateSourceInput>()
                && let Some(credentials) = input.credentials.take()
            {
                input.credentials = Some(self.encrypt_source_credentials(ctx, credentials).await?);
            }
            if entity_name == "MatchingAssistant"
                && let Some(input) = input.downcast_mut::<UpdateMatchingAssistantInput>()
            {
                // Whether a key is stored follows the key itself, and nothing else can claim it.
                input.has_api_key = None;
                if let Some(key) = input.api_key.take() {
                    input.has_api_key = Some(!key.is_empty());
                    input.api_key = Some(self.encrypt_source_credentials(ctx, key).await?);
                }
            }
            Ok(())
        })
    }
}

#[derive(Debug, Default)]
struct AppPostCommitErrorHandler;

impl PostCommitErrorHandler for AppPostCommitErrorHandler {
    fn on_post_commit_error<'a>(&'a self, _db: &'a Database, error: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            warn!(
                error = %error,
                "GraphQL ORM deferred post-commit lifecycle hook failed"
            );
        })
    }
}

/// MutationQuery inherits connection bounds (and the pinned renderer caps a page at 100).
/// Read all stable primary-key pages on the same transaction instead of mistaking one page
/// for a canonical aggregate. Repository hook reads are trusted; request ACLs belong at entry.
async fn transaction_rows<T, F>(
    tx: &mut MutationContext<'_>,
    filter: F,
) -> graphql_orm::Result<Vec<T>>
where
    T: Entity + FromSqlRow<DefaultBackend> + Clone + Send + Sync + 'static,
    F: DatabaseFilter + Clone + Send + Sync,
{
    let limit = tx
        .database()
        .pagination_config()
        .max_limit
        .unwrap_or(100)
        .min(100);
    if limit < 1 {
        return Err(graphql_orm::Error::Protocol(
            "summary maintenance requires a positive page limit".to_string(),
        ));
    }
    let mut rows = Vec::new();
    let mut offset = 0_i64;
    loop {
        let page = tx
            .query::<T>()
            .filter(filter.clone())
            .limit(limit)
            .offset(offset)
            .fetch_all()
            .await?;
        let complete = page.len() < limit as usize;
        rows.extend(page);
        if complete {
            return Ok(rows);
        }
        offset = offset.checked_add(limit).ok_or_else(|| {
            graphql_orm::Error::Protocol("summary page offset overflow".to_string())
        })?;
    }
}

/// Explicit maintenance result. Counts describe parents, not tracks/chapters or events.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StoredSummaryReconciliation {
    pub artists_examined: u64,
    pub artists_changed: u64,
    pub albums_examined: u64,
    pub albums_changed: u64,
    pub audiobooks_examined: u64,
    pub audiobooks_changed: u64,
    pub dry_run: bool,
}

/// Admin/system maintenance entry point; the GraphQL action must establish admin authority.
/// Never run automatically on startup. A state-machine transaction pins all typed reads and
/// writes; dry run performs no writes/events, and a failed apply rolls back the whole repair.
pub(crate) async fn reconcile_stored_media_summaries(
    db: &Database,
    dry_run: bool,
) -> Result<StoredSummaryReconciliation> {
    db.transaction(TransactionMode::StateMachine, move |tx| {
        Box::pin(async move {
            let albums = transaction_rows::<Album, _>(tx, AlbumWhereInput::default()).await?;
            let books =
                transaction_rows::<Audiobook, _>(tx, AudiobookWhereInput::default()).await?;
            let mut report = StoredSummaryReconciliation {
                dry_run,
                ..Default::default()
            };
            for artist in transaction_rows::<Artist, _>(tx, ArtistWhereInput::default()).await? {
                report.artists_examined += 1;
                if reconcile_artist_summary(tx, &artist.id, dry_run, false).await? {
                    report.artists_changed += 1;
                }
            }
            for album in albums {
                report.albums_examined += 1;
                if reconcile_album_summary(tx, &album.id, dry_run, false).await? {
                    report.albums_changed += 1;
                }
            }
            for book in books {
                report.audiobooks_examined += 1;
                if reconcile_audiobook_summary(tx, &book.id, dry_run, false).await? {
                    report.audiobooks_changed += 1;
                }
            }
            Ok(report)
        })
    })
    .await
    .map_err(anyhow::Error::from)
}

/// Artist membership follows explicit stored artist IDs, never an album fallback for
/// tracks. A malformed cross-library link must not contribute to another library's totals.
async fn reconcile_artist_summary(
    tx: &mut MutationContext<'_>,
    id: &str,
    dry_run: bool,
    invalidate: bool,
) -> graphql_orm::Result<bool> {
    let Some(parent) = tx.find_by_id::<Artist>(&id.to_string()).await? else {
        return Ok(false);
    };
    let album_count = tx
        .query::<Album>()
        .filter(AlbumWhereInput {
            artist_id: Some(AppMutationHook::string_eq(id)),
            library_id: Some(AppMutationHook::string_eq(&parent.library_id)),
            ..Default::default()
        })
        .count()
        .await?;
    let tracks = transaction_rows::<Track, _>(
        tx,
        TrackWhereInput {
            artist_id: Some(AppMutationHook::string_eq(id)),
            library_id: Some(AppMutationHook::string_eq(&parent.library_id)),
            ..Default::default()
        },
    )
    .await?;
    let album_count = i32::try_from(album_count)
        .map_err(|_| graphql_orm::Error::Protocol("artist album count overflow".into()))?;
    let track_count = i32::try_from(tracks.len())
        .map_err(|_| graphql_orm::Error::Protocol("artist track count overflow".into()))?;
    let mut duration = 0_i32;
    for track in tracks {
        let value = track.duration_secs.unwrap_or(0);
        if value < 0 {
            return Err(graphql_orm::Error::Protocol(format!(
                "negative track duration for {}",
                track.id
            )));
        }
        duration = duration
            .checked_add(value)
            .ok_or_else(|| graphql_orm::Error::Protocol("artist duration sum overflow".into()))?;
    }
    let changed = parent.album_count != Some(album_count)
        || parent.track_count != Some(track_count)
        || parent.total_duration_secs != Some(duration);
    if !dry_run && (changed || invalidate) {
        tx.update_by_id::<Artist>(
            &parent.id,
            UpdateArtistInput {
                album_count: changed.then_some(Some(album_count)),
                track_count: changed.then_some(Some(track_count)),
                total_duration_secs: changed.then_some(Some(duration)),
                ..Default::default()
            },
        )
        .await?;
    }
    Ok(changed)
}

/// The canonical summary follows playable child links. One file shared by several chapters
/// contributes its bytes once. A dangling link cannot make content available; zero is a
/// known empty size. Provider track/chapter totals and durations are not local-file counts.
async fn linked_file_summary(
    tx: &mut MutationContext<'_>,
    file_ids: BTreeSet<String>,
) -> graphql_orm::Result<(bool, i64)> {
    let mut has_files = false;
    let mut size = 0_i64;
    for id in file_ids {
        if let Some(file) = tx.find_by_id::<MediaFile>(&id).await? {
            has_files = true;
            if file.size < 0 {
                return Err(graphql_orm::Error::Protocol(format!(
                    "negative media size for {}",
                    file.id
                )));
            }
            size = size.checked_add(file.size).ok_or_else(|| {
                graphql_orm::Error::Protocol("linked media size sum overflow".to_string())
            })?;
        }
    }
    Ok((has_files, size))
}

async fn reconcile_album_summary(
    tx: &mut MutationContext<'_>,
    id: &str,
    dry_run: bool,
    invalidate: bool,
) -> graphql_orm::Result<bool> {
    let Some(parent) = tx.find_by_id::<Album>(&id.to_string()).await? else {
        return Ok(false);
    };
    let tracks = transaction_rows::<Track, _>(
        tx,
        TrackWhereInput {
            album_id: Some(AppMutationHook::string_eq(id)),
            ..Default::default()
        },
    )
    .await?;
    let (has_files, size) = linked_file_summary(
        tx,
        tracks
            .into_iter()
            .filter_map(|track| track.media_file_id)
            .collect(),
    )
    .await?;
    let changed = parent.has_files != has_files || parent.size_bytes != Some(size);
    if !dry_run && (changed || invalidate) {
        tx.update_by_id::<Album>(
            &parent.id,
            UpdateAlbumInput {
                has_files: changed.then_some(has_files),
                size_bytes: changed.then_some(Some(size)),
                ..Default::default()
            },
        )
        .await?;
    }
    Ok(changed)
}

async fn reconcile_audiobook_summary(
    tx: &mut MutationContext<'_>,
    id: &str,
    dry_run: bool,
    invalidate: bool,
) -> graphql_orm::Result<bool> {
    let Some(parent) = tx.find_by_id::<Audiobook>(&id.to_string()).await? else {
        return Ok(false);
    };
    let chapters = transaction_rows::<Chapter, _>(
        tx,
        ChapterWhereInput {
            audiobook_id: Some(AppMutationHook::string_eq(id)),
            ..Default::default()
        },
    )
    .await?;
    let (has_files, size) = linked_file_summary(
        tx,
        chapters
            .into_iter()
            .filter_map(|chapter| chapter.media_file_id)
            .collect(),
    )
    .await?;
    let changed = parent.has_files != has_files || parent.size_bytes != Some(size);
    if !dry_run && (changed || invalidate) {
        tx.update_by_id::<Audiobook>(
            &parent.id,
            UpdateAudiobookInput {
                has_files: changed.then_some(has_files),
                size_bytes: changed.then_some(Some(size)),
                ..Default::default()
            },
        )
        .await?;
    }
    Ok(changed)
}

/// `artwork_type` of a resized copy is the original's type, this marker and a width
/// (`poster@w300`); see `ArtworkService`.
const ARTWORK_VARIANT_MARKER: &str = "@w";

#[derive(Debug, Default)]
struct AppMutationHook {
    services: Option<Weak<ServicesManager>>,
}

/// A direct parent write gives delivery current parent provenance after a child
/// disappears/moves. Stable create/update propagation remains the entity metadata's job.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ParentInvalidation(&'static str, String);

impl AppMutationHook {
    fn db_error(error: impl std::fmt::Display) -> async_graphql::Error {
        async_graphql::Error::new(error.to_string())
    }

    fn before<'a, T: 'static>(
        event: &'a MutationEvent,
        entity_name: &str,
    ) -> async_graphql::Result<&'a T> {
        event.before::<T>()?.ok_or_else(|| {
            async_graphql::Error::new(format!(
                "Missing before_state for {entity_name} lifecycle hook"
            ))
        })
    }

    fn after<'a, T: 'static>(
        event: &'a MutationEvent,
        entity_name: &str,
    ) -> async_graphql::Result<&'a T> {
        event.after::<T>()?.ok_or_else(|| {
            async_graphql::Error::new(format!(
                "Missing after_state for {entity_name} lifecycle hook"
            ))
        })
    }

    fn string_eq(value: impl Into<String>) -> StringFilter {
        StringFilter {
            eq: Some(value.into()),
            ..StringFilter::default()
        }
    }

    fn now_rfc3339() -> String {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339())
    }

    fn services_from_ctx(
        &self,
        ctx: Option<&async_graphql::Context<'_>>,
    ) -> Option<Arc<ServicesManager>> {
        ctx.and_then(|ctx| ctx.data::<Arc<ServicesManager>>().ok().cloned())
            .or_else(|| self.services.as_ref().and_then(Weak::upgrade))
    }

    async fn cleanup_library_children(
        hook_ctx: &mut MutationContext<'_>,
        library_id: &str,
    ) -> async_graphql::Result<()> {
        hook_ctx
            .delete_where::<Show>(ShowWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..ShowWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .delete_where::<Movie>(MovieWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..MovieWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .delete_where::<Collection>(CollectionWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..CollectionWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .delete_where::<Album>(AlbumWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..AlbumWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .delete_where::<Artist>(ArtistWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..ArtistWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .delete_where::<Audiobook>(AudiobookWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..AudiobookWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        // Scan history is owned by its library, not retained as inaccessible orphan rows.
        hook_ctx
            .delete_where::<LibraryScanIssue>(LibraryScanIssueWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..Default::default()
            })
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .delete_where::<LibraryScanRun>(LibraryScanRunWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..Default::default()
            })
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .delete_where::<MediaFile>(MediaFileWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..MediaFileWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .delete_where::<RssFeed>(RssFeedWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..RssFeedWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .delete_where::<SourcePriorityRule>(SourcePriorityRuleWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..SourcePriorityRuleWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .delete_where::<Notification>(NotificationWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..NotificationWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .update_where::<Torrent>(
                TorrentWhereInput {
                    library_id: Some(Self::string_eq(library_id)),
                    ..TorrentWhereInput::default()
                },
                UpdateTorrentInput {
                    library_id: Some(None),
                    ..UpdateTorrentInput::default()
                },
            )
            .await
            .map_err(Self::db_error)?;
        hook_ctx
            .update_where::<UsenetDownload>(
                UsenetDownloadWhereInput {
                    library_id: Some(Self::string_eq(library_id)),
                    ..UsenetDownloadWhereInput::default()
                },
                UpdateUsenetDownloadInput {
                    library_id: Some(None),
                    ..UpdateUsenetDownloadInput::default()
                },
            )
            .await
            .map_err(Self::db_error)?;

        Ok(())
    }

    async fn cleanup_parent_children(
        tx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        match event.entity_name {
            "Show" => {
                tx.delete_where::<Episode>(EpisodeWhereInput {
                    show_id: Some(Self::string_eq(&event.id)),
                    ..Default::default()
                })
                .await
                .map_err(Self::db_error)?;
                // Through the entity layer, not the FK cascade, so season hooks (artwork
                // cleanup, subscriptions) run for each season.
                tx.delete_where::<Season>(SeasonWhereInput {
                    show_id: Some(Self::string_eq(&event.id)),
                    ..Default::default()
                })
                .await
                .map_err(Self::db_error)?;
            }
            "Album" => {
                tx.delete_where::<Track>(TrackWhereInput {
                    album_id: Some(Self::string_eq(&event.id)),
                    ..Default::default()
                })
                .await
                .map_err(Self::db_error)?;
            }
            "Audiobook" => {
                tx.delete_where::<Chapter>(ChapterWhereInput {
                    audiobook_id: Some(Self::string_eq(&event.id)),
                    ..Default::default()
                })
                .await
                .map_err(Self::db_error)?;
            }
            "Movie" => {
                tx.delete_where::<MovieCastCredit>(MovieCastCreditWhereInput {
                    movie_id: Some(Self::string_eq(&event.id)),
                    ..Default::default()
                })
                .await
                .map_err(Self::db_error)?;
            }
            "RssFeed" => {
                tx.delete_where::<RssFeedItem>(RssFeedItemWhereInput {
                    feed_id: Some(Self::string_eq(&event.id)),
                    ..Default::default()
                })
                .await
                .map_err(Self::db_error)?;
            }
            // The credit's physical foreign key cascades without hooks or events. Deleting
            // the credits here gives each one a delete event, which invalidates its movie.
            "Person" => {
                tx.delete_where::<MovieCastCredit>(MovieCastCreditWhereInput {
                    person_id: Some(Self::string_eq(&event.id)),
                    ..Default::default()
                })
                .await
                .map_err(Self::db_error)?;
            }
            "LibraryScanRun" => {
                tx.delete_where::<LibraryScanIssue>(LibraryScanIssueWhereInput {
                    scan_run_id: Some(Self::string_eq(&event.id)),
                    ..Default::default()
                })
                .await
                .map_err(Self::db_error)?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn detach_deleted_title(
        tx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        match event.entity_name {
            "Movie" => {
                tx.update_where::<MediaFile>(
                    MediaFileWhereInput {
                        movie_id: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    },
                    UpdateMediaFileInput {
                        movie_id: Some(None),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?;
                // Pending acquisition records retain source/debug history, but not dead title IDs.
                tx.update_where::<PendingFileMatch>(
                    PendingFileMatchWhereInput {
                        movie_id: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    },
                    UpdatePendingFileMatchInput {
                        movie_id: Some(None),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?;
            }
            "Episode" => {
                tx.update_where::<MediaFile>(
                    MediaFileWhereInput {
                        episode_id: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    },
                    UpdateMediaFileInput {
                        episode_id: Some(None),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?;
                // Pending acquisition records retain source/debug history, but not dead title IDs.
                tx.update_where::<PendingFileMatch>(
                    PendingFileMatchWhereInput {
                        episode_id: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    },
                    UpdatePendingFileMatchInput {
                        episode_id: Some(None),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?;
            }
            "Track" => {
                tx.update_where::<MediaFile>(
                    MediaFileWhereInput {
                        track_id: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    },
                    UpdateMediaFileInput {
                        track_id: Some(None),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?;
                // Pending acquisition records retain source/debug history, but not dead title IDs.
                tx.update_where::<PendingFileMatch>(
                    PendingFileMatchWhereInput {
                        track_id: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    },
                    UpdatePendingFileMatchInput {
                        track_id: Some(None),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?;
            }
            "Chapter" => {
                tx.update_where::<MediaFile>(
                    MediaFileWhereInput {
                        chapter_id: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    },
                    UpdateMediaFileInput {
                        chapter_id: Some(None),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?;
                // Pending acquisition records retain source/debug history, but not dead title IDs.
                tx.update_where::<PendingFileMatch>(
                    PendingFileMatchWhereInput {
                        chapter_id: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    },
                    UpdatePendingFileMatchInput {
                        chapter_id: Some(None),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Clear ids that point at a row about to be deleted, for the relations no physical foreign
    /// key covers (`emit_fk = false`). Each relation's `on_delete` states what happens here.
    /// Rows keep their history; they lose only the dead id, through the entity layer so each
    /// cleared row emits its own update event.
    async fn detach_deleted_references(
        tx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        macro_rules! clear {
            ($ty:ty, $filter:ident, $update:ident, $field:ident) => {
                tx.update_where::<$ty>(
                    $filter {
                        $field: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    },
                    $update {
                        $field: Some(None),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?;
            };
        }
        match event.entity_name {
            "Movie" => {
                clear!(Torrent, TorrentWhereInput, UpdateTorrentInput, movie_id);
                clear!(
                    UsenetDownload,
                    UsenetDownloadWhereInput,
                    UpdateUsenetDownloadInput,
                    movie_id
                );
                clear!(
                    PlaybackSession,
                    PlaybackSessionWhereInput,
                    UpdatePlaybackSessionInput,
                    movie_id
                );
                clear!(
                    ReleaseBlocklist,
                    ReleaseBlocklistWhereInput,
                    UpdateReleaseBlocklistInput,
                    movie_id
                );
            }
            "Show" => {
                clear!(Torrent, TorrentWhereInput, UpdateTorrentInput, show_id);
                clear!(
                    PlaybackSession,
                    PlaybackSessionWhereInput,
                    UpdatePlaybackSessionInput,
                    tv_show_id
                );
                clear!(
                    ReleaseBlocklist,
                    ReleaseBlocklistWhereInput,
                    UpdateReleaseBlocklistInput,
                    show_id
                );
            }
            "Episode" => {
                clear!(Torrent, TorrentWhereInput, UpdateTorrentInput, episode_id);
                clear!(
                    UsenetDownload,
                    UsenetDownloadWhereInput,
                    UpdateUsenetDownloadInput,
                    episode_id
                );
                clear!(
                    PlaybackSession,
                    PlaybackSessionWhereInput,
                    UpdatePlaybackSessionInput,
                    episode_id
                );
                clear!(
                    CastSession,
                    CastSessionWhereInput,
                    UpdateCastSessionInput,
                    episode_id
                );
            }
            "Album" => {
                clear!(Torrent, TorrentWhereInput, UpdateTorrentInput, album_id);
                clear!(
                    UsenetDownload,
                    UsenetDownloadWhereInput,
                    UpdateUsenetDownloadInput,
                    album_id
                );
                clear!(
                    PlaybackSession,
                    PlaybackSessionWhereInput,
                    UpdatePlaybackSessionInput,
                    album_id
                );
                clear!(
                    ReleaseBlocklist,
                    ReleaseBlocklistWhereInput,
                    UpdateReleaseBlocklistInput,
                    album_id
                );
            }
            "Audiobook" => {
                clear!(Torrent, TorrentWhereInput, UpdateTorrentInput, audiobook_id);
                clear!(
                    UsenetDownload,
                    UsenetDownloadWhereInput,
                    UpdateUsenetDownloadInput,
                    audiobook_id
                );
                clear!(
                    PlaybackSession,
                    PlaybackSessionWhereInput,
                    UpdatePlaybackSessionInput,
                    audiobook_id
                );
                clear!(
                    ReleaseBlocklist,
                    ReleaseBlocklistWhereInput,
                    UpdateReleaseBlocklistInput,
                    audiobook_id
                );
            }
            "Track" => {
                clear!(Torrent, TorrentWhereInput, UpdateTorrentInput, track_id);
                clear!(
                    PlaybackSession,
                    PlaybackSessionWhereInput,
                    UpdatePlaybackSessionInput,
                    track_id
                );
            }
            "Chapter" => {
                clear!(Torrent, TorrentWhereInput, UpdateTorrentInput, chapter_id);
            }
            "Torrent" => {
                clear!(
                    Notification,
                    NotificationWhereInput,
                    UpdateNotificationInput,
                    torrent_id
                );
                clear!(
                    RssFeedItem,
                    RssFeedItemWhereInput,
                    UpdateRssFeedItemInput,
                    torrent_id
                );
            }
            "RssFeed" => {
                clear!(
                    Torrent,
                    TorrentWhereInput,
                    UpdateTorrentInput,
                    source_feed_id
                );
            }
            "PendingFileMatch" => {
                clear!(
                    Notification,
                    NotificationWhereInput,
                    UpdateNotificationInput,
                    pending_match_id
                );
            }
            // Albums cannot lose their artist (`Album.artist_id` is required), so an artist
            // that still has albums is not deleted. Credits on other artists' tracks are
            // only a nullable credit and are cleared.
            "Artist" => {
                let albums = tx
                    .query::<Album>()
                    .filter(AlbumWhereInput {
                        artist_id: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    })
                    .limit(1)
                    .fetch_all()
                    .await
                    .map_err(Self::db_error)?;
                if !albums.is_empty() {
                    return Err(async_graphql::Error::new(
                        "This artist still has albums. Delete or move them first.",
                    )
                    .extend_with(|_, extensions| extensions.set("code", "CONFLICT")));
                }
                clear!(Track, TrackWhereInput, UpdateTrackInput, artist_id);
            }
            "QualityProfile" => {
                clear!(
                    Library,
                    LibraryWhereInput,
                    UpdateLibraryInput,
                    quality_profile_id
                );
                clear!(Movie, MovieWhereInput, UpdateMovieInput, quality_profile_id);
                clear!(Show, ShowWhereInput, UpdateShowInput, quality_profile_id);
                clear!(Album, AlbumWhereInput, UpdateAlbumInput, quality_profile_id);
                clear!(
                    Audiobook,
                    AudiobookWhereInput,
                    UpdateAudiobookInput,
                    quality_profile_id
                );
            }
            "TorznabCategory" => {
                clear!(
                    TorznabCategory,
                    TorznabCategoryWhereInput,
                    UpdateTorznabCategoryInput,
                    parent_id
                );
            }
            _ => {}
        }
        Ok(())
    }

    async fn cleanup_media_file_dependents(
        tx: &mut MutationContext<'_>,
        id: &str,
    ) -> async_graphql::Result<()> {
        tx.delete_where::<VideoStream>(VideoStreamWhereInput {
            media_file_id: Some(Self::string_eq(id)),
            ..Default::default()
        })
        .await
        .map_err(Self::db_error)?;
        tx.delete_where::<AudioStream>(AudioStreamWhereInput {
            media_file_id: Some(Self::string_eq(id)),
            ..Default::default()
        })
        .await
        .map_err(Self::db_error)?;
        tx.delete_where::<Subtitle>(SubtitleWhereInput {
            media_file_id: Some(Self::string_eq(id)),
            ..Default::default()
        })
        .await
        .map_err(Self::db_error)?;
        tx.delete_where::<MediaChapter>(MediaChapterWhereInput {
            media_file_id: Some(Self::string_eq(id)),
            ..Default::default()
        })
        .await
        .map_err(Self::db_error)?;
        tx.delete_where::<PlaybackProgress>(PlaybackProgressWhereInput {
            media_file_id: Some(Self::string_eq(id)),
            ..Default::default()
        })
        .await
        .map_err(Self::db_error)?;
        // Notifications and torrent file rows keep their history and lose only the dead id.
        tx.update_where::<Notification>(
            NotificationWhereInput {
                media_file_id: Some(Self::string_eq(id)),
                ..Default::default()
            },
            UpdateNotificationInput {
                media_file_id: Some(None),
                ..Default::default()
            },
        )
        .await
        .map_err(Self::db_error)?;
        tx.update_where::<TorrentFile>(
            TorrentFileWhereInput {
                media_file_id: Some(Self::string_eq(id)),
                ..Default::default()
            },
            UpdateTorrentFileInput {
                media_file_id: Some(None),
                ..Default::default()
            },
        )
        .await
        .map_err(Self::db_error)?;
        // A duplicate issue is about this copy, so removing it leaves nothing to settle.
        tx.update_where::<LibraryScanIssue>(
            LibraryScanIssueWhereInput {
                media_file_id: Some(Self::string_eq(id)),
                issue_code: Some(Self::string_eq("BYTE_IDENTICAL_DUPLICATE")),
                resolved_at: Some(graphql_orm::graphql::filters::DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
            UpdateLibraryScanIssueInput {
                resolved_at: Some(Some(Self::now_rfc3339())),
                resolution: Some(Some(format!(
                    "{}: this copy was removed from the library.",
                    crate::services::library_scan::NO_LONGER_DUPLICATE
                ))),
                ..Default::default()
            },
        )
        .await
        .map_err(Self::db_error)?;
        // Retain scan history while its Library/ScanRun exists, with a nullable file link.
        tx.update_where::<LibraryScanIssue>(
            LibraryScanIssueWhereInput {
                media_file_id: Some(Self::string_eq(id)),
                ..Default::default()
            },
            UpdateLibraryScanIssueInput {
                media_file_id: Some(None),
                ..Default::default()
            },
        )
        .await
        .map_err(Self::db_error)?;
        // Sessions retain their owner and content history; they cannot keep playing a dead file.
        tx.update_where::<PlaybackSession>(
            PlaybackSessionWhereInput {
                media_file_id: Some(Self::string_eq(id)),
                ..Default::default()
            },
            UpdatePlaybackSessionInput {
                media_file_id: Some(None),
                is_playing: Some(false),
                ..Default::default()
            },
        )
        .await
        .map_err(Self::db_error)?;
        tx.update_where::<CastSession>(
            CastSessionWhereInput {
                media_file_id: Some(Self::string_eq(id)),
                ..Default::default()
            },
            UpdateCastSessionInput {
                media_file_id: Some(None),
                player_state: Some("IDLE".to_string()),
                ended_at: Some(Some(Self::now_rfc3339())),
                stream_url: Some(String::new()),
                ..Default::default()
            },
        )
        .await
        .map_err(Self::db_error)?;
        Ok(())
    }

    async fn invalidate_parent(
        tx: &mut MutationContext<'_>,
        parent: ParentInvalidation,
    ) -> async_graphql::Result<()> {
        let ParentInvalidation(kind, id) = parent;
        match kind {
            "Artist" => {
                tx.update_by_id::<Artist>(&id, UpdateArtistInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            "Collection" => {
                tx.update_by_id::<Collection>(&id, UpdateCollectionInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            "Album" => {
                reconcile_album_summary(tx, &id, false, true)
                    .await
                    .map_err(Self::db_error)?;
            }
            "Audiobook" => {
                reconcile_audiobook_summary(tx, &id, false, true)
                    .await
                    .map_err(Self::db_error)?;
            }
            "Show" => {
                tx.update_by_id::<Show>(&id, UpdateShowInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            "Library" => {
                tx.update_by_id::<Library>(&id, UpdateLibraryInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            "Movie" => {
                tx.update_by_id::<Movie>(&id, UpdateMovieInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            "Episode" => {
                tx.update_by_id::<Episode>(&id, UpdateEpisodeInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            "Track" => {
                tx.update_by_id::<Track>(&id, UpdateTrackInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            "Chapter" => {
                tx.update_by_id::<Chapter>(&id, UpdateChapterInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            "Torrent" => {
                tx.update_by_id::<Torrent>(&id, UpdateTorrentInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            "RssFeed" => {
                tx.update_by_id::<RssFeed>(&id, UpdateRssFeedInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            "LibraryScanRun" => {
                tx.update_by_id::<LibraryScanRun>(&id, UpdateLibraryScanRunInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            "MediaFile" => {
                tx.update_by_id::<MediaFile>(&id, UpdateMediaFileInput::default())
                    .await
                    .map_err(Self::db_error)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Local sets deduplicate old/new identities. The pinned context exposes no
    /// transaction-local extensions or before-commit callbacks, so cross-event coalescing
    /// cannot safely use a process-global cache or post-commit writes. Real summaries skip
    /// unchanged values; generated timestamp-only updates cover provenance gaps.
    async fn maintain_summaries_and_parent_events(
        tx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        if event.phase != MutationPhase::After {
            return Ok(());
        }
        let mut parents = BTreeSet::new();
        let mut summaries = BTreeSet::new();
        macro_rules! edge {
            ($child:ty, $field:ident, $parent:literal) => {{
                let before = event.before::<$child>()?.map(|row| row.$field.as_str());
                let after = event.after::<$child>()?.map(|row| row.$field.as_str());
                if event.action == ChangeAction::Deleted || (before.is_some() && before != after) {
                    for id in [before, after].into_iter().flatten() {
                        parents.insert(ParentInvalidation($parent, id.to_string()));
                    }
                }
            }};
        }
        macro_rules! optional_edge {
            ($child:ty, $field:ident, $parent:literal) => {{
                let before = event
                    .before::<$child>()?
                    .and_then(|row| row.$field.as_deref());
                let after = event
                    .after::<$child>()?
                    .and_then(|row| row.$field.as_deref());
                if event.action == ChangeAction::Deleted
                    || (event.action == ChangeAction::Updated && before != after)
                {
                    for id in [before, after].into_iter().flatten() {
                        parents.insert(ParentInvalidation($parent, id.to_string()));
                    }
                }
            }};
        }
        match event.entity_name {
            "Track" => {
                edge!(Track, album_id, "Album");
                edge!(Track, library_id, "Library");
                // Creates and updates reach the credited artist through `Track.artist`
                // propagation; only a delete or a re-credit needs a direct write.
                optional_edge!(Track, artist_id, "Artist");
                let before = event.before::<Track>()?;
                let after = event.after::<Track>()?;
                let artist_summary_changed = before
                    .map(|row| (&row.artist_id, &row.library_id, row.duration_secs))
                    != after.map(|row| (&row.artist_id, &row.library_id, row.duration_secs));
                for row in [before, after].into_iter().flatten() {
                    summaries.insert(ParentInvalidation("Album", row.album_id.clone()));
                    if artist_summary_changed && let Some(id) = &row.artist_id {
                        summaries.insert(ParentInvalidation("Artist", id.clone()));
                    }
                }
            }
            "Chapter" => {
                edge!(Chapter, audiobook_id, "Audiobook");
                for row in [event.before::<Chapter>()?, event.after::<Chapter>()?]
                    .into_iter()
                    .flatten()
                {
                    summaries.insert(ParentInvalidation("Audiobook", row.audiobook_id.clone()));
                }
            }
            "Episode" => edge!(Episode, show_id, "Show"),
            "Season" => edge!(Season, show_id, "Show"),
            "Show" => edge!(Show, library_id, "Library"),
            "Movie" => {
                edge!(Movie, library_id, "Library");
                // Creates, updates and the new membership reach the collection through
                // `Movie.collection` propagation. A delete or a move leaves the old
                // collection without a surviving source, so it is written directly.
                let before = event.before::<Movie>()?;
                let after = event.after::<Movie>()?;
                let left = before
                    .filter(|old| {
                        after.is_none_or(|new| {
                            (&new.library_id, new.collection_id)
                                != (&old.library_id, old.collection_id)
                        })
                    })
                    .and_then(|old| old.collection_id.map(|key| (old.library_id.clone(), key)));
                if let Some((library_id, key)) = left {
                    for collection in transaction_rows::<Collection, _>(
                        tx,
                        CollectionWhereInput {
                            library_id: Some(Self::string_eq(library_id)),
                            tmdb_collection_id: Some(graphql_orm::graphql::filters::IntFilter {
                                eq: Some(key),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(Self::db_error)?
                    {
                        parents.insert(ParentInvalidation("Collection", collection.id));
                    }
                }
            }
            "Artist" => {
                edge!(Artist, library_id, "Library");
                // Recompute on insertion/library moves only. Summary timestamp writes
                // reenter this hook but cannot schedule another Artist write.
                let before = event.before::<Artist>()?;
                if let Some(after) = event.after::<Artist>()?
                    && before.is_none_or(|row| row.library_id != after.library_id)
                {
                    reconcile_artist_summary(tx, &after.id, false, false)
                        .await
                        .map_err(Self::db_error)?;
                }
            }
            "Collection" => edge!(Collection, library_id, "Library"),
            // Torrent, RssFeed, PendingFileMatch and MediaFile reach their titles and
            // libraries through `propagate_change = "up"` on create and update, so the
            // 10-second torrent sync writes no parent rows. Propagation reads the
            // post-update row only: a delete or a move still writes the surviving
            // (old and new) parents here.
            "RssFeed" => optional_edge!(RssFeed, library_id, "Library"),
            "Torrent" => {
                optional_edge!(Torrent, library_id, "Library");
                optional_edge!(Torrent, movie_id, "Movie");
                optional_edge!(Torrent, episode_id, "Episode");
                optional_edge!(Torrent, track_id, "Track");
                optional_edge!(Torrent, chapter_id, "Chapter");
                optional_edge!(Torrent, show_id, "Show");
                optional_edge!(Torrent, album_id, "Album");
                optional_edge!(Torrent, audiobook_id, "Audiobook");
            }
            "PendingFileMatch" => {
                optional_edge!(PendingFileMatch, movie_id, "Movie");
                optional_edge!(PendingFileMatch, episode_id, "Episode");
                optional_edge!(PendingFileMatch, track_id, "Track");
                optional_edge!(PendingFileMatch, chapter_id, "Chapter");
                // Kept by hand: the source is a (info hash, owner) pair behind a
                // `source_condition`, which propagation ignores.
                let sources = [
                    event.before::<PendingFileMatch>()?,
                    event.after::<PendingFileMatch>()?,
                ]
                .into_iter()
                .flatten()
                .filter(|row| row.source_type == "torrent")
                .filter_map(|row| {
                    row.source_id
                        .as_ref()
                        .map(|hash| (row.user_id.clone(), hash.clone()))
                })
                .collect::<BTreeSet<_>>();
                for (owner, hash) in sources {
                    // The import pipeline stores an info hash, never Torrent.id.
                    // Provenance is qualified by the pending row's owner even for
                    // trusted/admin writes; a foreign UUID/hash grants no authority.
                    let mut torrents = transaction_rows::<Torrent, _>(
                        tx,
                        TorrentWhereInput {
                            info_hash: Some(Self::string_eq(hash)),
                            user_id: Some(Self::string_eq(owner)),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(Self::db_error)?;
                    if torrents.len() > 1 {
                        return Err(async_graphql::Error::new(
                            "Ambiguous owned torrent source for pending file match",
                        ));
                    }
                    if let Some(torrent) = torrents.pop() {
                        parents.insert(ParentInvalidation("Torrent", torrent.id));
                    }
                }
            }
            "Album" => {
                edge!(Album, library_id, "Library");
                // Creates and updates reach the artist through `Album.artist` propagation.
                edge!(Album, artist_id, "Artist");
                let before = event.before::<Album>()?;
                let after = event.after::<Album>()?;
                let artist_summary_changed = before.map(|row| (&row.artist_id, &row.library_id))
                    != after.map(|row| (&row.artist_id, &row.library_id));
                for row in [before, after].into_iter().flatten() {
                    if artist_summary_changed {
                        summaries.insert(ParentInvalidation("Artist", row.artist_id.clone()));
                    }
                }
            }
            "Audiobook" => edge!(Audiobook, library_id, "Library"),
            "MovieCastCredit" => edge!(MovieCastCredit, movie_id, "Movie"),
            "TorrentFile" => edge!(TorrentFile, torrent_id, "Torrent"),
            "RssFeedItem" => edge!(RssFeedItem, feed_id, "RssFeed"),
            "LibraryScanIssue" => edge!(LibraryScanIssue, scan_run_id, "LibraryScanRun"),
            "LibraryScanRun" => edge!(LibraryScanRun, library_id, "Library"),
            "MediaFile" => {
                optional_edge!(MediaFile, library_id, "Library");
                // File analysis, quality and size changes reach the linked title through
                // `MediaFile.movie/episode/track/chapter` propagation. A delete or a
                // re-link has no surviving source for the title it left, so write it.
                optional_edge!(MediaFile, movie_id, "Movie");
                optional_edge!(MediaFile, episode_id, "Episode");
                optional_edge!(MediaFile, track_id, "Track");
                optional_edge!(MediaFile, chapter_id, "Chapter");
                // Child-side links are canonical, including historical one-sided rows.
                for track in transaction_rows::<Track, _>(
                    tx,
                    TrackWhereInput {
                        media_file_id: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?
                {
                    summaries.insert(ParentInvalidation("Album", track.album_id));
                }
                for chapter in transaction_rows::<Chapter, _>(
                    tx,
                    ChapterWhereInput {
                        media_file_id: Some(Self::string_eq(&event.id)),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?
                {
                    summaries.insert(ParentInvalidation("Audiobook", chapter.audiobook_id));
                }
            }
            "VideoStream" => edge!(VideoStream, media_file_id, "MediaFile"),
            "AudioStream" => edge!(AudioStream, media_file_id, "MediaFile"),
            "Subtitle" => edge!(Subtitle, media_file_id, "MediaFile"),
            "MediaChapter" => edge!(MediaChapter, media_file_id, "MediaFile"),
            _ => {}
        }
        for parent in summaries {
            if parent.0 == "Artist" {
                let invalidate = parents.remove(&parent);
                reconcile_artist_summary(tx, &parent.1, false, invalidate)
                    .await
                    .map_err(Self::db_error)?;
                continue;
            }
            if parents.remove(&parent) {
                Self::invalidate_parent(tx, parent).await?;
            } else {
                match parent.0 {
                    "Album" => {
                        reconcile_album_summary(tx, &parent.1, false, false)
                            .await
                            .map_err(Self::db_error)?;
                    }
                    "Audiobook" => {
                        reconcile_audiobook_summary(tx, &parent.1, false, false)
                            .await
                            .map_err(Self::db_error)?;
                    }
                    _ => {}
                }
            }
        }
        for parent in parents {
            Self::invalidate_parent(tx, parent).await?;
        }
        Ok(())
    }

    fn source_change_requires_reload(event: &MutationEvent) -> bool {
        if event.entity_name != "Source" || event.phase != MutationPhase::After {
            return false;
        }

        match event.action {
            ChangeAction::Created | ChangeAction::Deleted => true,
            ChangeAction::Updated => event.changes.iter().any(|change| {
                matches!(
                    change.field.as_str(),
                    "name"
                        | "Name"
                        | "source_type"
                        | "SourceType"
                        | "sourceType"
                        | "definition_id"
                        | "DefinitionId"
                        | "definitionId"
                        | "enabled"
                        | "Enabled"
                        | "priority"
                        | "Priority"
                        | "media_types"
                        | "MediaTypes"
                        | "mediaTypes"
                        | "site_url"
                        | "SiteUrl"
                        | "siteUrl"
                        | "supports_search"
                        | "SupportsSearch"
                        | "supportsSearch"
                        | "supports_tv_search"
                        | "SupportsTvSearch"
                        | "supportsTvSearch"
                        | "supports_movie_search"
                        | "SupportsMovieSearch"
                        | "supportsMovieSearch"
                        | "supports_music_search"
                        | "SupportsMusicSearch"
                        | "supportsMusicSearch"
                        | "supports_book_search"
                        | "SupportsBookSearch"
                        | "supportsBookSearch"
                        | "credentials"
                        | "Credentials"
                        | "settings"
                        | "Settings"
                )
            }),
        }
    }

    fn defer_source_reload(
        &self,
        ctx: Option<&async_graphql::Context<'_>>,
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) {
        let Some(services) = self.services_from_ctx(ctx) else {
            warn!(
                source_id = %event.id,
                action = ?event.action,
                "Services manager unavailable after source configuration mutation; source reload skipped"
            );
            return;
        };
        let source_id = event.id.clone();
        let action = format!("{:?}", event.action);

        hook_ctx.defer(move |_db| async move {
            if let Some(sources_svc) = services.get_sources().await {
                sources_svc.reload().await?;
                tracing::info!(
                    source_id = %source_id,
                    action = %action,
                    "Reloaded sources after source configuration mutation"
                );
            } else {
                tracing::warn!(
                    source_id = %source_id,
                    action = %action,
                    "Sources service unavailable after source configuration mutation; source reload skipped"
                );
            }

            Ok::<(), anyhow::Error>(())
        });
    }

    async fn revoke_user_sessions_if_needed(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = Self::before::<User>(event, "User")?;
        let after = Self::after::<User>(event, "User")?;
        let should_revoke = (before.is_active && !after.is_active)
            || before.role != after.role
            || before.password_hash != after.password_hash;
        if !should_revoke {
            return Ok(());
        }

        let revoked_at = Self::now_rfc3339();
        let tokens = transaction_rows::<RefreshToken, _>(
            hook_ctx,
            RefreshTokenWhereInput {
                user_id: Some(Self::string_eq(&after.id)),
                revoked_at: Some(graphql_orm::graphql::filters::DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                ..RefreshTokenWhereInput::default()
            },
        )
        .await
        .map_err(Self::db_error)?;
        for token in tokens {
            if token.revoked_at.is_some() {
                continue;
            }
            hook_ctx
                .update_by_id::<RefreshToken>(
                    &token.id,
                    UpdateRefreshTokenInput {
                        revoked_at: Some(Some(revoked_at.clone())),
                        revocation_reason: Some(Some("AdminRevoked".to_string())),
                        ..UpdateRefreshTokenInput::default()
                    },
                )
                .await
                .map_err(Self::db_error)?;
        }

        Ok(())
    }

    async fn set_movie_media_file(
        hook_ctx: &mut MutationContext<'_>,
        movie_id: &str,
        media_file_id: Option<&str>,
    ) -> async_graphql::Result<()> {
        let Some(movie) = hook_ctx
            .find_by_id::<Movie>(&movie_id.to_string())
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(());
        };
        let wanted = media_file_id.is_none();
        let has_file = media_file_id.is_some();
        if movie.media_file_id.as_deref() == media_file_id
            && movie.wanted == wanted
            && movie.has_file == has_file
        {
            return Ok(());
        }
        hook_ctx
            .update_by_id::<Movie>(
                &movie.id,
                UpdateMovieInput {
                    media_file_id: Some(media_file_id.map(ToOwned::to_owned)),
                    wanted: Some(wanted),
                    has_file: Some(has_file),
                    ..UpdateMovieInput::default()
                },
            )
            .await
            .map_err(Self::db_error)?;
        Ok(())
    }

    async fn set_episode_media_file(
        hook_ctx: &mut MutationContext<'_>,
        episode_id: &str,
        media_file_id: Option<&str>,
    ) -> async_graphql::Result<()> {
        let Some(episode) = hook_ctx
            .find_by_id::<Episode>(&episode_id.to_string())
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(());
        };
        let wanted = media_file_id.is_none();
        if episode.media_file_id.as_deref() == media_file_id && episode.wanted == wanted {
            return Ok(());
        }
        hook_ctx
            .update_by_id::<Episode>(
                &episode.id,
                UpdateEpisodeInput {
                    media_file_id: Some(media_file_id.map(ToOwned::to_owned)),
                    wanted: Some(wanted),
                    ..UpdateEpisodeInput::default()
                },
            )
            .await
            .map_err(Self::db_error)?;
        Ok(())
    }

    async fn set_track_media_file(
        hook_ctx: &mut MutationContext<'_>,
        track_id: &str,
        media_file_id: Option<&str>,
    ) -> async_graphql::Result<()> {
        let Some(track) = hook_ctx
            .find_by_id::<Track>(&track_id.to_string())
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(());
        };
        let wanted = media_file_id.is_none();
        if track.media_file_id.as_deref() == media_file_id && track.wanted == wanted {
            return Ok(());
        }
        hook_ctx
            .update_by_id::<Track>(
                &track.id,
                UpdateTrackInput {
                    media_file_id: Some(media_file_id.map(ToOwned::to_owned)),
                    wanted: Some(wanted),
                    ..UpdateTrackInput::default()
                },
            )
            .await
            .map_err(Self::db_error)?;
        Ok(())
    }

    async fn set_chapter_media_file(
        hook_ctx: &mut MutationContext<'_>,
        chapter_id: &str,
        media_file_id: Option<&str>,
    ) -> async_graphql::Result<()> {
        let Some(chapter) = hook_ctx
            .find_by_id::<Chapter>(&chapter_id.to_string())
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(());
        };
        let wanted = media_file_id.is_none();
        if chapter.media_file_id.as_deref() == media_file_id && chapter.wanted == wanted {
            return Ok(());
        }
        hook_ctx
            .update_by_id::<Chapter>(
                &chapter.id,
                UpdateChapterInput {
                    media_file_id: Some(media_file_id.map(ToOwned::to_owned)),
                    wanted: Some(wanted),
                    ..UpdateChapterInput::default()
                },
            )
            .await
            .map_err(Self::db_error)?;
        Ok(())
    }

    async fn set_media_file_target(
        hook_ctx: &mut MutationContext<'_>,
        media_file_id: &str,
        movie_id: Option<&str>,
        episode_id: Option<&str>,
        track_id: Option<&str>,
        chapter_id: Option<&str>,
    ) -> async_graphql::Result<()> {
        let Some(media_file) = hook_ctx
            .find_by_id::<MediaFile>(&media_file_id.to_string())
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(());
        };
        if media_file.movie_id.as_deref() == movie_id
            && media_file.episode_id.as_deref() == episode_id
            && media_file.track_id.as_deref() == track_id
            && media_file.chapter_id.as_deref() == chapter_id
        {
            return Ok(());
        }
        hook_ctx
            .update_by_id::<MediaFile>(
                &media_file.id,
                UpdateMediaFileInput {
                    movie_id: Some(movie_id.map(ToOwned::to_owned)),
                    episode_id: Some(episode_id.map(ToOwned::to_owned)),
                    track_id: Some(track_id.map(ToOwned::to_owned)),
                    chapter_id: Some(chapter_id.map(ToOwned::to_owned)),
                    ..UpdateMediaFileInput::default()
                },
            )
            .await
            .map_err(Self::db_error)?;
        Ok(())
    }

    /// Detach only the inverse that still points at this target. A nested rematch may
    /// already have installed another target; stale outer preimages must not erase it.
    async fn clear_media_file_target_if_matches(
        tx: &mut MutationContext<'_>,
        file_id: &str,
        kind: &str,
        target_id: &str,
    ) -> async_graphql::Result<()> {
        let Some(file) = tx
            .find_by_id::<MediaFile>(&file_id.to_string())
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(());
        };
        let matches = match kind {
            "Movie" => file.movie_id.as_deref() == Some(target_id),
            "Episode" => file.episode_id.as_deref() == Some(target_id),
            "Track" => file.track_id.as_deref() == Some(target_id),
            "Chapter" => file.chapter_id.as_deref() == Some(target_id),
            _ => false,
        };
        if matches {
            Self::set_media_file_target(tx, file_id, None, None, None, None).await?;
        }
        Ok(())
    }

    async fn sync_media_file_link_update(
        tx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = event.before::<MediaFile>()?;
        let Some(after) = tx
            .find_by_id::<MediaFile>(&event.id)
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(());
        };
        let active_movie = after.movie_id.as_deref();
        let active_episode = after
            .episode_id
            .as_deref()
            .filter(|_| active_movie.is_none());
        let active_track = after
            .track_id
            .as_deref()
            .filter(|_| active_movie.is_none() && active_episode.is_none());
        let active_chapter = after.chapter_id.as_deref().filter(|_| {
            active_movie.is_none() && active_episode.is_none() && active_track.is_none()
        });
        Self::set_media_file_target(
            tx,
            &after.id,
            active_movie,
            active_episode,
            active_track,
            active_chapter,
        )
        .await?;
        let old = before.and_then(|row| row.movie_id.as_deref());
        if old != active_movie
            && let Some(id) = old
            && let Some(row) = tx
                .find_by_id::<Movie>(&id.to_string())
                .await
                .map_err(Self::db_error)?
            && row.media_file_id.as_deref() == Some(after.id.as_str())
        {
            Self::set_movie_media_file(tx, id, None).await?;
        }
        if let Some(id) = active_movie {
            Self::set_movie_media_file(tx, id, Some(&after.id)).await?;
        }
        let old = before.and_then(|row| row.episode_id.as_deref());
        if old != active_episode
            && let Some(id) = old
            && let Some(row) = tx
                .find_by_id::<Episode>(&id.to_string())
                .await
                .map_err(Self::db_error)?
            && row.media_file_id.as_deref() == Some(after.id.as_str())
        {
            Self::set_episode_media_file(tx, id, None).await?;
        }
        if let Some(id) = active_episode {
            Self::set_episode_media_file(tx, id, Some(&after.id)).await?;
        }
        let old = before.and_then(|row| row.track_id.as_deref());
        if old != active_track
            && let Some(id) = old
            && let Some(row) = tx
                .find_by_id::<Track>(&id.to_string())
                .await
                .map_err(Self::db_error)?
            && row.media_file_id.as_deref() == Some(after.id.as_str())
        {
            Self::set_track_media_file(tx, id, None).await?;
        }
        if let Some(id) = active_track {
            Self::set_track_media_file(tx, id, Some(&after.id)).await?;
        }
        let old = before.and_then(|row| row.chapter_id.as_deref());
        if old != active_chapter
            && let Some(id) = old
            && let Some(row) = tx
                .find_by_id::<Chapter>(&id.to_string())
                .await
                .map_err(Self::db_error)?
            && row.media_file_id.as_deref() == Some(after.id.as_str())
        {
            Self::set_chapter_media_file(tx, id, None).await?;
        }
        // A file that held a whole book leaves every chapter it was shared with.
        if old.is_some() && old != active_chapter {
            // The chapter it is moving to keeps it: clearing that too would undo the move and start
            // these hooks over.
            tx.update_where::<Chapter>(
                ChapterWhereInput {
                    media_file_id: Some(Self::string_eq(&after.id)),
                    id: active_chapter.map(|id| graphql_orm::graphql::filters::StringFilter {
                        ne: Some(id.to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                UpdateChapterInput {
                    media_file_id: Some(None),
                    wanted: Some(true),
                    ..Default::default()
                },
            )
            .await
            .map_err(Self::db_error)?;
        }
        if let Some(id) = active_chapter {
            Self::set_chapter_media_file(tx, id, Some(&after.id)).await?;
        }
        Ok(())
    }

    async fn clear_deleted_media_file_links(
        tx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        // Read the child-side links too: historical one-sided links must not survive deletion.
        tx.update_where::<Movie>(
            MovieWhereInput {
                media_file_id: Some(Self::string_eq(&event.id)),
                ..Default::default()
            },
            UpdateMovieInput {
                media_file_id: Some(None),
                wanted: Some(true),
                has_file: Some(false),
                ..Default::default()
            },
        )
        .await
        .map_err(Self::db_error)?;
        tx.update_where::<Episode>(
            EpisodeWhereInput {
                media_file_id: Some(Self::string_eq(&event.id)),
                ..Default::default()
            },
            UpdateEpisodeInput {
                media_file_id: Some(None),
                wanted: Some(true),
                ..Default::default()
            },
        )
        .await
        .map_err(Self::db_error)?;
        tx.update_where::<Track>(
            TrackWhereInput {
                media_file_id: Some(Self::string_eq(&event.id)),
                ..Default::default()
            },
            UpdateTrackInput {
                media_file_id: Some(None),
                wanted: Some(true),
                ..Default::default()
            },
        )
        .await
        .map_err(Self::db_error)?;
        tx.update_where::<Chapter>(
            ChapterWhereInput {
                media_file_id: Some(Self::string_eq(&event.id)),
                ..Default::default()
            },
            UpdateChapterInput {
                media_file_id: Some(None),
                wanted: Some(true),
                ..Default::default()
            },
        )
        .await
        .map_err(Self::db_error)?;
        Self::cleanup_media_file_dependents(tx, &event.id).await?;
        Ok(())
    }

    async fn sync_movie_media_file_update(
        tx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = event.before::<Movie>()?;
        let Some(after) = tx
            .find_by_id::<Movie>(&event.id)
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(());
        };
        let old = before.and_then(|row| row.media_file_id.as_deref());
        if old == after.media_file_id.as_deref() {
            return Ok(());
        }
        if let Some(file_id) = old {
            Self::clear_media_file_target_if_matches(tx, file_id, "Movie", &after.id).await?;
        }
        if let Some(file_id) = after.media_file_id.as_deref() {
            Self::set_media_file_target(tx, file_id, Some(&after.id), None, None, None).await?;
        }
        Self::set_movie_media_file(tx, &after.id, after.media_file_id.as_deref()).await
    }

    async fn sync_episode_media_file_update(
        tx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = event.before::<Episode>()?;
        let Some(after) = tx
            .find_by_id::<Episode>(&event.id)
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(());
        };
        let old = before.and_then(|row| row.media_file_id.as_deref());
        if old == after.media_file_id.as_deref() {
            return Ok(());
        }
        if let Some(file_id) = old {
            Self::clear_media_file_target_if_matches(tx, file_id, "Episode", &after.id).await?;
        }
        if let Some(file_id) = after.media_file_id.as_deref() {
            Self::set_media_file_target(tx, file_id, None, Some(&after.id), None, None).await?;
        }
        Self::set_episode_media_file(tx, &after.id, after.media_file_id.as_deref()).await
    }

    async fn sync_track_media_file_update(
        tx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = event.before::<Track>()?;
        let Some(after) = tx
            .find_by_id::<Track>(&event.id)
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(());
        };
        let old = before.and_then(|row| row.media_file_id.as_deref());
        if old == after.media_file_id.as_deref() {
            return Ok(());
        }
        if let Some(file_id) = old {
            Self::clear_media_file_target_if_matches(tx, file_id, "Track", &after.id).await?;
        }
        if let Some(file_id) = after.media_file_id.as_deref() {
            Self::set_media_file_target(tx, file_id, None, None, Some(&after.id), None).await?;
        }
        Self::set_track_media_file(tx, &after.id, after.media_file_id.as_deref()).await
    }

    async fn sync_chapter_media_file_update(
        tx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = event.before::<Chapter>()?;
        let Some(after) = tx
            .find_by_id::<Chapter>(&event.id)
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(());
        };
        let old = before.and_then(|row| row.media_file_id.as_deref());
        if old == after.media_file_id.as_deref() {
            return Ok(());
        }
        // A book in parts has several files naming chapters they don't start: moving a chapter
        // between two files of its own book takes neither file out of the book.
        let same_book = match after.media_file_id.as_deref() {
            Some(file_id) => Self::file_belongs_to_chapters_book(tx, file_id, &after).await?,
            None => false,
        };
        if let Some(file_id) = old
            && !same_book
        {
            Self::clear_media_file_target_if_matches(tx, file_id, "Chapter", &after.id).await?;
        }
        if let Some(file_id) = after.media_file_id.as_deref()
            && !same_book
        {
            Self::set_media_file_target(tx, file_id, None, None, None, Some(&after.id)).await?;
        }
        Self::set_chapter_media_file(tx, &after.id, after.media_file_id.as_deref()).await
    }

    /// One file can hold a whole book, so every chapter of that book points at it while the file
    /// names just one of them. A chapter joining them must not take the file from the others.
    async fn file_belongs_to_chapters_book(
        tx: &mut MutationContext<'_>,
        file_id: &str,
        chapter: &Chapter,
    ) -> async_graphql::Result<bool> {
        let Some(named) = tx
            .find_by_id::<MediaFile>(&file_id.to_string())
            .await
            .map_err(Self::db_error)?
            .and_then(|file| file.chapter_id)
        else {
            return Ok(false);
        };
        if named == chapter.id {
            return Ok(true);
        }
        let Some(other) = tx
            .find_by_id::<Chapter>(&named)
            .await
            .map_err(Self::db_error)?
        else {
            return Ok(false);
        };
        // Only the book matters. In a book cut into parts, the chapter a file names can already
        // point at a later part, and insisting it point back made these hooks chase each other.
        Ok(other.audiobook_id == chapter.audiobook_id)
    }

    fn defer_storage_object_delete(
        &self,
        ctx: Option<&async_graphql::Context<'_>>,
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let object = Self::before::<StorageObject>(event, "StorageObject")?.clone();
        let Some(services) = self.services_from_ctx(ctx) else {
            warn!(
                storage_object_id = %object.id,
                object_id = %object.object_id,
                "Services manager unavailable after storage object deletion; object bytes cleanup skipped"
            );
            return Ok(());
        };

        hook_ctx.defer(move |_db| async move {
            let Some(storage_svc) = services.get_storage().await else {
                warn!(
                    storage_object_id = %object.id,
                    object_id = %object.object_id,
                    "Object storage service unavailable after storage object deletion; object bytes cleanup skipped"
                );
                return Ok::<(), anyhow::Error>(());
            };
            let stored = match storage_svc.stored_object_from_entity(&object) {
                Ok(stored) => stored,
                Err(error) => {
                    warn!(
                        storage_object_id = %object.id,
                        object_id = %object.object_id,
                        error = %error,
                        "Failed to rebuild stored object metadata after deletion; object bytes cleanup skipped"
                    );
                    return Ok::<(), anyhow::Error>(());
                }
            };
            storage_svc.storage().delete_object(&stored).await?;
            Ok::<(), anyhow::Error>(())
        });

        Ok(())
    }

    /// Delete a storage object once no artwork row references it; the StorageObject
    /// delete hook removes its bytes after commit.
    async fn release_unreferenced_storage_object(
        hook_ctx: &mut MutationContext<'_>,
        storage_object_id: &str,
    ) -> async_graphql::Result<()> {
        if storage_object_id.is_empty() {
            return Ok(());
        }
        let remaining = hook_ctx
            .query::<ArtworkCache>()
            .filter(ArtworkCacheWhereInput {
                storage_object_id: Some(Self::string_eq(storage_object_id)),
                ..ArtworkCacheWhereInput::default()
            })
            .limit(1)
            .fetch_all()
            .await
            .map_err(Self::db_error)?;
        if remaining.is_empty() {
            hook_ctx
                .delete_by_id::<StorageObject>(&storage_object_id.to_string())
                .await
                .map_err(Self::db_error)?;
        }
        Ok(())
    }

    async fn cleanup_deleted_artwork_cache(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let cache = Self::before::<ArtworkCache>(event, "ArtworkCache")?;
        Self::release_unreferenced_storage_object(hook_ctx, &cache.storage_object_id).await
    }

    /// Re-pointing artwork at a new storage object (the provider's image changed)
    /// releases the old object like a delete does, and drops the resized copies made
    /// from it. A resized copy that is itself re-pointed has none of its own.
    async fn cleanup_replaced_artwork_cache(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = Self::before::<ArtworkCache>(event, "ArtworkCache")?;
        let after = Self::after::<ArtworkCache>(event, "ArtworkCache")?;
        if before.storage_object_id == after.storage_object_id {
            return Ok(());
        }
        Self::release_unreferenced_storage_object(hook_ctx, &before.storage_object_id).await?;
        if !after.artwork_type.contains(ARTWORK_VARIANT_MARKER) {
            hook_ctx
                .delete_where::<ArtworkCache>(ArtworkCacheWhereInput {
                    entity_type: Some(Self::string_eq(&after.entity_type)),
                    entity_id: Some(Self::string_eq(&after.entity_id)),
                    artwork_type: Some(StringFilter {
                        starts_with: Some(format!(
                            "{}{ARTWORK_VARIANT_MARKER}",
                            after.artwork_type
                        )),
                        ..StringFilter::default()
                    }),
                    ..ArtworkCacheWhereInput::default()
                })
                .await
                .map_err(Self::db_error)?;
        }
        Ok(())
    }

    /// Cached artwork appears after its title is saved, so the title row does not
    /// change by itself. A generated empty update republishes it, in the same
    /// transaction as the cache write, so open library views refresh when the image
    /// becomes available (this replaces the provider service's hand-made "touch").
    /// Resized copies do not count; only the original changes what a view shows.
    async fn publish_artwork_owner(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let cache = Self::after::<ArtworkCache>(event, "ArtworkCache")?;
        if cache.artwork_type.contains(ARTWORK_VARIANT_MARKER) {
            return Ok(());
        }
        let id = cache.entity_id.clone();
        macro_rules! touch {
            ($ty:ty, $input:ty) => {
                if hook_ctx
                    .find_by_id::<$ty>(&id)
                    .await
                    .map_err(Self::db_error)?
                    .is_some()
                {
                    hook_ctx
                        .update_by_id::<$ty>(&id, <$input>::default())
                        .await
                        .map_err(Self::db_error)?;
                }
            };
        }
        match cache.entity_type.as_str() {
            "movie" => touch!(Movie, UpdateMovieInput),
            "show" => touch!(Show, UpdateShowInput),
            "album" => touch!(Album, UpdateAlbumInput),
            "audiobook" => touch!(Audiobook, UpdateAudiobookInput),
            _ => {}
        }
        Ok(())
    }

    /// Remove an entity's cached artwork (originals and resized copies) with the entity, in
    /// the same transaction. The ArtworkCache hook releases unreferenced storage objects,
    /// whose bytes the StorageObject hook deletes after commit.
    async fn cleanup_entity_artwork(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let (entity_type, entity_id) = match event.entity_name {
            // Collection artwork is shared by TMDB collection id across libraries: it goes
            // with the last library collection that uses it.
            "Collection" => {
                let collection = Self::before::<Collection>(event, "Collection")?;
                let others = hook_ctx
                    .query::<Collection>()
                    .filter(CollectionWhereInput {
                        tmdb_collection_id: Some(graphql_orm::graphql::filters::IntFilter {
                            eq: Some(collection.tmdb_collection_id),
                            ..Default::default()
                        }),
                        id: Some(graphql_orm::graphql::filters::StringFilter {
                            ne: Some(collection.id.clone()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .limit(1)
                    .fetch_all()
                    .await
                    .map_err(Self::db_error)?;
                if !others.is_empty() {
                    return Ok(());
                }
                ("collection", collection.tmdb_collection_id.to_string())
            }
            name => (Self::artwork_entity_type(name), event.id.clone()),
        };
        hook_ctx
            .delete_where::<ArtworkCache>(ArtworkCacheWhereInput {
                entity_type: Some(Self::string_eq(entity_type)),
                entity_id: Some(Self::string_eq(&entity_id)),
                ..ArtworkCacheWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        Ok(())
    }

    /// The artwork cache's entity kind for an entity name.
    fn artwork_entity_type(entity_name: &str) -> &'static str {
        match entity_name {
            "Movie" => "movie",
            "Show" => "show",
            "Season" => "season",
            "Episode" => "episode",
            "Album" => "album",
            "Artist" => "artist",
            "Audiobook" => "audiobook",
            "Person" => "person",
            _ => "unknown",
        }
    }

    /// Guide posters belong to a TVMaze show while the guide still lists it.
    async fn cleanup_guide_artwork(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let entry = Self::before::<ScheduleCache>(event, "ScheduleCache")?;
        let tvmaze_show_id = entry.tvmaze_show_id;
        let remaining = hook_ctx
            .query::<ScheduleCache>()
            .filter(ScheduleCacheWhereInput {
                tvmaze_show_id: Some(graphql_orm::graphql::filters::IntFilter {
                    eq: Some(tvmaze_show_id),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .limit(1)
            .fetch_all()
            .await
            .map_err(Self::db_error)?;
        if remaining.is_empty() {
            hook_ctx
                .delete_where::<ArtworkCache>(ArtworkCacheWhereInput {
                    entity_type: Some(Self::string_eq("guide")),
                    entity_id: Some(Self::string_eq(tvmaze_show_id.to_string())),
                    ..ArtworkCacheWhereInput::default()
                })
                .await
                .map_err(Self::db_error)?;
        }
        Ok(())
    }

    /// The one outcome for a Cast session that is replaced or whose device is
    /// removed: IDLE, ended now, the resume position kept. (A receiver that
    /// stops responding ends DISCONNECTED instead; see `retire_cast_monitor_with_binding`.)
    fn retired_cast_session(session: &CastSession, now: &str) -> UpdateCastSessionInput {
        UpdateCastSessionInput {
            player_state: Some("IDLE".to_string()),
            ended_at: Some(Some(now.to_string())),
            last_position: Some(Some(session.current_position)),
            ..UpdateCastSessionInput::default()
        }
    }

    async fn end_other_active_cast_sessions(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let session = Self::after::<CastSession>(event, "CastSession")?;
        let Some(device_id) = session.device_id.as_deref() else {
            return Ok(());
        };
        if session.ended_at.is_some() {
            return Ok(());
        }

        let now = Self::now_rfc3339();
        let sessions = hook_ctx
            .query::<CastSession>()
            .filter(CastSessionWhereInput {
                device_id: Some(Self::string_eq(device_id)),
                ..CastSessionWhereInput::default()
            })
            .fetch_all()
            .await
            .map_err(Self::db_error)?;
        for old_session in sessions {
            if old_session.id == session.id || old_session.ended_at.is_some() {
                continue;
            }
            hook_ctx
                .update_by_id::<CastSession>(
                    &old_session.id,
                    Self::retired_cast_session(&old_session, &now),
                )
                .await
                .map_err(Self::db_error)?;
        }

        Ok(())
    }

    async fn detach_cast_device_sessions(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let now = Self::now_rfc3339();
        let sessions = hook_ctx
            .query::<CastSession>()
            .filter(CastSessionWhereInput {
                device_id: Some(Self::string_eq(&event.id)),
                ..CastSessionWhereInput::default()
            })
            .fetch_all()
            .await
            .map_err(Self::db_error)?;
        for session in sessions {
            let mut update = if session.ended_at.is_none() {
                Self::retired_cast_session(&session, &now)
            } else {
                UpdateCastSessionInput::default()
            };
            update.device_id = Some(None);
            hook_ctx
                .update_by_id::<CastSession>(&session.id, update)
                .await
                .map_err(Self::db_error)?;
        }
        Ok(())
    }

    /// Stop playback on the receiver of a deleted device, after the delete
    /// commits. Hardware failures are only logged here; the rows are already
    /// ended by `detach_cast_device_sessions` in the deleting transaction.
    fn defer_cast_receiver_stop(
        &self,
        ctx: Option<&async_graphql::Context<'_>>,
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) {
        let Some(services) = self.services_from_ctx(ctx) else {
            warn!(
                cast_device_id = %event.id,
                "Services manager unavailable after Cast device deletion; receiver stop skipped"
            );
            return;
        };
        let device_id = event.id.clone();
        hook_ctx.defer(move |_db| async move {
            let Some(cast) = services.get_cast().await else {
                return Ok::<(), anyhow::Error>(());
            };
            match cast.stop_device_sessions(&device_id).await {
                Ok(0) => {}
                Ok(stopped) => info!(
                    cast_device_id = %device_id,
                    stopped,
                    "Stopped Cast playback on a deleted device"
                ),
                Err(error) => warn!(
                    cast_device_id = %device_id,
                    error = %error,
                    "Could not stop Cast playback on a deleted device"
                ),
            }
            Ok(())
        });
    }

    /// Apply the saved auto-discovery switch and interval to the running
    /// service after commit. Only those two fields matter: applying restarts the
    /// discovery loop, whose first tick scans immediately, so an unrelated edit
    /// (volume, quality) must not trigger it. The runtime never writes this
    /// entity, so there is no feedback loop.
    fn defer_cast_runtime_settings(
        &self,
        ctx: Option<&async_graphql::Context<'_>>,
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let after = Self::after::<CastSetting>(event, "CastSetting")?;
        if let Some(before) = event.before::<CastSetting>()?
            && before.auto_discovery_enabled == after.auto_discovery_enabled
            && before.discovery_interval_seconds == after.discovery_interval_seconds
        {
            return Ok(());
        }
        let Ok(interval) = u64::try_from(after.discovery_interval_seconds) else {
            return Ok(());
        };
        let auto_discovery = after.auto_discovery_enabled;
        let Some(services) = self.services_from_ctx(ctx) else {
            debug!(
                "Services manager unavailable after Cast settings change; runtime apply skipped"
            );
            return Ok(());
        };
        hook_ctx.defer(move |_db| async move {
            // Absent while the database seeds defaults at startup; the service
            // reads the persisted row itself when it starts.
            let Some(cast) = services.get_cast().await else {
                return Ok::<(), anyhow::Error>(());
            };
            cast.apply_runtime_settings(auto_discovery, interval).await
        });
        Ok(())
    }

    /// Whether an edit to a quality profile can change any file's evaluation.
    /// Naming and search-only fields (seeder floor, release age) cannot.
    fn quality_profile_changes_evaluation(before: &QualityProfile, after: &QualityProfile) -> bool {
        let mut comparable = after.clone();
        comparable.name = before.name.clone();
        comparable.updated_at = before.updated_at.clone();
        comparable.min_seeders = before.min_seeders;
        comparable.max_release_age_days = before.max_release_age_days;
        serde_json::to_value(before).ok() != serde_json::to_value(&comparable).ok()
    }

    /// Libraries holding files that can resolve to this profile: the libraries
    /// assigned it, the libraries of titles overriding it, and every library when it
    /// is (or was) the default that unassigned files fall back to. Read inside the
    /// transaction so a deleted profile's references are still visible.
    async fn libraries_resolving_quality_profile(
        tx: &mut MutationContext<'_>,
        profile_id: &str,
        is_default: bool,
    ) -> async_graphql::Result<Vec<String>> {
        let mut ids = BTreeSet::new();
        if is_default {
            for library in transaction_rows::<Library, _>(tx, LibraryWhereInput::default())
                .await
                .map_err(Self::db_error)?
            {
                ids.insert(library.id);
            }
            return Ok(ids.into_iter().collect());
        }
        macro_rules! collect {
            ($ty:ty, $filter:ident, $library:expr) => {
                for row in transaction_rows::<$ty, _>(
                    tx,
                    $filter {
                        quality_profile_id: Some(Self::string_eq(profile_id)),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Self::db_error)?
                {
                    ids.insert($library(row));
                }
            };
        }
        collect!(Library, LibraryWhereInput, |row: Library| row.id);
        collect!(Movie, MovieWhereInput, |row: Movie| row.library_id);
        collect!(Show, ShowWhereInput, |row: Show| row.library_id);
        collect!(Album, AlbumWhereInput, |row: Album| row.library_id);
        collect!(Audiobook, AudiobookWhereInput, |row: Audiobook| row
            .library_id);
        Ok(ids.into_iter().collect())
    }

    /// Re-evaluate quality after commit. Bulk work is spawned: `commit_and_emit`
    /// awaits deferred actions inline, and a library pass must not hold the request
    /// that triggered it. The pass only writes `qualityStatus`, which the MediaFile
    /// hook ignores, so it cannot loop. Failures are logged by the pass.
    fn defer_quality_recompute<F, Fut>(hook_ctx: &mut MutationContext<'_>, run: F)
    where
        F: FnOnce(Database) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        hook_ctx.defer(move |db| async move {
            tokio::spawn(run(db));
            Ok::<(), anyhow::Error>(())
        });
    }

    async fn requalify_for_quality_profile(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let (profile, relevant) = match event.action {
            ChangeAction::Updated => {
                let before = Self::before::<QualityProfile>(event, "QualityProfile")?;
                let after = Self::after::<QualityProfile>(event, "QualityProfile")?;
                // The default flag moving changes which profile every unassigned
                // file falls back to, for the old default as well as the new one.
                let relevant = Self::quality_profile_changes_evaluation(before, after)
                    || before.is_default != after.is_default;
                (after.clone(), relevant)
            }
            _ => (
                Self::before::<QualityProfile>(event, "QualityProfile")?.clone(),
                true,
            ),
        };
        if !relevant {
            return Ok(());
        }
        let scope_is_default = profile.is_default
            || event
                .before::<QualityProfile>()?
                .is_some_and(|before| before.is_default);
        let libraries =
            Self::libraries_resolving_quality_profile(hook_ctx, &profile.id, scope_is_default)
                .await?;
        if libraries.is_empty() {
            return Ok(());
        }
        Self::defer_quality_recompute(hook_ctx, move |db| async move {
            let summary =
                crate::services::quality::recompute::recompute_libraries(&db, &libraries).await;
            info!(
                quality_profile_id = %profile.id,
                libraries = libraries.len(),
                evaluated = summary.evaluated,
                failed = summary.failed,
                "Re-evaluated quality status after a quality profile change"
            );
        });
        Ok(())
    }

    /// A library or title assigned a different profile re-evaluates the files it
    /// governs.
    fn requalify_for_assignment(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        use crate::services::quality::recompute::{self, TitleKind};
        macro_rules! changed {
            ($ty:ty, $name:literal) => {{
                let before = Self::before::<$ty>(event, $name)?;
                let after = Self::after::<$ty>(event, $name)?;
                (before.quality_profile_id != after.quality_profile_id).then(|| after.id.clone())
            }};
        }
        let id = event.id.clone();
        match event.entity_name {
            "Library" => {
                if changed!(Library, "Library").is_some() {
                    Self::defer_quality_recompute(hook_ctx, move |db| async move {
                        recompute::recompute_libraries(&db, &[id]).await;
                    });
                }
            }
            name => {
                let kind = match name {
                    "Movie" if changed!(Movie, "Movie").is_some() => TitleKind::Movie,
                    "Show" if changed!(Show, "Show").is_some() => TitleKind::Show,
                    "Album" if changed!(Album, "Album").is_some() => TitleKind::Album,
                    "Audiobook" if changed!(Audiobook, "Audiobook").is_some() => {
                        TitleKind::Audiobook
                    }
                    _ => return Ok(()),
                };
                Self::defer_quality_recompute(hook_ctx, move |db| async move {
                    recompute::recompute_title(&db, kind, &id).await;
                });
            }
        }
        Ok(())
    }

    /// Re-evaluate one file after commit when its analysis or its link (which
    /// decides its profile) changed, as `analyze_media_file_inner` used to do by
    /// hand. Only analysed files have a status to judge;
    /// the status write itself is not a trigger, which ends the loop.
    fn requalify_media_file(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let after = Self::after::<MediaFile>(event, "MediaFile")?;
        if after.analyzed_at.is_none() {
            return Ok(());
        }
        let relevant = match event.before::<MediaFile>()? {
            None => true,
            Some(before) => {
                before.analyzed_at != after.analyzed_at
                    || before.library_id != after.library_id
                    || before.movie_id != after.movie_id
                    || before.episode_id != after.episode_id
                    || before.track_id != after.track_id
                    || before.chapter_id != after.chapter_id
                    || before.path != after.path
                    || before.size != after.size
                    || before.container != after.container
                    || before.video_codec != after.video_codec
                    || before.audio_codec != after.audio_codec
                    || before.resolution != after.resolution
                    || before.is_hdr != after.is_hdr
                    || before.hdr_type != after.hdr_type
                    || before.audio_channels != after.audio_channels
            }
        };
        if !relevant {
            return Ok(());
        }
        // One file is cheap, so this runs inline after commit: the analysis job that
        // wrote the properties finishes with the status already current.
        let id = event.id.clone();
        hook_ctx.defer(move |db| async move {
            crate::services::quality::recompute::recompute_files(&db, &[id]).await;
            Ok::<(), anyhow::Error>(())
        });
        Ok(())
    }

    /// A deleted user's sessions go with them: a refresh token left behind would still
    /// name a user that no longer exists. Revocation on disable or role change is
    /// `revoke_user_sessions_if_needed`; deletion removes the rows instead.
    async fn cleanup_user_refresh_tokens(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        hook_ctx
            .delete_where::<RefreshToken>(RefreshTokenWhereInput {
                user_id: Some(Self::string_eq(&event.id)),
                ..RefreshTokenWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        Ok(())
    }

    /// Refuse to delete a torrent's files while completed-file post-processing is still
    /// importing them. Runs before the delete so the error reaches the client and the
    /// transaction rolls back with the row, the files and the live torrent all intact.
    async fn guard_torrent_file_deletion(
        &self,
        ctx: Option<&async_graphql::Context<'_>>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let torrent = Self::before::<Torrent>(event, "Torrent")?;
        if !torrent.delete_files_on_remove {
            return Ok(());
        }
        let Some(service) = self.torrent_service(ctx).await else {
            return Ok(());
        };
        service
            .ensure_files_deletable(&torrent.info_hash)
            .await
            .map_err(|error| {
                // Resolver failures without a public code are sanitized to "internal error".
                crate::services::graphql::error::public_error(
                    crate::services::graphql::error::CONFLICT_CODE,
                    error.to_string(),
                )
            })
    }

    /// Stop the torrent in the session after the delete has committed.
    ///
    /// The work is irreversible (it can kill a live download and delete its payload), so
    /// it must not run before commit. `commit_and_emit` awaits deferred actions inline and
    /// only logs their failures, so the deferred action just hands the work to its own task
    /// and that task reports its own failures.
    fn defer_torrent_runtime_removal(
        &self,
        ctx: Option<&async_graphql::Context<'_>>,
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let torrent = Self::before::<Torrent>(event, "Torrent")?;
        let info_hash = torrent.info_hash.clone();
        let name = torrent.name.clone();
        let delete_files = torrent.delete_files_on_remove;
        let Some(services) = self.services_from_ctx(ctx) else {
            // Offline callers (repair, tests on a bare database) have no torrent session.
            debug!(
                info_hash = %info_hash,
                torrent_name = %name,
                "No services manager after torrent deletion; torrent session left untouched"
            );
            return Ok(());
        };
        hook_ctx.defer(move |_db| async move {
            tokio::spawn(async move {
                let Some(service) = services.get_torrent().await else {
                    return;
                };
                if let Err(error) = service.remove_runtime(&info_hash, delete_files).await {
                    error!(
                        info_hash = %info_hash,
                        torrent_name = %name,
                        delete_files,
                        error = %error,
                        "Torrent '{}' ({}) was deleted but could not be removed from the torrent session; it keeps running without a database record until the session is cleaned up",
                        name,
                        info_hash
                    );
                }
            });
            Ok::<(), anyhow::Error>(())
        });
        Ok(())
    }

    /// After a title leaves a library, or a library is made, downloads compared with it are
    /// looked at again: what they matched may be gone, or they may now have somewhere to go.
    fn defer_identified_recheck(
        &self,
        ctx: Option<&async_graphql::Context<'_>>,
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) {
        let owner_and_type = match event.entity_name {
            "Album" => event
                .before::<Album>()
                .ok()
                .flatten()
                .map(|row| (row.user_id.clone(), "music")),
            "Movie" => event
                .before::<Movie>()
                .ok()
                .flatten()
                .map(|row| (row.user_id.clone(), "movies")),
            "Show" => event
                .before::<Show>()
                .ok()
                .flatten()
                .map(|row| (row.user_id.clone(), "tv")),
            "Audiobook" => event
                .before::<Audiobook>()
                .ok()
                .flatten()
                .map(|row| (row.user_id.clone(), "audiobooks")),
            "Library" => event
                .after::<Library>()
                .ok()
                .flatten()
                .map(|row| (row.user_id.clone(), row.library_type.as_str())),
            _ => None,
        };
        let (Some((owner, library_type)), Some(services)) =
            (owner_and_type, self.services_from_ctx(ctx))
        else {
            return;
        };
        let library_type = library_type.to_string();
        hook_ctx.defer(move |_db| async move {
            if let Some(scan) = services.get_library_scan().await {
                scan.recheck_identified(&owner, &library_type);
            }
            Ok::<(), anyhow::Error>(())
        });
    }

    async fn torrent_service(
        &self,
        ctx: Option<&async_graphql::Context<'_>>,
    ) -> Option<Arc<crate::services::torrent::TorrentService>> {
        self.services_from_ctx(ctx)?.get_torrent().await
    }

    async fn cleanup_torrent_files(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        hook_ctx
            .delete_where::<TorrentFile>(TorrentFileWhereInput {
                torrent_id: Some(Self::string_eq(&event.id)),
                ..TorrentFileWhereInput::default()
            })
            .await
            .map_err(Self::db_error)?;
        Ok(())
    }
}

impl MutationHook for AppMutationHook {
    fn on_mutation<'a>(
        &'a self,
        ctx: Option<&'a async_graphql::Context<'_>>,
        hook_ctx: &'a mut MutationContext<'_>,
        event: &'a MutationEvent,
    ) -> BoxFuture<'a, async_graphql::Result<()>> {
        Box::pin(async move {
            crate::services::graphql::personal_policy::on_mutation(ctx, hook_ctx, event).await?;
            match (&event.phase, &event.action, event.entity_name) {
                (MutationPhase::Before, ChangeAction::Deleted, "Library") => {
                    Self::cleanup_library_children(hook_ctx, &event.id).await?;
                }
                (
                    MutationPhase::Before,
                    ChangeAction::Deleted,
                    "Show" | "Album" | "Audiobook" | "Movie" | "RssFeed" | "LibraryScanRun"
                    | "Person",
                ) => {
                    Self::cleanup_parent_children(hook_ctx, event).await?;
                }
                (MutationPhase::Before, ChangeAction::Deleted, "User") => {
                    Self::cleanup_user_refresh_tokens(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Updated, "User") => {
                    Self::revoke_user_sessions_if_needed(hook_ctx, event).await?;
                }
                (
                    MutationPhase::After,
                    ChangeAction::Created | ChangeAction::Updated,
                    "MediaFile",
                ) => {
                    Self::sync_media_file_link_update(hook_ctx, event).await?;
                }
                (MutationPhase::Before, ChangeAction::Deleted, "MediaFile") => {
                    Self::clear_deleted_media_file_links(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Created | ChangeAction::Updated, "Movie") => {
                    Self::sync_movie_media_file_update(hook_ctx, event).await?;
                }
                (
                    MutationPhase::After,
                    ChangeAction::Created | ChangeAction::Updated,
                    "Episode",
                ) => {
                    Self::sync_episode_media_file_update(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Created | ChangeAction::Updated, "Track") => {
                    Self::sync_track_media_file_update(hook_ctx, event).await?;
                }
                (
                    MutationPhase::After,
                    ChangeAction::Created | ChangeAction::Updated,
                    "Chapter",
                ) => {
                    Self::sync_chapter_media_file_update(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Deleted, "StorageObject") => {
                    self.defer_storage_object_delete(ctx, hook_ctx, event)?;
                }
                (MutationPhase::After, ChangeAction::Deleted, "ArtworkCache") => {
                    Self::cleanup_deleted_artwork_cache(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Updated, "ArtworkCache") => {
                    Self::cleanup_replaced_artwork_cache(hook_ctx, event).await?;
                    Self::publish_artwork_owner(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Created, "ArtworkCache") => {
                    Self::publish_artwork_owner(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Deleted, "ScheduleCache") => {
                    Self::cleanup_guide_artwork(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Created, "CastSession") => {
                    Self::end_other_active_cast_sessions(hook_ctx, event).await?;
                }
                (MutationPhase::Before, ChangeAction::Deleted, "CastDevice") => {
                    Self::detach_cast_device_sessions(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Deleted, "CastDevice") => {
                    self.defer_cast_receiver_stop(ctx, hook_ctx, event);
                }
                (
                    MutationPhase::After,
                    ChangeAction::Created | ChangeAction::Updated,
                    "CastSetting",
                ) => {
                    self.defer_cast_runtime_settings(ctx, hook_ctx, event)?;
                }
                (MutationPhase::Before, ChangeAction::Deleted, "Torrent") => {
                    // Everything that should fail visibly to the client belongs here: a
                    // Before hook can abort the transaction, a deferred action cannot.
                    self.guard_torrent_file_deletion(ctx, event).await?;
                    Self::cleanup_torrent_files(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Deleted, "Torrent") => {
                    self.defer_torrent_runtime_removal(ctx, hook_ctx, event)?;
                }
                (
                    MutationPhase::After,
                    ChangeAction::Deleted,
                    "Album" | "Movie" | "Show" | "Audiobook",
                )
                | (MutationPhase::After, ChangeAction::Created, "Library") => {
                    self.defer_identified_recheck(ctx, hook_ctx, event);
                }
                _ => {}
            }

            if event.phase == MutationPhase::Before
                && event.action == ChangeAction::Deleted
                && matches!(event.entity_name, "Movie" | "Episode" | "Track" | "Chapter")
            {
                Self::detach_deleted_title(hook_ctx, event).await?;
            }
            if event.phase == MutationPhase::Before
                && event.action == ChangeAction::Deleted
                && matches!(
                    event.entity_name,
                    "Movie"
                        | "Show"
                        | "Episode"
                        | "Album"
                        | "Audiobook"
                        | "Track"
                        | "Chapter"
                        | "Torrent"
                        | "RssFeed"
                        | "PendingFileMatch"
                        | "Artist"
                        | "QualityProfile"
                        | "TorznabCategory"
                )
            {
                Self::detach_deleted_references(hook_ctx, event).await?;
            }
            if event.phase == MutationPhase::Before
                && event.action == ChangeAction::Deleted
                && matches!(
                    event.entity_name,
                    "Movie"
                        | "Show"
                        | "Season"
                        | "Episode"
                        | "Album"
                        | "Artist"
                        | "Audiobook"
                        | "Person"
                        | "Collection"
                )
            {
                Self::cleanup_entity_artwork(hook_ctx, event).await?;
            }
            if event.phase == MutationPhase::After && event.action == ChangeAction::Updated {
                match event.entity_name {
                    "Library" | "Movie" | "Show" | "Album" | "Audiobook" => {
                        Self::requalify_for_assignment(hook_ctx, event)?;
                    }
                    "MediaFile" => Self::requalify_media_file(hook_ctx, event)?,
                    _ => {}
                }
            }
            if event.entity_name == "QualityProfile"
                && match event.action {
                    ChangeAction::Updated => event.phase == MutationPhase::After,
                    ChangeAction::Deleted => event.phase == MutationPhase::Before,
                    ChangeAction::Created => false,
                }
            {
                Self::requalify_for_quality_profile(hook_ctx, event).await?;
            }
            Self::maintain_summaries_and_parent_events(hook_ctx, event).await?;

            if Self::source_change_requires_reload(event) {
                self.defer_source_reload(ctx, hook_ctx, event);
            }

            Ok(())
        })
    }
}

#[async_trait]
impl Service for DatabaseService {
    fn name(&self) -> &str {
        "database"
    }

    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }

    async fn start(&self) -> Result<()> {
        info!(
            service = "database",
            "Database service starting: validating connection, syncing schema, and seeding defaults"
        );
        info!(service = "database", "Validating GraphQL ORM entity schema");
        bootstrap_schema(self.pool())
            .await
            .context("GraphQL ORM schema bootstrap failed")?;
        info!(
            service = "database",
            "GraphQL ORM entity schema is up to date"
        );

        info!(
            service = "database",
            "Applying GraphQL ORM default seed data"
        );
        seed_defaults(self.pool())
            .await
            .context("GraphQL ORM default seed data failed")?;
        info!(
            service = "database",
            "GraphQL ORM default seed data applied"
        );

        info!(service = "database", "Database service started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.pool.pool().close().await;
        info!(
            service = "database",
            "Database service stopped: connection pool closed"
        );
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        Ok(ServiceHealth::healthy())
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use graphql_orm::DbPool;

    #[test]
    fn entity_schema_declares_provider_unique_indexes() {
        let entities = entity_metadata();
        let target_schema = graphql_orm::graphql::orm::SchemaModel::from_entities(&entities);

        let shows = target_schema
            .tables
            .iter()
            .find(|table| table.table_name == "shows")
            .expect("shows table should be in entity schema");
        assert!(shows.indexes.iter().any(|index| {
            index.name == "idx_shows_library_id_tvmaze_id"
                && index.is_unique
                && index.columns == vec!["library_id", "tvmaze_id"]
        }));
        assert!(shows.indexes.iter().any(|index| {
            index.name == "idx_shows_library_id_tvdb_id"
                && index.is_unique
                && index.columns == vec!["library_id", "tvdb_id"]
        }));

        let episodes = target_schema
            .tables
            .iter()
            .find(|table| table.table_name == "episodes")
            .expect("episodes table should be in entity schema");
        assert!(episodes.indexes.iter().any(|index| {
            index.name == "idx_episodes_show_id_tvmaze_id"
                && index.is_unique
                && index.columns == vec!["show_id", "tvmaze_id"]
        }));
    }

    #[tokio::test]
    async fn ordinary_startup_cannot_apply_field_rename_as_drop_and_add() {
        let pool = DbPool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite database should connect");
        let db = Database::new(pool);
        let current = graphql_orm::graphql::orm::SchemaModel::from_entities(&entity_metadata());
        let mut renamed = current.clone();
        let app_settings = renamed
            .tables
            .iter_mut()
            .find(|table| table.table_name == "app_settings")
            .expect("app_settings should be in entity schema");
        let description = app_settings
            .columns
            .iter_mut()
            .find(|column| column.name == "description")
            .expect("description column should exist");
        description.name = "renamed_description".to_string();
        let initial = db
            .schema()
            .plan_migration(
                "rename-safety-v1",
                "initial schema",
                &graphql_orm::graphql::orm::SchemaModel {
                    extensions: Vec::new(),
                    tables: Vec::new(),
                },
                &current,
            )
            .expect("initial schema should plan");
        assert_eq!(schema_plan_class(&initial), SchemaChangeClass::Additive);
        db.schema()
            .apply_migration(
                &initial,
                ApplyOptions {
                    additive_only: true,
                    ..ApplyOptions::default()
                },
            )
            .await
            .expect("initial additive schema should apply");

        let rename_plan = db
            .schema()
            .plan_migration("rename-safety-v2", "rename field", &current, &renamed)
            .expect("rename should plan");
        let error = apply_startup_schema_plan(&db, &rename_plan)
            .await
            .expect_err("ordinary startup must block rename-as-drop/add");
        assert!(
            error
                .to_string()
                .contains("ordinary startup only applies additive")
        );

        let current_validation = db
            .schema()
            .validate_against_entities(&entity_metadata())
            .await
            .expect("original schema should still validate");
        assert!(
            !current_validation.has_errors(),
            "blocked startup must leave the original column intact"
        );
    }

    #[tokio::test]
    async fn sql_type_conversion_is_classified_as_destructive_and_blocked() {
        let pool = DbPool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite database should connect");
        let db = Database::new(pool);
        let current = graphql_orm::graphql::orm::SchemaModel::from_entities(&entity_metadata());
        let mut converted = current.clone();
        let app_settings = converted
            .tables
            .iter_mut()
            .find(|table| table.table_name == "app_settings")
            .expect("app_settings should be in entity schema");
        let value = app_settings
            .columns
            .iter_mut()
            .find(|column| column.name == "value")
            .expect("value column should exist");
        value.sql_type = "INTEGER".to_string();
        let initial = db
            .schema()
            .plan_migration(
                "type-safety-v1",
                "initial schema",
                &graphql_orm::graphql::orm::SchemaModel {
                    extensions: Vec::new(),
                    tables: Vec::new(),
                },
                &current,
            )
            .expect("initial schema should plan");
        db.schema()
            .apply_migration(
                &initial,
                ApplyOptions {
                    additive_only: true,
                    ..ApplyOptions::default()
                },
            )
            .await
            .expect("initial additive schema should apply");

        let conversion = db
            .schema()
            .plan_migration("type-safety-v2", "convert value", &current, &converted)
            .expect("type conversion should plan");
        assert_eq!(
            schema_plan_class(&conversion),
            SchemaChangeClass::Destructive
        );
        assert!(
            apply_startup_schema_plan(&db, &conversion).await.is_err(),
            "ordinary startup must block type conversion"
        );
    }

    async fn fresh_database() -> Database {
        let pool = DbPool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite database should connect");
        let db = Database::new(pool);
        let service = DatabaseService::new(db);
        service
            .start()
            .await
            .expect("database service should start");
        service.pool().clone()
    }

    fn now() -> String {
        AppMutationHook::now_rfc3339()
    }

    async fn create_user(db: &Database) -> User {
        User::insert(
            db,
            CreateUserInput {
                username: "test-user".to_string(),
                email: Some("test@example.com".to_string()),
                password_hash: "hash".to_string(),
                role: "User".to_string(),
                display_name: None,
                avatar_url: None,
                is_active: true,
                last_login_at: None,
            },
        )
        .await
        .expect("user should insert")
    }

    async fn create_library(db: &Database, user_id: &str) -> Library {
        Library::insert(
            db,
            CreateLibraryInput {
                user_id: user_id.to_string(),
                name: "Movies".to_string(),
                path: "/tmp/movies".to_string(),
                library_type: "movies".to_string(),
                icon: None,
                color: None,
                auto_scan: false,
                auto_organize: false,
                auto_resolve_duplicates: None,
                naming_pattern: "default".to_string(),
                scan_interval_minutes: 60,
                watch_for_changes: false,
                scanning: false,
                last_scanned_at: None,
                quality_profile_id: None,
            },
        )
        .await
        .expect("library should insert")
    }

    async fn create_movie(db: &Database, library_id: &str, user_id: &str, title: &str) -> Movie {
        Movie::insert(
            db,
            CreateMovieInput {
                locked_fields: None,
                library_id: library_id.to_string(),
                user_id: user_id.to_string(),
                title: title.to_string(),
                sort_title: None,
                original_title: None,
                year: Some(2024),
                tmdb_id: None,
                imdb_id: None,
                overview: None,
                tagline: None,
                runtime: None,
                genres: Vec::new(),
                director: None,
                cast_names: Vec::new(),
                production_countries: Vec::new(),
                spoken_languages: Vec::new(),
                tmdb_rating: None,
                tmdb_vote_count: None,
                poster_url: None,
                backdrop_url: None,
                collection_id: None,
                collection_name: None,
                collection_poster_url: None,
                release_date: None,
                certification: None,
                monitored: true,
                tmdb_status: None,
                wanted: true,
                ignored: None,
                download_status: None,
                has_file: false,
                media_file_id: None,
                quality_profile_id: None,
            },
        )
        .await
        .expect("movie should insert")
    }

    async fn create_media_file(db: &Database, library_id: &str, path: &str) -> MediaFile {
        MediaFile::insert(
            db,
            CreateMediaFileInput {
                library_id: Some(library_id.to_string()),
                episode_id: None,
                movie_id: None,
                track_id: None,
                chapter_id: None,
                path: path.to_string(),
                relative_path: None,
                original_name: None,
                size: 1024,
                container: Some("mkv".to_string()),
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                resolution: None,
                is_hdr: false,
                hdr_type: None,
                audio_channels: None,
                metadata: None,
                content_type: Some("movie".to_string()),
                added_at: now(),
                analyzed_at: None,
                file_modified_at: None,
                match_type: None,
                matched_by_user_id: None,
                match_confirmed_at: None,
                quality_status: None,
            },
        )
        .await
        .expect("media file should insert")
    }

    #[tokio::test]
    async fn disabling_user_revokes_active_refresh_tokens_for_that_user() {
        let db = fresh_database().await;
        let user = create_user(&db).await;
        let other = User::insert(
            &db,
            CreateUserInput {
                username: "other-user".to_string(),
                email: None,
                password_hash: "hash".to_string(),
                role: "User".to_string(),
                display_name: None,
                avatar_url: None,
                is_active: true,
                last_login_at: None,
            },
        )
        .await
        .expect("other user should insert");

        let token = RefreshToken::insert(
            &db,
            CreateRefreshTokenInput {
                id: "token-1".to_string(),
                user_id: user.id.clone(),
                token_hash: "hash-1".to_string(),
                session_id: "session-1".to_string(),
                session_family_id: "family-1".to_string(),
                scopes: vec!["read".to_string()],
                session: "{}".to_string(),
                grant_source: None,
                roles: None,
                authorization_roles: None,
                ip_address: None,
                user_agent: None,
                expires_at: now(),
                last_used_at: None,
                revoked_at: None,
                replaced_by_token_id: None,
                revocation_reason: None,
            },
        )
        .await
        .expect("refresh token should insert");
        let other_token = RefreshToken::insert(
            &db,
            CreateRefreshTokenInput {
                id: "token-2".to_string(),
                user_id: other.id.clone(),
                token_hash: "hash-2".to_string(),
                session_id: "session-2".to_string(),
                session_family_id: "family-2".to_string(),
                scopes: vec!["read".to_string()],
                session: "{}".to_string(),
                grant_source: None,
                roles: None,
                authorization_roles: None,
                ip_address: None,
                user_agent: None,
                expires_at: now(),
                last_used_at: None,
                revoked_at: None,
                replaced_by_token_id: None,
                revocation_reason: None,
            },
        )
        .await
        .expect("other refresh token should insert");

        User::update_by_id(
            &db,
            &user.id,
            UpdateUserInput {
                is_active: Some(false),
                ..UpdateUserInput::default()
            },
        )
        .await
        .expect("user should update");

        let token = RefreshToken::get(db.pool(), &token.id)
            .await
            .expect("token should query")
            .expect("token should exist");
        let other_token = RefreshToken::get(db.pool(), &other_token.id)
            .await
            .expect("other token should query")
            .expect("other token should exist");
        assert!(token.revoked_at.is_some());
        assert_eq!(token.revocation_reason.as_deref(), Some("AdminRevoked"));
        assert!(other_token.revoked_at.is_none());
    }

    #[tokio::test]
    async fn updating_media_file_movie_link_updates_movie_inverse_fields() {
        let db = fresh_database().await;
        let user = create_user(&db).await;
        let library = create_library(&db, &user.id).await;
        let movie = create_movie(&db, &library.id, &user.id, "One").await;
        let media_file = create_media_file(&db, &library.id, "/tmp/movies/one.mkv").await;

        MediaFile::update_by_id(
            &db,
            &media_file.id,
            UpdateMediaFileInput {
                movie_id: Some(Some(movie.id.clone())),
                ..UpdateMediaFileInput::default()
            },
        )
        .await
        .expect("media file should update");

        let movie = Movie::get(db.pool(), &movie.id)
            .await
            .expect("movie should query")
            .expect("movie should exist");
        assert_eq!(movie.media_file_id.as_deref(), Some(media_file.id.as_str()));
        assert!(!movie.wanted);
        assert!(movie.has_file);
    }

    #[tokio::test]
    async fn deleting_torrent_deletes_torrent_files() {
        let db = fresh_database().await;
        let user = create_user(&db).await;
        let torrent = Torrent::insert(
            &db,
            CreateTorrentInput {
                user_id: user.id.clone(),
                info_hash: "infohash".to_string(),
                magnet_uri: None,
                name: "Torrent".to_string(),
                state: "downloading".to_string(),
                progress: 0.0,
                total_bytes: 1024,
                downloaded_bytes: 0,
                uploaded_bytes: 0,
                uploaded_bytes_total: 0,
                delete_files_on_remove: false,
                save_path: "/tmp".to_string(),
                download_path: None,
                source_url: None,
                source_feed_id: None,
                source_indexer_id: None,
                library_id: None,
                identification_kind: None,
                identification_status: None,
                identification_summary: None,
                post_process_status: None,
                post_process_error: None,
                processed_at: None,
                episode_id: None,
                movie_id: None,
                track_id: None,
                chapter_id: None,
                show_id: None,
                album_id: None,
                audiobook_id: None,
                minimum_ratio: None,
                minimum_seed_time_minutes: None,
                season: None,
                excluded_files: Vec::new(),
                added_at: now(),
                completed_at: None,
            },
        )
        .await
        .expect("torrent should insert");
        TorrentFile::insert(
            &db,
            CreateTorrentFileInput {
                torrent_id: torrent.id.clone(),
                file_index: 0,
                file_path: "/tmp/file.mkv".to_string(),
                relative_path: "file.mkv".to_string(),
                file_size: 1024,
                downloaded_bytes: 0,
                progress: 0.0,
                media_file_id: None,
                is_excluded: false,
            },
        )
        .await
        .expect("torrent file should insert");

        Torrent::delete_by_id(&db, &torrent.id)
            .await
            .expect("torrent should delete");

        let remaining = TorrentFile::query(db.pool())
            .filter(TorrentFileWhereInput {
                torrent_id: Some(AppMutationHook::string_eq(&torrent.id)),
                ..TorrentFileWhereInput::default()
            })
            .fetch_all()
            .await
            .expect("torrent files should query");
        assert!(remaining.is_empty());
    }
}

#[cfg(test)]
#[path = "database_summary_tests.rs"]
mod summary_tests;

#[cfg(test)]
#[path = "database_parent_summary_tests.rs"]
mod parent_summary_tests;

#[cfg(test)]
#[path = "entity_relations_tests.rs"]
mod entity_relations_tests;

#[cfg(test)]
#[path = "database_write_rules_tests.rs"]
mod write_rules_tests;
