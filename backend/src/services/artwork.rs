//! Artwork caching service
//!
//! Downloads and caches artwork (posters, backdrops) from external URLs (TMDB, etc.)
//! into object storage for fast local serving.
//!
//! This is a utility service (not a lifecycle Service) - no background tasks,
//! just on-demand operations called during metadata operations.

use std::collections::{BTreeSet, HashMap};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use anyhow::{Context, Result};
use graphql_orm::graphql::filters::{IntFilter, StringFilter};
use graphql_orm_storage::{StorageNamespace, StoragePutRequest, StoredObject};
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, LOCATION};
use reqwest::{StatusCode, Url};
use thiserror::Error;
use tokio::net::lookup_host;
use tokio::time::timeout;
use tracing::{debug, info, warn};
use url::Host;

use crate::db::Database;
use crate::graphql::entities::{
    Album, AppSetting, AppSettingWhereInput, Artist, ArtworkCache, ArtworkCacheWhereInput,
    Audiobook, Collection, CollectionWhereInput, CreateArtworkCacheInput, CreateStorageObjectInput,
    Episode, Movie, Person, ScheduleCache, ScheduleCacheWhereInput, Season, Show, StorageObject,
    UpdateArtworkCacheInput,
};
use crate::services::ObjectStorageService;

const DEFAULT_ARTWORK_HOSTS: &[&str] = &[
    "image.tmdb.org",
    "static.tvmaze.com",
    "covers.openlibrary.org",
    "coverartarchive.org",
    "archive.org",
];
const ARTWORK_ALLOWED_HOSTS_SETTING: &str = "artwork_allowed_hosts";
const MAX_REDIRECTS: usize = 3;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(12);
const BODY_IDLE_TIMEOUT: Duration = Duration::from_secs(5);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(30);

/// Widths a client may ask for. Requests snap up to the next step so a handful of variants
/// serve every layout (grid posters, shelf tiles, episode stills, full-screen backdrops).
pub const ARTWORK_WIDTHS: &[u32] = &[185, 342, 500, 780, 1280, 1920];
const VARIANT_JPEG_QUALITY: u8 = 84;

/// One download per artwork at a time: concurrent requests for the same uncached image
/// (a grid of tiles, two devices) wait for the first instead of fetching it again.
static IN_FLIGHT: LazyLock<std::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    LazyLock::new(Default::default);

struct InFlightGuard {
    key: String,
    lock: Arc<tokio::sync::Mutex<()>>,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        if let Ok(mut map) = IN_FLIGHT.lock() {
            // Only this guard and the map hold the lock: nobody else is waiting.
            if Arc::strong_count(&self.lock) <= 2 {
                map.remove(&self.key);
            }
        }
    }
}

async fn lock_artwork(key: String) -> (InFlightGuard, tokio::sync::OwnedMutexGuard<()>) {
    let lock = {
        let mut map = IN_FLIGHT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        map.entry(key.clone()).or_default().clone()
    };
    let held = lock.clone().lock_owned().await;
    (InFlightGuard { key, lock }, held)
}

/// Snap a requested width to the next supported step (`None` when no resize is wanted).
pub fn normalize_artwork_width(requested: Option<u32>) -> Option<u32> {
    let width = requested.filter(|width| *width > 0)?;
    Some(
        ARTWORK_WIDTHS
            .iter()
            .copied()
            .find(|step| *step >= width)
            .unwrap_or(*ARTWORK_WIDTHS.last().unwrap_or(&1920)),
    )
}

fn variant_type(artwork_type: &str, width: u32) -> String {
    format!("{artwork_type}@w{width}")
}

#[derive(Debug, Error)]
pub enum ArtworkFetchError {
    #[error("DESTINATION_DENIED: artwork destination is not permitted")]
    DestinationDenied,
    #[error("REDIRECT_DENIED: artwork redirect destination is not permitted")]
    RedirectDenied,
    #[error("TIMEOUT: artwork request exceeded its deadline")]
    Timeout,
    #[error("TOO_LARGE: artwork response exceeds the 20 MiB limit")]
    TooLarge,
    #[error("INVALID_IMAGE: artwork response is not a decodable supported image")]
    InvalidImage,
    #[error("UNSUPPORTED_CONTENT_TYPE: artwork response content type is not supported")]
    UnsupportedContentType,
    #[error("HTTP_STATUS: artwork server returned HTTP {0}")]
    HttpStatus(StatusCode),
    #[error("NETWORK_ERROR: artwork request failed")]
    Network,
}

#[derive(Debug)]
struct DownloadedArtwork {
    data: Vec<u8>,
    mime_type: String,
    width: i32,
    height: i32,
    final_host: String,
}

/// Artwork service for caching images from external URLs
#[derive(Clone)]
pub struct ArtworkService {
    db: Database,
    storage: Arc<ObjectStorageService>,
}

impl ArtworkService {
    const MAX_ARTWORK_BYTES: u64 = 20 * 1024 * 1024;

    /// Create a new artwork service
    pub fn new(db: Database, storage: Arc<ObjectStorageService>) -> Self {
        Self { db, storage }
    }

    /// Cache an image from a source URL
    ///
    /// Downloads the image, stores bytes in object storage, persists metadata,
    /// and returns the internal URL for serving.
    ///
    /// Returns the cached URL: `/api/artwork/{entity_type}/{entity_id}/{artwork_type}`
    pub async fn cache_image(
        &self,
        source_url: &str,
        entity_type: &str,
        entity_id: &str,
        artwork_type: &str,
    ) -> Result<String> {
        let source_host = Url::parse(source_url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .unwrap_or_else(|| "invalid".to_string());
        debug!(
            source_host = %source_host,
            entity_type = %entity_type,
            entity_id = %entity_id,
            artwork_type = %artwork_type,
            "Caching artwork from URL"
        );

        let _lock = lock_artwork(format!("{entity_type}/{entity_id}/{artwork_type}")).await;

        // Already cached from this source: nothing to do. A different source (the
        // provider changed its artwork) downloads the new image and replaces the old one.
        let existing = self
            .get_cached_entry(entity_type, entity_id, artwork_type)
            .await?;
        if let Some(existing) = &existing
            && (existing.source_url.as_deref() == Some(source_url) || existing.source_url.is_none())
        {
            debug!(
                entity_type = %entity_type,
                entity_id = %entity_id,
                artwork_type = %artwork_type,
                "Artwork already cached, skipping download"
            );
            return Ok(format!(
                "/api/artwork/{}/{}/{}",
                entity_type, entity_id, artwork_type
            ));
        }

        let allowed_hosts = self.allowed_artwork_hosts().await?;
        let downloaded = timeout(
            TOTAL_TIMEOUT,
            self.download_image(source_url, &allowed_hosts),
        )
        .await
        .map_err(|_| ArtworkFetchError::Timeout)??;
        let DownloadedArtwork {
            data,
            mime_type,
            width,
            height,
            final_host,
        } = downloaded;
        let size_bytes = data.len() as i64;

        let stored = self
            .storage
            .storage()
            .put_object(StoragePutRequest {
                namespace: StorageNamespace::Derivatives,
                file_name: source_url_file_name(source_url),
                mime_type: Some(mime_type.clone()),
                bytes: data,
            })
            .await
            .with_context(|| {
                format!(
                    "Failed to store artwork object for {entity_type}:{entity_id}:{artwork_type}"
                )
            })?;

        let storage_row = match self.insert_storage_object(&stored).await {
            Ok(row) => row,
            Err(err) => {
                self.cleanup_object_after_db_failure(&stored, "storage object insert")
                    .await;
                return Err(err).context("Failed to insert artwork storage object metadata");
            }
        };

        if let Some(existing) = existing {
            if let Err(err) = ArtworkCache::update_by_id(
                &self.db,
                &existing.id,
                UpdateArtworkCacheInput {
                    storage_object_id: Some(storage_row.id.clone()),
                    source_url: Some(Some(source_url.to_string())),
                    width: Some(Some(width)),
                    height: Some(Some(height)),
                    ..Default::default()
                },
            )
            .await
            {
                // Deleting the row defers its bytes through the storage hook; only an orphan
                // without metadata needs direct cleanup.
                if StorageObject::delete_by_id(&self.db, &storage_row.id)
                    .await
                    .is_err()
                {
                    self.cleanup_object_after_db_failure(&stored, "artwork cache update")
                        .await;
                }
                return Err(err).context("Failed to replace artwork cache metadata");
            }
            self.release_storage_object(&existing.storage_object_id)
                .await;
            self.delete_variants(entity_type, entity_id, artwork_type)
                .await;
            info!(
                entity_type = %entity_type,
                entity_id = %entity_id,
                artwork_type = %artwork_type,
                source_host = %final_host,
                size_kb = size_bytes / 1024,
                "Replaced cached artwork with the provider's new image"
            );
            return Ok(format!(
                "/api/artwork/{entity_type}/{entity_id}/{artwork_type}"
            ));
        }

        if let Err(err) = ArtworkCache::insert(
            &self.db,
            CreateArtworkCacheInput {
                entity_type: entity_type.to_string(),
                entity_id: entity_id.to_string(),
                artwork_type: artwork_type.to_string(),
                storage_object_id: storage_row.id.clone(),
                source_url: Some(source_url.to_string()),
                width: Some(width),
                height: Some(height),
            },
        )
        .await
        {
            if let Err(delete_err) = StorageObject::delete_by_id(&self.db, &storage_row.id).await {
                warn!(
                    storage_object_id = %storage_row.id,
                    error = %delete_err,
                    "Failed to remove orphaned storage object metadata after artwork cache insert failure"
                );
                self.cleanup_object_after_db_failure(&stored, "artwork cache insert")
                    .await;
            }
            return Err(err).context("Failed to insert artwork cache metadata");
        }

        info!(
            entity_type = %entity_type,
            entity_id = %entity_id,
            artwork_type = %artwork_type,
            source_host = %final_host,
            size_kb = size_bytes / 1024,
            storage_key = %stored.storage_key,
            width,
            height,
            "Cached artwork"
        );

        Ok(format!(
            "/api/artwork/{}/{}/{}",
            entity_type, entity_id, artwork_type
        ))
    }

    async fn allowed_artwork_hosts(&self) -> Result<BTreeSet<String>> {
        let mut hosts = DEFAULT_ARTWORK_HOSTS
            .iter()
            .map(|host| host.to_string())
            .collect::<BTreeSet<_>>();
        let settings = AppSetting::query(self.db.pool())
            .filter(AppSettingWhereInput {
                key: Some(StringFilter {
                    eq: Some(ARTWORK_ALLOWED_HOSTS_SETTING.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
            .context("Failed to read artwork host allowlist")?;

        if let Some(setting) = settings.into_iter().next() {
            let configured =
                serde_json::from_str::<Vec<String>>(&setting.value).unwrap_or_else(|_| {
                    setting
                        .value
                        .split([',', '\n'])
                        .map(str::to_string)
                        .collect()
                });
            for host in configured {
                if let Some(host) = normalize_allowed_host(&host) {
                    hosts.insert(host);
                }
            }
        }
        Ok(hosts)
    }

    async fn download_image(
        &self,
        source_url: &str,
        allowed_hosts: &BTreeSet<String>,
    ) -> Result<DownloadedArtwork> {
        let mut current =
            Url::parse(source_url).map_err(|_| ArtworkFetchError::DestinationDenied)?;

        for redirect_count in 0..=MAX_REDIRECTS {
            let destination = resolve_artwork_destination(&current, allowed_hosts)
                .await
                .map_err(|error| {
                    if redirect_count == 0 {
                        error
                    } else {
                        ArtworkFetchError::RedirectDenied
                    }
                })?;
            let client = pinned_artwork_client(&destination.host, destination.socket_addr)?;
            let mut response = timeout(REQUEST_TIMEOUT, client.get(current.clone()).send())
                .await
                .map_err(|_| ArtworkFetchError::Timeout)?
                .map_err(|_| ArtworkFetchError::Network)?;

            if response.status().is_redirection() {
                if redirect_count == MAX_REDIRECTS {
                    return Err(ArtworkFetchError::RedirectDenied.into());
                }
                let location = response
                    .headers()
                    .get(LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or(ArtworkFetchError::RedirectDenied)?;
                current = current
                    .join(location)
                    .map_err(|_| ArtworkFetchError::RedirectDenied)?;
                continue;
            }
            if !response.status().is_success() {
                return Err(ArtworkFetchError::HttpStatus(response.status()).into());
            }

            let mime_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .and_then(normalize_image_mime)
                .ok_or(ArtworkFetchError::UnsupportedContentType)?;
            if response
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .is_some_and(|length| length > Self::MAX_ARTWORK_BYTES)
            {
                return Err(ArtworkFetchError::TooLarge.into());
            }

            let mut data = Vec::new();
            loop {
                let next = timeout(BODY_IDLE_TIMEOUT, response.chunk())
                    .await
                    .map_err(|_| ArtworkFetchError::Timeout)?
                    .map_err(|_| ArtworkFetchError::Network)?;
                let Some(chunk) = next else {
                    break;
                };
                if data.len().saturating_add(chunk.len()) > Self::MAX_ARTWORK_BYTES as usize {
                    return Err(ArtworkFetchError::TooLarge.into());
                }
                data.extend_from_slice(&chunk);
            }

            let format = image::guess_format(&data).map_err(|_| ArtworkFetchError::InvalidImage)?;
            if !mime_matches_format(&mime_type, format) {
                return Err(ArtworkFetchError::InvalidImage.into());
            }
            let image = image::load_from_memory_with_format(&data, format)
                .map_err(|_| ArtworkFetchError::InvalidImage)?;
            let width =
                i32::try_from(image.width()).map_err(|_| ArtworkFetchError::InvalidImage)?;
            let height =
                i32::try_from(image.height()).map_err(|_| ArtworkFetchError::InvalidImage)?;

            return Ok(DownloadedArtwork {
                data,
                mime_type,
                width,
                height,
                final_host: destination.host,
            });
        }

        Err(ArtworkFetchError::RedirectDenied.into())
    }

    /// Check if artwork is cached
    async fn get_cached_entry(
        &self,
        entity_type: &str,
        entity_id: &str,
        artwork_type: &str,
    ) -> Result<Option<ArtworkCache>> {
        let entry = ArtworkCache::query(self.db.pool())
            .filter(ArtworkCacheWhereInput {
                entity_type: Some(StringFilter {
                    eq: Some(entity_type.to_string()),
                    ..Default::default()
                }),
                entity_id: Some(StringFilter {
                    eq: Some(entity_id.to_string()),
                    ..Default::default()
                }),
                artwork_type: Some(StringFilter {
                    eq: Some(artwork_type.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;

        Ok(entry.into_iter().next())
    }

    async fn insert_storage_object(&self, stored: &StoredObject) -> Result<StorageObject> {
        StorageObject::insert(
            &self.db,
            CreateStorageObjectInput {
                object_id: stored.object_id.to_string(),
                namespace: stored.namespace.as_str().to_string(),
                backend: stored.backend.as_str().to_string(),
                storage_key: stored.storage_key.clone(),
                original_file_name: stored.original_file_name.clone(),
                mime_type: stored.mime_type.clone(),
                size_bytes: i64::try_from(stored.size_bytes).unwrap_or(i64::MAX),
                sha256_hex: stored.sha256_hex.clone(),
            },
        )
        .await
        .context("Failed to insert storage object metadata")
    }

    async fn cleanup_object_after_db_failure(&self, stored: &StoredObject, stage: &str) {
        if let Err(delete_err) = self.storage.storage().delete_object(stored).await {
            warn!(
                stage,
                storage_key = %stored.storage_key,
                object_id = %stored.object_id,
                error = %delete_err,
                "Failed to delete artwork object after database insert failure"
            );
        }
    }

    /// The provider URL an entity's artwork is downloaded from, read through the entity
    /// layer. `None` when the entity or its artwork does not exist.
    pub async fn source_url(
        &self,
        entity_type: &str,
        entity_id: &str,
        artwork_type: &str,
    ) -> Result<Option<String>> {
        let pool = self.db.pool();
        let id = entity_id.to_string();
        Ok(match (entity_type, artwork_type) {
            ("movie", "poster") => Movie::get(pool, &id).await?.and_then(|row| row.poster_url),
            ("movie", "backdrop") => Movie::get(pool, &id)
                .await?
                .and_then(|row| row.backdrop_url),
            ("show", "poster") => Show::get(pool, &id).await?.and_then(|row| row.poster_url),
            ("show", "backdrop") => Show::get(pool, &id).await?.and_then(|row| row.backdrop_url),
            ("season", "poster") => Season::get(pool, &id).await?.and_then(|row| row.poster_url),
            ("episode", "thumbnail") => {
                Episode::get(pool, &id).await?.and_then(|row| row.still_url)
            }
            ("album", "cover") => Album::get(pool, &id).await?.and_then(|row| row.cover_url),
            ("artist", "poster") => Artist::get(pool, &id).await?.and_then(|row| row.image_url),
            ("audiobook", "cover") => Audiobook::get(pool, &id)
                .await?
                .and_then(|row| row.cover_url),
            ("person", "profile") => Person::get(pool, &id)
                .await?
                .and_then(|row| row.profile_url),
            // TV guide posters, addressed by TVMaze show id: the guide lists shows that are
            // not in any library, and is rebuilt daily from the same few hundred posters.
            ("guide", "poster") => {
                let Ok(tvmaze_show_id) = entity_id.parse::<i32>() else {
                    return Ok(None);
                };
                ScheduleCache::query(pool)
                    .filter(ScheduleCacheWhereInput {
                        tvmaze_show_id: Some(IntFilter {
                            eq: Some(tvmaze_show_id),
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .limit(1)
                    .fetch_all()
                    .await?
                    .into_iter()
                    .find_map(|row| row.show_poster_url)
            }
            // Collection artwork is addressed by its TMDB collection id, shared by libraries.
            ("collection", "poster" | "backdrop") => {
                let Ok(tmdb_id) = entity_id.parse::<i32>() else {
                    return Ok(None);
                };
                let rows = Collection::query(pool)
                    .filter(CollectionWhereInput {
                        tmdb_collection_id: Some(IntFilter {
                            eq: Some(tmdb_id),
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .fetch_all()
                    .await?;
                rows.into_iter().find_map(|row| {
                    if artwork_type == "poster" {
                        row.poster_url
                    } else {
                        row.backdrop_url
                    }
                })
            }
            _ => None,
        })
    }

    /// Cache-through lookup: the cached entry, downloading it from the entity's source URL
    /// on first use. Artwork is always served from local storage, never hotlinked.
    pub async fn ensure_cached(
        &self,
        entity_type: &str,
        entity_id: &str,
        artwork_type: &str,
    ) -> Result<Option<ArtworkCache>> {
        if let Some(entry) = self
            .get_cached_entry(entity_type, entity_id, artwork_type)
            .await?
        {
            return Ok(Some(entry));
        }
        let Some(source) = self
            .source_url(entity_type, entity_id, artwork_type)
            .await?
        else {
            return Ok(None);
        };
        self.cache_image(&source, entity_type, entity_id, artwork_type)
            .await?;
        self.get_cached_entry(entity_type, entity_id, artwork_type)
            .await
    }

    /// A resized copy of cached artwork at one of `ARTWORK_WIDTHS`, generated once and
    /// stored beside the original. Never upscales: a narrower original is returned as is.
    pub async fn cached_variant(
        &self,
        original: &ArtworkCache,
        width: u32,
    ) -> Result<ArtworkCache> {
        if original
            .width
            .is_some_and(|original_width| original_width <= width as i32)
        {
            return Ok(original.clone());
        }
        let kind = variant_type(&original.artwork_type, width);
        let _lock = lock_artwork(format!(
            "{}/{}/{}",
            original.entity_type, original.entity_id, kind
        ))
        .await;
        if let Some(existing) = self
            .get_cached_entry(&original.entity_type, &original.entity_id, &kind)
            .await?
        {
            return Ok(existing);
        }

        let source_row = StorageObject::get(self.db.pool(), &original.storage_object_id)
            .await?
            .context("Original artwork storage object is missing")?;
        let stored = self.storage.stored_object_from_entity(&source_row)?;
        let bytes = self.storage.storage().get_object(&stored).await?.bytes;
        let (data, mime_type, variant_width, variant_height) =
            tokio::task::spawn_blocking(move || resize_artwork(&bytes, width))
                .await
                .context("Artwork resize task failed")??;

        let stored = self
            .storage
            .storage()
            .put_object(StoragePutRequest {
                namespace: StorageNamespace::Derivatives,
                file_name: Some(format!(
                    "{kind}.{}",
                    if mime_type == "image/png" {
                        "png"
                    } else {
                        "jpg"
                    }
                )),
                mime_type: Some(mime_type),
                bytes: data,
            })
            .await
            .context("Failed to store resized artwork")?;
        let storage_row = match self.insert_storage_object(&stored).await {
            Ok(row) => row,
            Err(err) => {
                self.cleanup_object_after_db_failure(&stored, "variant storage object insert")
                    .await;
                return Err(err);
            }
        };
        match ArtworkCache::insert(
            &self.db,
            CreateArtworkCacheInput {
                entity_type: original.entity_type.clone(),
                entity_id: original.entity_id.clone(),
                artwork_type: kind,
                storage_object_id: storage_row.id.clone(),
                source_url: None,
                width: Some(variant_width),
                height: Some(variant_height),
            },
        )
        .await
        {
            Ok(entry) => Ok(entry),
            Err(err) => {
                if StorageObject::delete_by_id(&self.db, &storage_row.id)
                    .await
                    .is_err()
                {
                    self.cleanup_object_after_db_failure(&stored, "variant cache insert")
                        .await;
                }
                Err(err).context("Failed to insert resized artwork metadata")
            }
        }
    }

    /// Resized copies belong to one original; drop them when it is replaced.
    async fn delete_variants(&self, entity_type: &str, entity_id: &str, artwork_type: &str) {
        let rows = ArtworkCache::query(self.db.pool())
            .filter(ArtworkCacheWhereInput {
                entity_type: Some(StringFilter {
                    eq: Some(entity_type.to_string()),
                    ..Default::default()
                }),
                entity_id: Some(StringFilter {
                    eq: Some(entity_id.to_string()),
                    ..Default::default()
                }),
                artwork_type: Some(StringFilter {
                    starts_with: Some(format!("{artwork_type}@w")),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await;
        let Ok(rows) = rows else {
            warn!(
                entity_type,
                entity_id, artwork_type, "Failed to list resized artwork for removal"
            );
            return;
        };
        for row in rows {
            // The ArtworkCache delete hook releases the storage object and its bytes.
            if let Err(error) = ArtworkCache::delete_by_id(&self.db, &row.id).await {
                warn!(artwork_cache_id = %row.id, error = %error, "Failed to remove resized artwork metadata");
            }
        }
    }

    /// Drop a storage object no artwork references any more. The StorageObject delete hook
    /// removes its bytes after the transaction commits.
    async fn release_storage_object(&self, storage_object_id: &str) {
        if let Err(error) =
            StorageObject::delete_by_id(&self.db, &storage_object_id.to_string()).await
        {
            warn!(storage_object_id, error = %error, "Failed to release replaced artwork object");
        }
    }

    /// Whether the entity an artwork row belongs to still exists. Unknown kinds count as
    /// existing so a sweep never removes artwork it does not understand.
    async fn artwork_owner_exists(&self, entity_type: &str, entity_id: &str) -> Result<bool> {
        let pool = self.db.pool();
        let id = entity_id.to_string();
        Ok(match entity_type {
            "movie" => Movie::get(pool, &id).await?.is_some(),
            "show" => Show::get(pool, &id).await?.is_some(),
            "season" => Season::get(pool, &id).await?.is_some(),
            "episode" => Episode::get(pool, &id).await?.is_some(),
            "album" => Album::get(pool, &id).await?.is_some(),
            "artist" => Artist::get(pool, &id).await?.is_some(),
            "audiobook" => Audiobook::get(pool, &id).await?.is_some(),
            "person" => Person::get(pool, &id).await?.is_some(),
            "collection" => {
                let Ok(tmdb_id) = entity_id.parse::<i32>() else {
                    return Ok(false);
                };
                !Collection::query(pool)
                    .filter(CollectionWhereInput {
                        tmdb_collection_id: Some(IntFilter {
                            eq: Some(tmdb_id),
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .limit(1)
                    .fetch_all()
                    .await?
                    .is_empty()
            }
            "guide" => {
                let Ok(tvmaze_show_id) = entity_id.parse::<i32>() else {
                    return Ok(false);
                };
                !ScheduleCache::query(pool)
                    .filter(ScheduleCacheWhereInput {
                        tvmaze_show_id: Some(IntFilter {
                            eq: Some(tvmaze_show_id),
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .limit(1)
                    .fetch_all()
                    .await?
                    .is_empty()
            }
            _ => true,
        })
    }

    /// Remove cached artwork whose entity is gone: rows left by deletions made before the
    /// delete hook existed, or by paths that bypass it. Storage rows and bytes follow
    /// through the ArtworkCache and StorageObject delete hooks. Returns rows removed.
    pub async fn sweep_orphans(&self) -> Result<usize> {
        let rows = ArtworkCache::query(self.db.pool()).fetch_all().await?;
        let mut owners: HashMap<(String, String), Vec<String>> = HashMap::new();
        for row in rows {
            owners
                .entry((row.entity_type, row.entity_id))
                .or_default()
                .push(row.id);
        }
        let mut removed = 0;
        for ((entity_type, entity_id), ids) in owners {
            if self.artwork_owner_exists(&entity_type, &entity_id).await? {
                continue;
            }
            for id in ids {
                match ArtworkCache::delete_by_id(&self.db, &id).await {
                    Ok(_) => removed += 1,
                    Err(error) => {
                        warn!(artwork_cache_id = %id, error = %error, "Failed to remove orphaned artwork")
                    }
                }
            }
        }
        if removed > 0 {
            info!(
                removed,
                "Removed cached artwork whose entity no longer exists"
            );
        }
        Ok(removed)
    }

    /// Cache a season poster.
    pub async fn cache_season_artwork(
        &self,
        season_id: &str,
        poster_url: Option<&str>,
    ) -> Option<String> {
        let url = poster_url?;
        match self.cache_image(url, "season", season_id, "poster").await {
            Ok(cached_url) => Some(cached_url),
            Err(e) => {
                warn!(season_id = %season_id, error = %e, "Failed to cache season poster");
                None
            }
        }
    }

    /// Cache movie artwork (poster and backdrop)
    ///
    /// Returns tuple of (cached_poster_url, cached_backdrop_url)
    pub async fn cache_movie_artwork(
        &self,
        movie_id: &str,
        poster_url: Option<&str>,
        backdrop_url: Option<&str>,
    ) -> (Option<String>, Option<String>) {
        let mut cached_poster = None;
        let mut cached_backdrop = None;

        if let Some(url) = poster_url {
            match self.cache_image(url, "movie", movie_id, "poster").await {
                Ok(cached_url) => cached_poster = Some(cached_url),
                Err(e) => {
                    warn!(
                        movie_id = %movie_id,
                        error = %e,
                        "Failed to cache movie poster"
                    );
                }
            }
        }

        if let Some(url) = backdrop_url {
            match self.cache_image(url, "movie", movie_id, "backdrop").await {
                Ok(cached_url) => cached_backdrop = Some(cached_url),
                Err(e) => {
                    warn!(
                        movie_id = %movie_id,
                        error = %e,
                        "Failed to cache movie backdrop"
                    );
                }
            }
        }

        (cached_poster, cached_backdrop)
    }

    /// Cache show artwork (poster and backdrop)
    pub async fn cache_show_artwork(
        &self,
        show_id: &str,
        poster_url: Option<&str>,
        backdrop_url: Option<&str>,
    ) -> (Option<String>, Option<String>) {
        let mut cached_poster = None;
        let mut cached_backdrop = None;

        if let Some(url) = poster_url {
            match self.cache_image(url, "show", show_id, "poster").await {
                Ok(cached_url) => cached_poster = Some(cached_url),
                Err(e) => {
                    warn!(
                        show_id = %show_id,
                        error = %e,
                        "Failed to cache show poster"
                    );
                }
            }
        }

        if let Some(url) = backdrop_url {
            match self.cache_image(url, "show", show_id, "backdrop").await {
                Ok(cached_url) => cached_backdrop = Some(cached_url),
                Err(e) => {
                    warn!(
                        show_id = %show_id,
                        error = %e,
                        "Failed to cache show backdrop"
                    );
                }
            }
        }

        (cached_poster, cached_backdrop)
    }

    /// Cache album artwork (cover)
    pub async fn cache_album_artwork(
        &self,
        album_id: &str,
        cover_url: Option<&str>,
    ) -> Option<String> {
        if let Some(url) = cover_url {
            match self.cache_image(url, "album", album_id, "cover").await {
                Ok(cached_url) => Some(cached_url),
                Err(e) => {
                    warn!(
                        album_id = %album_id,
                        error = %e,
                        "Failed to cache album cover"
                    );
                    None
                }
            }
        } else {
            None
        }
    }

    /// Cache audiobook artwork (cover)
    pub async fn cache_audiobook_artwork(
        &self,
        audiobook_id: &str,
        cover_url: Option<&str>,
    ) -> Option<String> {
        if let Some(url) = cover_url {
            match self
                .cache_image(url, "audiobook", audiobook_id, "cover")
                .await
            {
                Ok(cached_url) => Some(cached_url),
                Err(e) => {
                    warn!(
                        audiobook_id = %audiobook_id,
                        error = %e,
                        "Failed to cache audiobook cover"
                    );
                    None
                }
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct ResolvedArtworkDestination {
    host: String,
    socket_addr: Option<SocketAddr>,
}

/// Downscale artwork to `width` (aspect preserved). Images with transparency stay PNG;
/// everything else becomes a progressive-quality JPEG, which is far smaller for photos.
fn resize_artwork(bytes: &[u8], width: u32) -> Result<(Vec<u8>, String, i32, i32)> {
    let image = image::load_from_memory(bytes).map_err(|_| ArtworkFetchError::InvalidImage)?;
    let height =
        ((image.height() as u64 * width as u64) / image.width().max(1) as u64).max(1) as u32;
    let resized = image.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
    let mut out = std::io::Cursor::new(Vec::new());
    let mime = if resized.color().has_alpha() {
        resized
            .write_to(&mut out, image::ImageFormat::Png)
            .context("Failed to encode resized artwork")?;
        "image/png"
    } else {
        let rgb = resized.to_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, VARIANT_JPEG_QUALITY)
            .encode_image(&rgb)
            .context("Failed to encode resized artwork")?;
        "image/jpeg"
    };
    Ok((
        out.into_inner(),
        mime.to_string(),
        width as i32,
        height as i32,
    ))
}

fn normalize_allowed_host(input: &str) -> Option<String> {
    let candidate = input
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if candidate.is_empty()
        || candidate.chars().any(char::is_whitespace)
        || candidate.contains(['/', '@', '?', '#', '*'])
    {
        return None;
    }
    Host::parse(&candidate).ok().map(|host| host.to_string())
}

async fn resolve_artwork_destination(
    url: &Url,
    allowed_hosts: &BTreeSet<String>,
) -> std::result::Result<ResolvedArtworkDestination, ArtworkFetchError> {
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(ArtworkFetchError::DestinationDenied);
    }
    let expected_port = if url.scheme() == "https" { 443 } else { 80 };
    if url.port().is_some_and(|port| port != expected_port) {
        return Err(ArtworkFetchError::DestinationDenied);
    }
    let host = url.host().ok_or(ArtworkFetchError::DestinationDenied)?;
    let normalized_host = host
        .to_string()
        .trim_matches(['[', ']'])
        .to_ascii_lowercase();
    if !allowed_hosts.contains(&normalized_host) {
        return Err(ArtworkFetchError::DestinationDenied);
    }

    match host {
        Host::Domain(domain) => {
            let addresses = timeout(CONNECT_TIMEOUT, lookup_host((domain, expected_port)))
                .await
                .map_err(|_| ArtworkFetchError::Timeout)?
                .map_err(|_| ArtworkFetchError::Network)?
                .collect::<Vec<_>>();
            if addresses.is_empty() || addresses.iter().any(|addr| !is_public_ip(addr.ip())) {
                return Err(ArtworkFetchError::DestinationDenied);
            }
            Ok(ResolvedArtworkDestination {
                host: normalized_host,
                socket_addr: addresses.first().copied(),
            })
        }
        Host::Ipv4(address) => {
            if !is_public_ip(IpAddr::V4(address)) {
                return Err(ArtworkFetchError::DestinationDenied);
            }
            Ok(ResolvedArtworkDestination {
                host: normalized_host,
                socket_addr: None,
            })
        }
        Host::Ipv6(address) => {
            if !is_public_ip(IpAddr::V6(address)) {
                return Err(ArtworkFetchError::DestinationDenied);
            }
            Ok(ResolvedArtworkDestination {
                host: normalized_host,
                socket_addr: None,
            })
        }
    }
}

fn pinned_artwork_client(host: &str, socket_addr: Option<SocketAddr>) -> Result<reqwest::Client> {
    let mut builder = crate::services::http_client::outbound_client_builder(
        crate::services::http_client::OutboundHttpProfile::Artwork,
    )
    .connect_timeout(CONNECT_TIMEOUT)
    .timeout(REQUEST_TIMEOUT)
    .pool_idle_timeout(Duration::from_secs(10));
    if let Some(socket_addr) = socket_addr {
        builder = builder.resolve(host, socket_addr);
    }
    builder
        .build()
        .map_err(|_| ArtworkFetchError::Network.into())
}

fn is_public_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_ipv4(address),
        IpAddr::V6(address) => is_public_ipv6(address),
    }
}

fn is_public_ipv4(address: Ipv4Addr) -> bool {
    let [a, b, c, _] = address.octets();
    !matches!(
        (a, b, c),
        (0, _, _)
            | (10, _, _)
            | (100, 64..=127, _)
            | (127, _, _)
            | (169, 254, _)
            | (172, 16..=31, _)
            | (192, 0, _)
            | (192, 168, _)
            | (198, 18..=19, _)
            | (198, 51, 100)
            | (203, 0, 113)
            | (224..=255, _, _)
    )
}

fn is_public_ipv6(address: Ipv6Addr) -> bool {
    if let Some(mapped) = address.to_ipv4_mapped() {
        return is_public_ipv4(mapped);
    }
    let segments = address.segments();
    !(address.is_unspecified()
        || address.is_loopback()
        || address.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] == 0x2001 && segments[1] == 0x0db8))
}

fn normalize_image_mime(value: &str) -> Option<String> {
    match value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "image/jpeg" | "image/jpg" => Some("image/jpeg".to_string()),
        "image/png" => Some("image/png".to_string()),
        "image/webp" => Some("image/webp".to_string()),
        _ => None,
    }
}

fn mime_matches_format(mime_type: &str, format: image::ImageFormat) -> bool {
    matches!(
        (mime_type, format),
        ("image/jpeg", image::ImageFormat::Jpeg)
            | ("image/png", image::ImageFormat::Png)
            | ("image/webp", image::ImageFormat::WebP)
    )
}

fn source_url_file_name(source_url: &str) -> Option<String> {
    let without_query = source_url.split(['?', '#']).next().unwrap_or(source_url);
    without_query
        .rsplit('/')
        .next()
        .filter(|segment| !segment.trim().is_empty())
        .map(|segment| segment.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_reserved_and_documentation_ipv4_ranges_are_denied() {
        for address in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "192.0.0.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.51.100.10",
            "203.0.113.10",
            "224.0.0.1",
            "255.255.255.255",
        ] {
            assert!(
                !is_public_ip(address.parse().expect("test IP should parse")),
                "{address} must be denied"
            );
        }
        assert!(is_public_ip("1.1.1.1".parse().expect("public IP")));
        assert!(is_public_ip("8.8.8.8".parse().expect("public IP")));
    }

    #[test]
    fn local_reserved_and_mapped_private_ipv6_ranges_are_denied() {
        for address in [
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "fc00::1",
            "fd00::1",
            "fe80::1",
            "ff02::1",
            "2001:db8::1",
        ] {
            assert!(
                !is_public_ip(address.parse().expect("test IP should parse")),
                "{address} must be denied"
            );
        }
        assert!(is_public_ip(
            "2606:4700:4700::1111".parse().expect("public IP")
        ));
    }

    #[tokio::test]
    async fn destination_validation_rejects_schemes_credentials_ports_and_loopback() {
        let allowed = BTreeSet::from(["127.0.0.1".to_string(), "1.1.1.1".to_string()]);
        for url in [
            "file:///etc/passwd",
            "ftp://1.1.1.1/image.jpg",
            "https://user:secret@1.1.1.1/image.jpg",
            "https://1.1.1.1:8443/image.jpg",
            "http://127.0.0.1/image.jpg",
        ] {
            let parsed = Url::parse(url).expect("test URL should parse");
            assert!(
                resolve_artwork_destination(&parsed, &allowed)
                    .await
                    .is_err(),
                "{url} must be denied"
            );
        }

        let public = Url::parse("https://1.1.1.1/image.jpg").expect("test URL should parse");
        assert!(resolve_artwork_destination(&public, &allowed).await.is_ok());
    }

    #[test]
    fn only_supported_image_content_types_are_accepted() {
        assert_eq!(
            normalize_image_mime("image/jpeg; charset=binary").as_deref(),
            Some("image/jpeg")
        );
        assert_eq!(
            normalize_image_mime("IMAGE/PNG").as_deref(),
            Some("image/png")
        );
        assert!(normalize_image_mime("image/svg+xml").is_none());
        assert!(normalize_image_mime("text/html").is_none());
        assert!(normalize_image_mime("").is_none());
    }

    #[test]
    fn requested_widths_snap_up_to_a_supported_step() {
        assert_eq!(normalize_artwork_width(None), None);
        assert_eq!(normalize_artwork_width(Some(0)), None);
        assert_eq!(normalize_artwork_width(Some(150)), Some(185));
        assert_eq!(normalize_artwork_width(Some(342)), Some(342));
        assert_eq!(normalize_artwork_width(Some(343)), Some(500));
        assert_eq!(normalize_artwork_width(Some(9000)), Some(1920));
    }

    #[test]
    fn resized_photos_become_jpeg_and_transparent_art_stays_png() {
        let encode = |image: image::DynamicImage, format| {
            let mut out = std::io::Cursor::new(Vec::new());
            image.write_to(&mut out, format).expect("encode test image");
            out.into_inner()
        };
        let photo = encode(
            image::DynamicImage::new_rgb8(1000, 1500),
            image::ImageFormat::Png,
        );
        let (bytes, mime, width, height) = resize_artwork(&photo, 342).expect("resize photo");
        assert_eq!((mime.as_str(), width, height), ("image/jpeg", 342, 513));
        assert_eq!(
            image::guess_format(&bytes).ok(),
            Some(image::ImageFormat::Jpeg)
        );

        let logo = encode(
            image::DynamicImage::new_rgba8(800, 400),
            image::ImageFormat::Png,
        );
        let (_, mime, width, height) = resize_artwork(&logo, 500).expect("resize logo");
        assert_eq!((mime.as_str(), width, height), ("image/png", 500, 250));
    }
}
