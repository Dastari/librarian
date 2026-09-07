//! Database service: wraps the SQLite pool for lifecycle (start/stop/health) and dependencies.
//!
//! Other services that need the database (e.g. logging) should declare `dependencies: ["database"]`.

use std::sync::{Arc, Weak};
use std::time::Duration;

use anyhow::{Context, Result};
use async_graphql::ErrorExtensions;
use async_trait::async_trait;
use futures::future::BoxFuture;
use graphql_orm::graphql::filters::StringFilter;
use graphql_orm::graphql::orm::{
    ApplyOptions, ChangeAction, Entity, MigrationRisk, MigrationStep, MutationContext,
    MutationEvent, MutationHook, MutationPhase, PaginationConfig, PlannedMigration,
    PostCommitErrorHandler, WriteInputTransform,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tracing::{info, warn};

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
        Ok(Self { pool })
    }

    /// Access the pool (e.g. to clone for app state). Valid until [Service::stop] is called.
    pub fn pool(&self) -> &Database {
        &self.pool
    }

    fn install_hooks(pool: &mut Database, services: Option<Weak<ServicesManager>>) {
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
        <CastDevice as Entity>::metadata(),
        <CastSession as Entity>::metadata(),
        <CastSetting as Entity>::metadata(),
        <UsenetServer as Entity>::metadata(),
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
    let Some(plan) = plan_schema(pool).await? else {
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
    fn before_create<'a>(
        &'a self,
        ctx: Option<&'a async_graphql::Context<'_>>,
        db: &'a Database,
        entity_name: &'static str,
        input: &'a mut (dyn std::any::Any + Send + Sync),
    ) -> BoxFuture<'a, async_graphql::Result<()>> {
        Box::pin(async move {
            Self::assign_create_owner(ctx, entity_name, input);
            Self::validate_upsert_link_workaround(ctx, db, entity_name, input, true).await?;
            Self::reject_member_artwork_source_write(ctx, entity_name, input)?;
            if entity_name == "Source"
                && let Some(input) = input.downcast_mut::<CreateSourceInput>()
            {
                input.credentials = self
                    .encrypt_source_credentials(ctx, input.credentials.clone())
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
        _existing_row: Option<&'a (dyn std::any::Any + Send + Sync)>,
        input: &'a mut (dyn std::any::Any + Send + Sync),
    ) -> BoxFuture<'a, async_graphql::Result<()>> {
        Box::pin(async move {
            Self::reject_owner_update(ctx, entity_name, input)?;
            Self::validate_upsert_link_workaround(ctx, db, entity_name, input, false).await?;
            Self::reject_member_artwork_source_write(ctx, entity_name, input)?;
            if entity_name == "Source"
                && let Some(input) = input.downcast_mut::<UpdateSourceInput>()
                && let Some(credentials) = input.credentials.take()
            {
                input.credentials = Some(self.encrypt_source_credentials(ctx, credentials).await?);
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

#[derive(Debug, Default)]
struct AppMutationHook {
    services: Option<Weak<ServicesManager>>,
}

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
        let shows = hook_ctx
            .query::<Show>()
            .filter(ShowWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..ShowWhereInput::default()
            })
            .fetch_all()
            .await
            .map_err(Self::db_error)?;
        for show in &shows {
            hook_ctx
                .delete_where::<Episode>(EpisodeWhereInput {
                    show_id: Some(Self::string_eq(&show.id)),
                    ..EpisodeWhereInput::default()
                })
                .await
                .map_err(Self::db_error)?;
        }

        let albums = hook_ctx
            .query::<Album>()
            .filter(AlbumWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..AlbumWhereInput::default()
            })
            .fetch_all()
            .await
            .map_err(Self::db_error)?;
        for album in &albums {
            hook_ctx
                .delete_where::<Track>(TrackWhereInput {
                    album_id: Some(Self::string_eq(&album.id)),
                    ..TrackWhereInput::default()
                })
                .await
                .map_err(Self::db_error)?;
        }

        let audiobooks = hook_ctx
            .query::<Audiobook>()
            .filter(AudiobookWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..AudiobookWhereInput::default()
            })
            .fetch_all()
            .await
            .map_err(Self::db_error)?;
        for audiobook in &audiobooks {
            hook_ctx
                .delete_where::<Chapter>(ChapterWhereInput {
                    audiobook_id: Some(Self::string_eq(&audiobook.id)),
                    ..ChapterWhereInput::default()
                })
                .await
                .map_err(Self::db_error)?;
        }

        let movies = hook_ctx
            .query::<Movie>()
            .filter(MovieWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..MovieWhereInput::default()
            })
            .fetch_all()
            .await
            .map_err(Self::db_error)?;
        for movie in &movies {
            hook_ctx
                .delete_where::<MovieCastCredit>(MovieCastCreditWhereInput {
                    movie_id: Some(Self::string_eq(&movie.id)),
                    ..MovieCastCreditWhereInput::default()
                })
                .await
                .map_err(Self::db_error)?;
        }

        let rss_feeds = hook_ctx
            .query::<RssFeed>()
            .filter(RssFeedWhereInput {
                library_id: Some(Self::string_eq(library_id)),
                ..RssFeedWhereInput::default()
            })
            .fetch_all()
            .await
            .map_err(Self::db_error)?;
        for feed in &rss_feeds {
            hook_ctx
                .delete_where::<RssFeedItem>(RssFeedItemWhereInput {
                    feed_id: Some(Self::string_eq(&feed.id)),
                    ..RssFeedItemWhereInput::default()
                })
                .await
                .map_err(Self::db_error)?;
        }

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
        let should_revoke = (before.is_active && !after.is_active) || before.role != after.role;
        if !should_revoke {
            return Ok(());
        }

        let revoked_at = Self::now_rfc3339();
        let tokens = hook_ctx
            .query::<RefreshToken>()
            .filter(RefreshTokenWhereInput {
                user_id: Some(Self::string_eq(&after.id)),
                ..RefreshTokenWhereInput::default()
            })
            .fetch_all()
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

    async fn sync_media_file_link_update(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = Self::before::<MediaFile>(event, "MediaFile")?;
        let after = Self::after::<MediaFile>(event, "MediaFile")?;

        let active_movie = after.movie_id.as_deref();
        let active_episode = if active_movie.is_none() {
            after.episode_id.as_deref()
        } else {
            None
        };
        let active_track = if active_movie.is_none() && active_episode.is_none() {
            after.track_id.as_deref()
        } else {
            None
        };
        let active_chapter =
            if active_movie.is_none() && active_episode.is_none() && active_track.is_none() {
                after.chapter_id.as_deref()
            } else {
                None
            };

        if after.movie_id.as_deref() != active_movie
            || after.episode_id.as_deref() != active_episode
            || after.track_id.as_deref() != active_track
            || after.chapter_id.as_deref() != active_chapter
        {
            Self::set_media_file_target(
                hook_ctx,
                &after.id,
                active_movie,
                active_episode,
                active_track,
                active_chapter,
            )
            .await?;
        }

        if before.movie_id.as_deref() != active_movie {
            if let Some(old_id) = before.movie_id.as_deref() {
                Self::set_movie_media_file(hook_ctx, old_id, None).await?;
            }
            if let Some(new_id) = active_movie {
                Self::set_movie_media_file(hook_ctx, new_id, Some(&after.id)).await?;
            }
        }
        if before.episode_id.as_deref() != active_episode {
            if let Some(old_id) = before.episode_id.as_deref() {
                Self::set_episode_media_file(hook_ctx, old_id, None).await?;
            }
            if let Some(new_id) = active_episode {
                Self::set_episode_media_file(hook_ctx, new_id, Some(&after.id)).await?;
            }
        }
        if before.track_id.as_deref() != active_track {
            if let Some(old_id) = before.track_id.as_deref() {
                Self::set_track_media_file(hook_ctx, old_id, None).await?;
            }
            if let Some(new_id) = active_track {
                Self::set_track_media_file(hook_ctx, new_id, Some(&after.id)).await?;
            }
        }
        if before.chapter_id.as_deref() != active_chapter {
            if let Some(old_id) = before.chapter_id.as_deref() {
                Self::set_chapter_media_file(hook_ctx, old_id, None).await?;
            }
            if let Some(new_id) = active_chapter {
                Self::set_chapter_media_file(hook_ctx, new_id, Some(&after.id)).await?;
            }
        }

        Ok(())
    }

    async fn clear_deleted_media_file_links(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let media_file = Self::before::<MediaFile>(event, "MediaFile")?;
        if let Some(movie_id) = media_file.movie_id.as_deref() {
            Self::set_movie_media_file(hook_ctx, movie_id, None).await?;
        }
        if let Some(episode_id) = media_file.episode_id.as_deref() {
            Self::set_episode_media_file(hook_ctx, episode_id, None).await?;
        }
        if let Some(track_id) = media_file.track_id.as_deref() {
            Self::set_track_media_file(hook_ctx, track_id, None).await?;
        }
        if let Some(chapter_id) = media_file.chapter_id.as_deref() {
            Self::set_chapter_media_file(hook_ctx, chapter_id, None).await?;
        }
        Ok(())
    }

    async fn sync_movie_media_file_update(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = Self::before::<Movie>(event, "Movie")?;
        let after = Self::after::<Movie>(event, "Movie")?;
        if before.media_file_id == after.media_file_id {
            return Ok(());
        }
        if let Some(old_media_file_id) = before.media_file_id.as_deref() {
            Self::set_media_file_target(hook_ctx, old_media_file_id, None, None, None, None)
                .await?;
        }
        if let Some(new_media_file_id) = after.media_file_id.as_deref() {
            Self::set_media_file_target(
                hook_ctx,
                new_media_file_id,
                Some(&after.id),
                None,
                None,
                None,
            )
            .await?;
        }
        Self::set_movie_media_file(hook_ctx, &after.id, after.media_file_id.as_deref()).await
    }

    async fn sync_episode_media_file_update(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = Self::before::<Episode>(event, "Episode")?;
        let after = Self::after::<Episode>(event, "Episode")?;
        if before.media_file_id == after.media_file_id {
            return Ok(());
        }
        if let Some(old_media_file_id) = before.media_file_id.as_deref() {
            Self::set_media_file_target(hook_ctx, old_media_file_id, None, None, None, None)
                .await?;
        }
        if let Some(new_media_file_id) = after.media_file_id.as_deref() {
            Self::set_media_file_target(
                hook_ctx,
                new_media_file_id,
                None,
                Some(&after.id),
                None,
                None,
            )
            .await?;
        }
        Self::set_episode_media_file(hook_ctx, &after.id, after.media_file_id.as_deref()).await
    }

    async fn sync_track_media_file_update(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = Self::before::<Track>(event, "Track")?;
        let after = Self::after::<Track>(event, "Track")?;
        if before.media_file_id == after.media_file_id {
            return Ok(());
        }
        if let Some(old_media_file_id) = before.media_file_id.as_deref() {
            Self::set_media_file_target(hook_ctx, old_media_file_id, None, None, None, None)
                .await?;
        }
        if let Some(new_media_file_id) = after.media_file_id.as_deref() {
            Self::set_media_file_target(
                hook_ctx,
                new_media_file_id,
                None,
                None,
                Some(&after.id),
                None,
            )
            .await?;
        }
        Self::set_track_media_file(hook_ctx, &after.id, after.media_file_id.as_deref()).await
    }

    async fn sync_chapter_media_file_update(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let before = Self::before::<Chapter>(event, "Chapter")?;
        let after = Self::after::<Chapter>(event, "Chapter")?;
        if before.media_file_id == after.media_file_id {
            return Ok(());
        }
        if let Some(old_media_file_id) = before.media_file_id.as_deref() {
            Self::set_media_file_target(hook_ctx, old_media_file_id, None, None, None, None)
                .await?;
        }
        if let Some(new_media_file_id) = after.media_file_id.as_deref() {
            Self::set_media_file_target(
                hook_ctx,
                new_media_file_id,
                None,
                None,
                None,
                Some(&after.id),
            )
            .await?;
        }
        Self::set_chapter_media_file(hook_ctx, &after.id, after.media_file_id.as_deref()).await
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

    async fn cleanup_deleted_artwork_cache(
        hook_ctx: &mut MutationContext<'_>,
        event: &MutationEvent,
    ) -> async_graphql::Result<()> {
        let cache = Self::before::<ArtworkCache>(event, "ArtworkCache")?;
        if cache.storage_object_id.is_empty() {
            return Ok(());
        }
        let remaining = hook_ctx
            .query::<ArtworkCache>()
            .filter(ArtworkCacheWhereInput {
                storage_object_id: Some(Self::string_eq(&cache.storage_object_id)),
                ..ArtworkCacheWhereInput::default()
            })
            .limit(1)
            .fetch_all()
            .await
            .map_err(Self::db_error)?;
        if remaining.is_empty() {
            hook_ctx
                .delete_by_id::<StorageObject>(&cache.storage_object_id)
                .await
                .map_err(Self::db_error)?;
        }
        Ok(())
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
                    UpdateCastSessionInput {
                        player_state: Some("IDLE".to_string()),
                        ended_at: Some(Some(now.clone())),
                        last_position: Some(Some(old_session.current_position)),
                        ..UpdateCastSessionInput::default()
                    },
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
            hook_ctx
                .update_by_id::<CastSession>(
                    &session.id,
                    UpdateCastSessionInput {
                        device_id: Some(None),
                        player_state: if session.ended_at.is_none() {
                            Some("IDLE".to_string())
                        } else {
                            None
                        },
                        ended_at: if session.ended_at.is_none() {
                            Some(Some(now.clone()))
                        } else {
                            None
                        },
                        last_position: if session.ended_at.is_none() {
                            Some(Some(session.current_position))
                        } else {
                            None
                        },
                        ..UpdateCastSessionInput::default()
                    },
                )
                .await
                .map_err(Self::db_error)?;
        }
        Ok(())
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
            match (&event.phase, &event.action, event.entity_name) {
                (MutationPhase::Before, ChangeAction::Deleted, "Library") => {
                    Self::cleanup_library_children(hook_ctx, &event.id).await?;
                }
                (MutationPhase::After, ChangeAction::Updated, "User") => {
                    Self::revoke_user_sessions_if_needed(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Updated, "MediaFile") => {
                    Self::sync_media_file_link_update(hook_ctx, event).await?;
                }
                (MutationPhase::Before, ChangeAction::Deleted, "MediaFile") => {
                    Self::clear_deleted_media_file_links(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Updated, "Movie") => {
                    Self::sync_movie_media_file_update(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Updated, "Episode") => {
                    Self::sync_episode_media_file_update(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Updated, "Track") => {
                    Self::sync_track_media_file_update(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Updated, "Chapter") => {
                    Self::sync_chapter_media_file_update(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Deleted, "StorageObject") => {
                    self.defer_storage_object_delete(ctx, hook_ctx, event)?;
                }
                (MutationPhase::After, ChangeAction::Deleted, "ArtworkCache") => {
                    Self::cleanup_deleted_artwork_cache(hook_ctx, event).await?;
                }
                (MutationPhase::After, ChangeAction::Created, "CastSession") => {
                    Self::end_other_active_cast_sessions(hook_ctx, event).await?;
                }
                (MutationPhase::Before, ChangeAction::Deleted, "CastDevice") => {
                    Self::detach_cast_device_sessions(hook_ctx, event).await?;
                }
                (MutationPhase::Before, ChangeAction::Deleted, "Torrent") => {
                    Self::cleanup_torrent_files(hook_ctx, event).await?;
                }
                _ => {}
            }

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
                save_path: "/tmp".to_string(),
                download_path: None,
                source_url: None,
                source_feed_id: None,
                source_indexer_id: None,
                library_id: None,
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
