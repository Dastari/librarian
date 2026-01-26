use async_graphql::{Result, SimpleObject};
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity,
    GraphQLRelations,
    GraphQLOperations,
    SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(name = "IndexerConfig")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "indexer_configs",
    plural = "IndexerConfigs",
    default_sort = "name"
)]
pub struct IndexerConfig {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "IndexerType")]
    #[filterable(type = "string")]
    #[sortable]
    pub indexer_type: String,

    #[graphql(name = "DefinitionId")]
    #[filterable(type = "string")]
    pub definition_id: Option<String>,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "Enabled")]
    #[filterable(type = "boolean")]
    pub enabled: bool,

    #[graphql(name = "Priority")]
    #[filterable(type = "number")]
    #[sortable]
    pub priority: i32,

    #[graphql(name = "PostDownloadAction")]
    #[filterable(type = "string")]
    pub post_download_action: Option<String>,

    #[graphql(name = "SiteUrl")]
    #[filterable(type = "string")]
    pub site_url: Option<String>,

    #[graphql(name = "SupportsSearch")]
    #[filterable(type = "boolean")]
    pub supports_search: bool,

    #[graphql(name = "SupportsTvSearch")]
    #[filterable(type = "boolean")]
    pub supports_tv_search: bool,

    #[graphql(name = "SupportsMovieSearch")]
    #[filterable(type = "boolean")]
    pub supports_movie_search: bool,

    #[graphql(name = "SupportsMusicSearch")]
    #[filterable(type = "boolean")]
    pub supports_music_search: bool,

    #[graphql(name = "SupportsBookSearch")]
    #[filterable(type = "boolean")]
    pub supports_book_search: bool,

    #[graphql(name = "SupportsImdbSearch")]
    #[filterable(type = "boolean")]
    pub supports_imdb_search: bool,

    #[graphql(name = "SupportsTvdbSearch")]
    #[filterable(type = "boolean")]
    pub supports_tvdb_search: bool,

    #[graphql(name = "Capabilities")]
    pub capabilities: Option<String>,

    #[graphql(name = "LastError")]
    pub last_error: Option<String>,

    #[graphql(name = "ErrorCount")]
    #[filterable(type = "number")]
    pub error_count: i32,

    #[graphql(name = "LastSuccessAt")]
    #[filterable(type = "date")]
    pub last_success_at: Option<String>,

    #[graphql(name = "LastErrorAt")]
    #[filterable(type = "date")]
    pub last_error_at: Option<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "CredentialType")]
    #[filterable(type = "string")]
    pub credential_type: String,

    #[graphql(name = "CredentialValue")]
    pub credential_value: String,

    #[graphql(name = "CredentialNonce")]
    pub credential_nonce: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

#[derive(Default)]
pub struct IndexerConfigCustomOperations;

// // ============================================================================
// // ComplexObject Resolvers (computed fields)
// // ============================================================================

// #[async_graphql::ComplexObject]
// impl IndexerConfigEntity {
//     /// Whether this indexer is considered healthy (no errors)
//     #[graphql(name = "IsHealthy")]
//     async fn is_healthy(&self) -> bool {
//         self.error_count == 0
//     }
// }

// // =============================================================================
// // Custom Indexer Operations (Encryption, External Searches, Testing)
// // =============================================================================

// fn format_bytes(bytes: u64) -> String {
//     const KB: u64 = 1024;
//     const MB: u64 = KB * 1024;
//     const GB: u64 = MB * 1024;

//     if bytes >= GB { format!("{:.2} GB", bytes as f64 / GB as f64) }
//     else if bytes >= MB { format!("{:.2} MB", bytes as f64 / MB as f64) }
//     else if bytes >= KB { format!("{:.2} KB", bytes as f64 / KB as f64) }
//     else { format!("{} B", bytes) }
// }

// /// Custom operations for indexers (credential management, search, testing)
// #[derive(Default)]
// pub struct IndexerCustomOperations;

// #[Object]
// impl IndexerCustomOperations {
//     // =========================================================================
//     // Queries
//     // =========================================================================

//     /// Get available indexer types
//     #[graphql(name = "AvailableIndexerTypes")]
//     async fn available_indexer_types(&self, ctx: &Context<'_>) -> Result<Vec<IndexerTypeInfo>> {
//         let _user = ctx.auth_user()?;
//         use crate::indexer::definitions::get_available_indexers;

//         let types = get_available_indexers()
//             .iter()
//             .map(|info| IndexerTypeInfo {
//                 id: info.id.to_string(),
//                 name: info.name.to_string(),
//                 description: info.description.to_string(),
//                 tracker_type: info.tracker_type.to_string(),
//                 language: info.language.to_string(),
//                 site_link: info.site_link.to_string(),
//                 required_credentials: info.required_credentials.iter().map(|s| s.to_string()).collect(),
//                 is_native: info.is_native,
//             })
//             .collect();

//         Ok(types)
//     }

//     /// Get setting definitions for an indexer type
//     #[graphql(name = "IndexerSettingDefinitions")]
//     async fn indexer_setting_definitions(
//         &self,
//         ctx: &Context<'_>,
//         #[graphql(name = "IndexerType")] indexer_type: String,
//     ) -> Result<Vec<IndexerSettingDefinition>> {
//         let _user = ctx.auth_user()?;
//         use crate::indexer::definitions::{SettingType, get_indexer_info};

//         let info = get_indexer_info(&indexer_type)
//             .ok_or_else(|| async_graphql::Error::new(format!("Unknown indexer type: {}", indexer_type)))?;

//         let settings = info
//             .optional_settings
//             .iter()
//             .map(|s| IndexerSettingDefinition {
//                 key: s.key.to_string(),
//                 label: s.label.to_string(),
//                 setting_type: match s.setting_type {
//                     SettingType::Text => "text".to_string(),
//                     SettingType::Password => "password".to_string(),
//                     SettingType::Checkbox => "checkbox".to_string(),
//                     SettingType::Select => "select".to_string(),
//                 },
//                 default_value: s.default_value.map(|s| s.to_string()),
//                 options: s.options.map(|opts| {
//                     opts.iter()
//                         .map(|(value, label)| IndexerSettingOption {
//                             value: value.to_string(),
//                             label: label.to_string(),
//                         })
//                         .collect()
//                 }),
//             })
//             .collect();

//         Ok(settings)
//     }

//     /// Search indexers for torrents
//     #[graphql(name = "SearchIndexers")]
//     async fn search_indexers(
//         &self,
//         ctx: &Context<'_>,
//         input: IndexerSearchInput,
//     ) -> Result<IndexerSearchResultSet> {
//         let user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();

//         let start = std::time::Instant::now();

//         let indexer_ids: Option<Vec<String>> = input.indexer_ids.clone();

//         // Get enabled indexers for user
//         let all_configs = EntityQuery::<IndexerConfigEntity>::new()
//             .filter(&IndexerConfigEntityWhereInput {
//                 user_id: Some(StringFilter::eq(&user.user_id)),
//                 enabled: Some(BoolFilter::is_true()),
//                 ..Default::default()
//             })
//             .fetch_all(db.pool())
//             .await?;

//         let configs: Vec<IndexerConfigEntity> = if let Some(ref ids) = indexer_ids {
//             all_configs.into_iter().filter(|c| ids.contains(&c.id)).collect()
//         } else {
//             all_configs
//         };

//         let encryption_key = get_or_create_encryption_key(db.pool()).await?;
//         let encryption = crate::indexer::encryption::CredentialEncryption::from_base64_key(&encryption_key)?;

//         use crate::indexer::{QueryType, TorznabQuery};
//         let query = TorznabQuery {
//             query_type: if input.season.is_some() || input.episode.is_some() {
//                 QueryType::TvSearch
//             } else if input.imdb_id.is_some() {
//                 QueryType::MovieSearch
//             } else {
//                 QueryType::Search
//             },
//             search_term: Some(input.query.clone()),
//             categories: input.categories.unwrap_or_default(),
//             season: input.season,
//             episode: input.episode,
//             imdb_id: input.imdb_id,
//             limit: input.limit,
//             cache: true,
//             ..Default::default()
//         };

//         let mut results: Vec<IndexerSearchResultItem> = Vec::new();
//         let mut total_releases = 0;
//         let configs_len = configs.len();

//         for config in configs {
//             let indexer_start = std::time::Instant::now();

//             // Get credentials
//             let credentials = EntityQuery::<IndexerCredentialEntity>::new()
//                 .filter(&IndexerCredentialEntityWhereInput {
//                     indexer_config_id: Some(StringFilter::eq(&config.id)),
//                     ..Default::default()
//                 })
//                 .fetch_all(db.pool())
//                 .await
//                 .unwrap_or_default();

//             // Get settings
//             let settings = EntityQuery::<IndexerSettingEntity>::new()
//                 .filter(&IndexerSettingEntityWhereInput {
//                     indexer_config_id: Some(StringFilter::eq(&config.id)),
//                     ..Default::default()
//                 })
//                 .fetch_all(db.pool())
//                 .await
//                 .unwrap_or_default();

//             let mut decrypted_creds: std::collections::HashMap<String, String> = std::collections::HashMap::new();
//             for cred in credentials {
//                 if let Ok(value) = encryption.decrypt(&cred.encrypted_value, &cred.nonce) {
//                     decrypted_creds.insert(cred.credential_type, value);
//                 }
//             }

//             let settings_map: std::collections::HashMap<String, String> = settings
//                 .into_iter()
//                 .map(|s| (s.setting_key, s.setting_value))
//                 .collect();

//             match config.indexer_type.as_str() {
//                 "iptorrents" => {
//                     use crate::indexer::Indexer;
//                     use crate::indexer::definitions::iptorrents::IPTorrentsIndexer;

//                     let cookie = decrypted_creds.get("cookie").cloned().unwrap_or_default();
//                     let user_agent = decrypted_creds.get("user_agent").cloned().unwrap_or_default();

//                     let indexer = match IPTorrentsIndexer::new(
//                         config.id.clone(),
//                         config.name.clone(),
//                         config.site_url.clone(),
//                         &cookie,
//                         &user_agent,
//                         settings_map,
//                     ) {
//                         Ok(idx) => idx,
//                         Err(e) => {
//                             results.push(IndexerSearchResultItem {
//                                 indexer_id: config.id.clone(),
//                                 indexer_name: config.name,
//                                 releases: vec![],
//                                 elapsed_ms: indexer_start.elapsed().as_millis() as i64,
//                                 from_cache: false,
//                                 error: Some(format!("Failed to create indexer: {}", e)),
//                             });
//                             continue;
//                         }
//                     };

//                     match indexer.search(&query).await {
//                         Ok(releases) => {
//                             self.record_success(db, &config.id).await.ok();
//                             let torrent_releases: Vec<TorrentRelease> = releases
//                                 .iter()
//                                 .map(|r| TorrentRelease {
//                                     title: r.title.clone(),
//                                     guid: r.guid.clone(),
//                                     link: r.link.clone(),
//                                     magnet_uri: r.magnet_uri.clone(),
//                                     info_hash: r.info_hash.clone(),
//                                     details: r.details.clone(),
//                                     publish_date: r.publish_date.to_rfc3339(),
//                                     categories: r.categories.clone(),
//                                     size: r.size,
//                                     size_formatted: r.size.map(|s| format_bytes(s as u64)),
//                                     seeders: r.seeders,
//                                     leechers: r.leechers(),
//                                     peers: r.peers,
//                                     grabs: r.grabs,
//                                     is_freeleech: r.is_freeleech(),
//                                     imdb_id: r.imdb.map(|id| format!("tt{:07}", id)),
//                                     poster: r.poster.clone(),
//                                     description: r.description.clone(),
//                                     indexer_id: Some(config.id.clone()),
//                                     indexer_name: Some(config.name.clone()),
//                                 })
//                                 .collect();

//                             total_releases += torrent_releases.len() as i32;

//                             results.push(IndexerSearchResultItem {
//                                 indexer_id: config.id,
//                                 indexer_name: config.name,
//                                 releases: torrent_releases,
//                                 elapsed_ms: indexer_start.elapsed().as_millis() as i64,
//                                 from_cache: false,
//                                 error: None,
//                             });
//                         }
//                         Err(e) => {
//                             self.record_error(db, &config.id, &e.to_string()).await.ok();
//                             results.push(IndexerSearchResultItem {
//                                 indexer_id: config.id,
//                                 indexer_name: config.name,
//                                 releases: vec![],
//                                 elapsed_ms: indexer_start.elapsed().as_millis() as i64,
//                                 from_cache: false,
//                                 error: Some(e.to_string()),
//                             });
//                         }
//                     }
//                 }
//                 _ => {
//                     results.push(IndexerSearchResultItem {
//                         indexer_id: config.id,
//                         indexer_name: config.name,
//                         releases: vec![],
//                         elapsed_ms: indexer_start.elapsed().as_millis() as i64,
//                         from_cache: false,
//                         error: Some(format!("Unsupported indexer type: {}", config.indexer_type)),
//                     });
//                 }
//             }
//         }

//         Ok(IndexerSearchResultSet {
//             indexers: results,
//             total_releases,
//             total_elapsed_ms: start.elapsed().as_millis() as i64,
//             stopped_early: false,
//             sources_searched: configs_len as i32,
//             priority_rule_used: None,
//         })
//     }

//     // =========================================================================
//     // Mutations
//     // =========================================================================
//     //
//     // NOTE: These CRUD operations are NOT auto-generated because they involve:
//     // 1. Credential encryption (AES-256-GCM) for sensitive data
//     // 2. Multi-table operations (indexer_configs, indexer_credentials, indexer_settings)
//     // 3. Complex business logic for indexer validation
//     // These remain as custom operations rather than using GraphQLOperations-generated mutations.

//     /// Create a new indexer with encrypted credentials
//     #[graphql(name = "CreateIndexer")]
//     async fn create_indexer(&self, ctx: &Context<'_>, input: CreateIndexerInput) -> Result<IndexerResult> {
//         let user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();

//         use crate::indexer::definitions::get_indexer_info;
//         if get_indexer_info(&input.indexer_type).is_none() {
//             return Ok(IndexerResult {
//                 success: false,
//                 error: Some(format!("Unknown indexer type: {}", input.indexer_type)),
//                 indexer: None,
//             });
//         }

//         // Create indexer config
//         let id = Uuid::new_v4().to_string();
//         sqlx::query(
//             r#"
//             INSERT INTO indexer_configs (
//                 id, user_id, indexer_type, name, site_url, enabled, priority,
//                 supports_search, supports_tv_search, supports_movie_search,
//                 supports_music_search, supports_book_search, supports_imdb_search, supports_tvdb_search,
//                 error_count, created_at, updated_at
//             ) VALUES (
//                 ?1, ?2, ?3, ?4, ?5, 1, 50,
//                 1, 1, 1, 0, 0, 0, 0,
//                 0, datetime('now'), datetime('now')
//             )
//             "#,
//         )
//         .bind(&id)
//         .bind(&user.user_id)
//         .bind(&input.indexer_type)
//         .bind(&input.name)
//         .bind(&input.site_url)
//         .execute(db.pool())
//         .await?;

//         // Encrypt and save credentials
//         let encryption_key = get_or_create_encryption_key(db.pool()).await?;
//         let encryption = crate::indexer::encryption::CredentialEncryption::from_base64_key(&encryption_key)?;

//         for cred in input.credentials {
//             let (encrypted_value, nonce) = encryption.encrypt(&cred.value)?;
//             let cred_id = Uuid::new_v4().to_string();
//             sqlx::query(
//                 r#"
//                 INSERT INTO indexer_credentials (id, indexer_config_id, credential_type, encrypted_value, nonce, created_at, updated_at)
//                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))
//                 ON CONFLICT(indexer_config_id, credential_type) DO UPDATE SET
//                     encrypted_value = ?4, nonce = ?5, updated_at = datetime('now')
//                 "#,
//             )
//             .bind(&cred_id)
//             .bind(&id)
//             .bind(&cred.credential_type)
//             .bind(&encrypted_value)
//             .bind(&nonce)
//             .execute(db.pool())
//             .await?;
//         }

//         // Save settings
//         for setting in input.settings {
//             let setting_id = Uuid::new_v4().to_string();
//             sqlx::query(
//                 r#"
//                 INSERT INTO indexer_settings (id, indexer_config_id, setting_key, setting_value, created_at, updated_at)
//                 VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))
//                 ON CONFLICT(indexer_config_id, setting_key) DO UPDATE SET
//                     setting_value = ?4, updated_at = datetime('now')
//                 "#,
//             )
//             .bind(&setting_id)
//             .bind(&id)
//             .bind(&setting.key)
//             .bind(&setting.value)
//             .execute(db.pool())
//             .await?;
//         }

//         // Fetch created record
//         let record = EntityQuery::<IndexerConfigEntity>::new()
//             .filter(&IndexerConfigEntityWhereInput {
//                 id: Some(StringFilter::eq(&id)),
//                 ..Default::default()
//             })
//             .fetch_one(db.pool())
//             .await?
//             .ok_or_else(|| async_graphql::Error::new("Failed to fetch created indexer"))?;

//         Ok(IndexerResult {
//             success: true,
//             error: None,
//             indexer: Some(IndexerConfig {
//                 id: record.id,
//                 indexer_type: record.indexer_type,
//                 name: record.name,
//                 enabled: record.enabled,
//                 priority: record.priority,
//                 site_url: record.site_url,
//                 is_healthy: true,
//                 last_error: None,
//                 error_count: 0,
//                 last_success_at: None,
//                 created_at: record.created_at,
//                 updated_at: record.updated_at,
//                 capabilities: IndexerCapabilities {
//                     supports_search: true,
//                     supports_tv_search: true,
//                     supports_movie_search: true,
//                     supports_music_search: false,
//                     supports_book_search: false,
//                     supports_imdb_search: false,
//                     supports_tvdb_search: false,
//                 },
//             }),
//         })
//     }

//     /// Update an existing indexer
//     #[graphql(name = "UpdateIndexer")]
//     async fn update_indexer(
//         &self,
//         ctx: &Context<'_>,
//         id: String,
//         input: UpdateIndexerInput,
//     ) -> Result<IndexerResult> {
//         let user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();

//         // Check ownership
//         let existing = EntityQuery::<IndexerConfigEntity>::new()
//             .filter(&IndexerConfigEntityWhereInput {
//                 id: Some(StringFilter::eq(&id)),
//                 user_id: Some(StringFilter::eq(&user.user_id)),
//                 ..Default::default()
//             })
//             .fetch_one(db.pool())
//             .await?;

//         if existing.is_none() {
//             return Ok(IndexerResult { success: false, error: Some("Indexer not found".to_string()), indexer: None });
//         }

//         // Build dynamic update
//         let mut updates = vec!["updated_at = datetime('now')"];
//         let mut values: Vec<SqlValue> = Vec::new();

//         if let Some(ref name) = input.name {
//             updates.push("name = ?");
//             values.push(SqlValue::String(name.clone()));
//         }
//         if let Some(enabled) = input.enabled {
//             updates.push("enabled = ?");
//             values.push(SqlValue::Bool(enabled));
//         }
//         if let Some(priority) = input.priority {
//             updates.push("priority = ?");
//             values.push(SqlValue::Int(priority as i64));
//         }
//         if let Some(ref site_url) = input.site_url {
//             updates.push("site_url = ?");
//             values.push(SqlValue::String(site_url.clone()));
//         }

//         // Add ID for WHERE clause
//         values.push(SqlValue::String(id.clone()));

//         let sql = format!("UPDATE indexer_configs SET {} WHERE id = ?", updates.join(", "));
//         execute_with_binds(&sql, &values, db.pool()).await?;

//         // Update credentials if provided
//         if let Some(credentials) = input.credentials {
//             let encryption_key = get_or_create_encryption_key(db.pool()).await?;
//             let encryption = crate::indexer::encryption::CredentialEncryption::from_base64_key(&encryption_key)?;

//             for cred in credentials {
//                 let (encrypted_value, nonce) = encryption.encrypt(&cred.value)?;
//                 let cred_id = Uuid::new_v4().to_string();
//                 sqlx::query(
//                     r#"
//                     INSERT INTO indexer_credentials (id, indexer_config_id, credential_type, encrypted_value, nonce, created_at, updated_at)
//                     VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))
//                     ON CONFLICT(indexer_config_id, credential_type) DO UPDATE SET
//                         encrypted_value = ?4, nonce = ?5, updated_at = datetime('now')
//                     "#,
//                 )
//                 .bind(&cred_id)
//                 .bind(&id)
//                 .bind(&cred.credential_type)
//                 .bind(&encrypted_value)
//                 .bind(&nonce)
//                 .execute(db.pool())
//                 .await?;
//             }
//         }

//         // Update settings if provided
//         if let Some(settings) = input.settings {
//             for setting in settings {
//                 let setting_id = Uuid::new_v4().to_string();
//                 sqlx::query(
//                     r#"
//                     INSERT INTO indexer_settings (id, indexer_config_id, setting_key, setting_value, created_at, updated_at)
//                     VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))
//                     ON CONFLICT(indexer_config_id, setting_key) DO UPDATE SET
//                         setting_value = ?4, updated_at = datetime('now')
//                     "#,
//                 )
//                 .bind(&setting_id)
//                 .bind(&id)
//                 .bind(&setting.key)
//                 .bind(&setting.value)
//                 .execute(db.pool())
//                 .await?;
//             }
//         }

//         // Fetch updated record
//         let record = EntityQuery::<IndexerConfigEntity>::new()
//             .filter(&IndexerConfigEntityWhereInput {
//                 id: Some(StringFilter::eq(&id)),
//                 ..Default::default()
//             })
//             .fetch_one(db.pool())
//             .await?
//             .ok_or_else(|| async_graphql::Error::new("Indexer not found after update"))?;

//         Ok(IndexerResult {
//             success: true,
//             error: None,
//             indexer: Some(IndexerConfig {
//                 id: record.id,
//                 indexer_type: record.indexer_type,
//                 name: record.name,
//                 enabled: record.enabled,
//                 priority: record.priority,
//                 site_url: record.site_url,
//                 is_healthy: record.error_count == 0,
//                 last_error: record.last_error,
//                 error_count: record.error_count,
//                 last_success_at: record.last_success_at,
//                 created_at: record.created_at,
//                 updated_at: record.updated_at,
//                 capabilities: IndexerCapabilities {
//                     supports_search: record.supports_search,
//                     supports_tv_search: record.supports_tv_search,
//                     supports_movie_search: record.supports_movie_search,
//                     supports_music_search: record.supports_music_search,
//                     supports_book_search: record.supports_book_search,
//                     supports_imdb_search: record.supports_imdb_search,
//                     supports_tvdb_search: record.supports_tvdb_search,
//                 },
//             }),
//         })
//     }

//     /// Delete an indexer
//     #[graphql(name = "DeleteIndexer")]
//     async fn delete_indexer(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
//         let user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();

//         // Check ownership
//         let existing = EntityQuery::<IndexerConfigEntity>::new()
//             .filter(&IndexerConfigEntityWhereInput {
//                 id: Some(StringFilter::eq(&id)),
//                 user_id: Some(StringFilter::eq(&user.user_id)),
//                 ..Default::default()
//             })
//             .fetch_one(db.pool())
//             .await?;

//         if existing.is_none() {
//             return Ok(MutationResult { success: false, error: Some("Indexer not found".to_string()) });
//         }

//         // Delete credentials and settings first (cascade)
//         execute_with_binds(
//             "DELETE FROM indexer_credentials WHERE indexer_config_id = ?",
//             &[SqlValue::String(id.clone())],
//             db.pool(),
//         ).await?;

//         execute_with_binds(
//             "DELETE FROM indexer_settings WHERE indexer_config_id = ?",
//             &[SqlValue::String(id.clone())],
//             db.pool(),
//         ).await?;

//         // Delete config
//         let result = execute_with_binds(
//             "DELETE FROM indexer_configs WHERE id = ?",
//             &[SqlValue::String(id)],
//             db.pool(),
//         ).await?;

//         Ok(MutationResult {
//             success: result.rows_affected() > 0,
//             error: if result.rows_affected() > 0 { None } else { Some("Failed to delete indexer".to_string()) },
//         })
//     }

//     /// Test an indexer connection
//     #[graphql(name = "TestIndexer")]
//     async fn test_indexer(&self, ctx: &Context<'_>, id: String) -> Result<IndexerTestResult> {
//         let user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();

//         let config = EntityQuery::<IndexerConfigEntity>::new()
//             .filter(&IndexerConfigEntityWhereInput {
//                 id: Some(StringFilter::eq(&id)),
//                 user_id: Some(StringFilter::eq(&user.user_id)),
//                 ..Default::default()
//             })
//             .fetch_one(db.pool())
//             .await?
//             .ok_or_else(|| async_graphql::Error::new("Indexer not found"))?;

//         let credentials = EntityQuery::<IndexerCredentialEntity>::new()
//             .filter(&IndexerCredentialEntityWhereInput {
//                 indexer_config_id: Some(StringFilter::eq(&id)),
//                 ..Default::default()
//             })
//             .fetch_all(db.pool())
//             .await?;

//         let settings = EntityQuery::<IndexerSettingEntity>::new()
//             .filter(&IndexerSettingEntityWhereInput {
//                 indexer_config_id: Some(StringFilter::eq(&id)),
//                 ..Default::default()
//             })
//             .fetch_all(db.pool())
//             .await?;

//         let encryption_key = get_or_create_encryption_key(db.pool()).await?;
//         let encryption = crate::indexer::encryption::CredentialEncryption::from_base64_key(&encryption_key)?;

//         let mut decrypted_creds: std::collections::HashMap<String, String> = std::collections::HashMap::new();
//         for cred in credentials {
//             if let Ok(value) = encryption.decrypt(&cred.encrypted_value, &cred.nonce) {
//                 decrypted_creds.insert(cred.credential_type, value);
//             }
//         }

//         let settings_map: std::collections::HashMap<String, String> = settings.into_iter().map(|s| (s.setting_key, s.setting_value)).collect();

//         let start = std::time::Instant::now();

//         match config.indexer_type.as_str() {
//             "iptorrents" => {
//                 use crate::indexer::definitions::iptorrents::IPTorrentsIndexer;
//                 use crate::indexer::{Indexer, TorznabQuery};

//                 let cookie = decrypted_creds.get("cookie").cloned().unwrap_or_default();
//                 let user_agent = decrypted_creds.get("user_agent").cloned().unwrap_or_default();

//                 let indexer = match IPTorrentsIndexer::new(id.clone(), config.name.clone(), config.site_url.clone(), &cookie, &user_agent, settings_map) {
//                     Ok(idx) => idx,
//                     Err(e) => return Ok(IndexerTestResult { success: false, error: Some(format!("Failed to create indexer: {}", e)), releases_found: None, elapsed_ms: Some(start.elapsed().as_millis() as i64) }),
//                 };

//                 match indexer.test_connection().await {
//                     Ok(true) => {
//                         let query = TorznabQuery::search("");
//                         match indexer.search(&query).await {
//                             Ok(releases) => {
//                                 self.record_success(db, &id).await.ok();
//                                 Ok(IndexerTestResult { success: true, error: None, releases_found: Some(releases.len() as i32), elapsed_ms: Some(start.elapsed().as_millis() as i64) })
//                             }
//                             Err(e) => {
//                                 self.record_error(db, &id, &e.to_string()).await.ok();
//                                 Ok(IndexerTestResult { success: false, error: Some(format!("Search failed: {}", e)), releases_found: None, elapsed_ms: Some(start.elapsed().as_millis() as i64) })
//                             }
//                         }
//                     }
//                     Ok(false) => {
//                         self.record_error(db, &id, "Connection test failed").await.ok();
//                         Ok(IndexerTestResult { success: false, error: Some("Connection test failed - check your cookie".to_string()), releases_found: None, elapsed_ms: Some(start.elapsed().as_millis() as i64) })
//                     }
//                     Err(e) => {
//                         self.record_error(db, &id, &e.to_string()).await.ok();
//                         Ok(IndexerTestResult { success: false, error: Some(format!("Connection error: {}", e)), releases_found: None, elapsed_ms: Some(start.elapsed().as_millis() as i64) })
//                     }
//                 }
//             }
//             other => Ok(IndexerTestResult { success: false, error: Some(format!("Unsupported indexer type: {}", other)), releases_found: None, elapsed_ms: Some(start.elapsed().as_millis() as i64) }),
//         }
//     }

//     // =========================================================================
//     // Helper methods (moved from standalone functions)
//     // =========================================================================

//     async fn record_success(&self, db: &Database, config_id: &str) -> Result<(), sqlx::Error> {
//         sqlx::query(
//             "UPDATE indexer_configs SET error_count = 0, last_error = NULL, last_success_at = datetime('now'), updated_at = datetime('now') WHERE id = ?"
//         )
//         .bind(config_id)
//         .execute(db.pool())
//         .await?;
//         Ok(())
//     }

//     async fn record_error(&self, db: &Database, config_id: &str, error: &str) -> Result<(), sqlx::Error> {
//         sqlx::query(
//             "UPDATE indexer_configs SET error_count = error_count + 1, last_error = ?, last_error_at = datetime('now'), updated_at = datetime('now') WHERE id = ?"
//         )
//         .bind(error)
//         .bind(config_id)
//         .execute(db.pool())
//         .await?;
//         Ok(())
//     }
// }

// // =============================================================================
// // Helper Functions
// // =============================================================================

// async fn get_or_create_encryption_key(pool: &sqlx::SqlitePool) -> Result<String, async_graphql::Error> {
//     // Try to get existing key
//     let setting = EntityQuery::<AppSettingEntity>::new()
//         .filter(&AppSettingEntityWhereInput {
//             key: Some(StringFilter::eq("indexer_encryption_key")),
//             ..Default::default()
//         })
//         .fetch_one(pool)
//         .await?;

//     if let Some(s) = setting {
//         return Ok(s.value);
//     }

//     // Generate and save new key
//     use base64::Engine;
//     let mut key_bytes = [0u8; 32];
//     getrandom::getrandom(&mut key_bytes).map_err(|e| async_graphql::Error::new(e.to_string()))?;
//     let key_value = base64::engine::general_purpose::STANDARD.encode(key_bytes);

//     let id = Uuid::new_v4().to_string();
//     sqlx::query(
//         "INSERT INTO app_settings (id, key, value, category, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))"
//     )
//     .bind(&id)
//     .bind("indexer_encryption_key")
//     .bind(&key_value)
//     .bind("security")
//     .execute(pool)
//     .await?;

//     Ok(key_value)
// }
