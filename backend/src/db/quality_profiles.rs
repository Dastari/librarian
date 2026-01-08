//! Quality Profile database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Quality Profile record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QualityProfileRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub preferred_resolution: Option<String>,
    pub min_resolution: Option<String>,
    pub preferred_codec: Option<String>,
    pub preferred_audio: Option<String>,
    pub require_hdr: bool,
    pub hdr_types: Vec<String>,
    pub preferred_language: Option<String>,
    pub max_size_gb: Option<rust_decimal::Decimal>,
    pub min_seeders: Option<i32>,
    pub release_group_whitelist: Vec<String>,
    pub release_group_blacklist: Vec<String>,
    pub upgrade_until: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Input for creating a quality profile
#[derive(Debug)]
pub struct CreateQualityProfile {
    pub user_id: Uuid,
    pub name: String,
    pub preferred_resolution: Option<String>,
    pub min_resolution: Option<String>,
    pub preferred_codec: Option<String>,
    pub preferred_audio: Option<String>,
    pub require_hdr: bool,
    pub hdr_types: Vec<String>,
    pub preferred_language: Option<String>,
    pub max_size_gb: Option<rust_decimal::Decimal>,
    pub min_seeders: Option<i32>,
    pub release_group_whitelist: Vec<String>,
    pub release_group_blacklist: Vec<String>,
    pub upgrade_until: Option<String>,
}

/// Input for updating a quality profile
#[derive(Debug, Default)]
pub struct UpdateQualityProfile {
    pub name: Option<String>,
    pub preferred_resolution: Option<String>,
    pub min_resolution: Option<String>,
    pub preferred_codec: Option<String>,
    pub preferred_audio: Option<String>,
    pub require_hdr: Option<bool>,
    pub hdr_types: Option<Vec<String>>,
    pub preferred_language: Option<String>,
    pub max_size_gb: Option<rust_decimal::Decimal>,
    pub min_seeders: Option<i32>,
    pub release_group_whitelist: Option<Vec<String>>,
    pub release_group_blacklist: Option<Vec<String>>,
    pub upgrade_until: Option<String>,
}

pub struct QualityProfileRepository {
    pool: PgPool,
}

impl QualityProfileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all quality profiles for a user
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<QualityProfileRecord>> {
        let records = sqlx::query_as::<_, QualityProfileRecord>(
            r#"
            SELECT id, user_id, name, preferred_resolution, min_resolution,
                   preferred_codec, preferred_audio, require_hdr, hdr_types,
                   preferred_language, max_size_gb, min_seeders,
                   release_group_whitelist, release_group_blacklist,
                   upgrade_until, created_at, updated_at
            FROM quality_profiles
            WHERE user_id = $1
            ORDER BY name
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a quality profile by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<QualityProfileRecord>> {
        let record = sqlx::query_as::<_, QualityProfileRecord>(
            r#"
            SELECT id, user_id, name, preferred_resolution, min_resolution,
                   preferred_codec, preferred_audio, require_hdr, hdr_types,
                   preferred_language, max_size_gb, min_seeders,
                   release_group_whitelist, release_group_blacklist,
                   upgrade_until, created_at, updated_at
            FROM quality_profiles
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a new quality profile
    pub async fn create(&self, input: CreateQualityProfile) -> Result<QualityProfileRecord> {
        let record = sqlx::query_as::<_, QualityProfileRecord>(
            r#"
            INSERT INTO quality_profiles (
                user_id, name, preferred_resolution, min_resolution,
                preferred_codec, preferred_audio, require_hdr, hdr_types,
                preferred_language, max_size_gb, min_seeders,
                release_group_whitelist, release_group_blacklist, upgrade_until
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING id, user_id, name, preferred_resolution, min_resolution,
                      preferred_codec, preferred_audio, require_hdr, hdr_types,
                      preferred_language, max_size_gb, min_seeders,
                      release_group_whitelist, release_group_blacklist,
                      upgrade_until, created_at, updated_at
            "#,
        )
        .bind(input.user_id)
        .bind(&input.name)
        .bind(&input.preferred_resolution)
        .bind(&input.min_resolution)
        .bind(&input.preferred_codec)
        .bind(&input.preferred_audio)
        .bind(input.require_hdr)
        .bind(&input.hdr_types)
        .bind(&input.preferred_language)
        .bind(input.max_size_gb)
        .bind(input.min_seeders)
        .bind(&input.release_group_whitelist)
        .bind(&input.release_group_blacklist)
        .bind(&input.upgrade_until)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update a quality profile
    pub async fn update(&self, id: Uuid, input: UpdateQualityProfile) -> Result<Option<QualityProfileRecord>> {
        let record = sqlx::query_as::<_, QualityProfileRecord>(
            r#"
            UPDATE quality_profiles SET
                name = COALESCE($2, name),
                preferred_resolution = COALESCE($3, preferred_resolution),
                min_resolution = COALESCE($4, min_resolution),
                preferred_codec = COALESCE($5, preferred_codec),
                preferred_audio = COALESCE($6, preferred_audio),
                require_hdr = COALESCE($7, require_hdr),
                hdr_types = COALESCE($8, hdr_types),
                preferred_language = COALESCE($9, preferred_language),
                max_size_gb = COALESCE($10, max_size_gb),
                min_seeders = COALESCE($11, min_seeders),
                release_group_whitelist = COALESCE($12, release_group_whitelist),
                release_group_blacklist = COALESCE($13, release_group_blacklist),
                upgrade_until = COALESCE($14, upgrade_until),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, name, preferred_resolution, min_resolution,
                      preferred_codec, preferred_audio, require_hdr, hdr_types,
                      preferred_language, max_size_gb, min_seeders,
                      release_group_whitelist, release_group_blacklist,
                      upgrade_until, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&input.name)
        .bind(&input.preferred_resolution)
        .bind(&input.min_resolution)
        .bind(&input.preferred_codec)
        .bind(&input.preferred_audio)
        .bind(input.require_hdr)
        .bind(&input.hdr_types)
        .bind(&input.preferred_language)
        .bind(input.max_size_gb)
        .bind(input.min_seeders)
        .bind(&input.release_group_whitelist)
        .bind(&input.release_group_blacklist)
        .bind(&input.upgrade_until)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a quality profile
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM quality_profiles WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Create default quality profiles for a user
    pub async fn create_defaults(&self, user_id: Uuid) -> Result<Vec<QualityProfileRecord>> {
        let mut profiles = Vec::new();

        // 4K HDR profile
        profiles.push(
            self.create(CreateQualityProfile {
                user_id,
                name: "4K HDR".to_string(),
                preferred_resolution: Some("2160p".to_string()),
                min_resolution: Some("1080p".to_string()),
                preferred_codec: Some("hevc".to_string()),
                preferred_audio: Some("atmos".to_string()),
                require_hdr: true,
                hdr_types: vec!["hdr10".to_string(), "hdr10plus".to_string(), "dolbyvision".to_string()],
                preferred_language: Some("en".to_string()),
                max_size_gb: None,
                min_seeders: Some(1),
                release_group_whitelist: vec![],
                release_group_blacklist: vec![],
                upgrade_until: Some("2160p".to_string()),
            })
            .await?,
        );

        // 1080p Standard profile
        profiles.push(
            self.create(CreateQualityProfile {
                user_id,
                name: "1080p Standard".to_string(),
                preferred_resolution: Some("1080p".to_string()),
                min_resolution: Some("720p".to_string()),
                preferred_codec: Some("any".to_string()),
                preferred_audio: Some("any".to_string()),
                require_hdr: false,
                hdr_types: vec![],
                preferred_language: Some("en".to_string()),
                max_size_gb: Some(rust_decimal::Decimal::new(5, 0)), // 5 GB
                min_seeders: Some(1),
                release_group_whitelist: vec![],
                release_group_blacklist: vec![],
                upgrade_until: Some("1080p".to_string()),
            })
            .await?,
        );

        // 720p Compact profile
        profiles.push(
            self.create(CreateQualityProfile {
                user_id,
                name: "720p Compact".to_string(),
                preferred_resolution: Some("720p".to_string()),
                min_resolution: Some("480p".to_string()),
                preferred_codec: Some("hevc".to_string()),
                preferred_audio: Some("any".to_string()),
                require_hdr: false,
                hdr_types: vec![],
                preferred_language: Some("en".to_string()),
                max_size_gb: Some(rust_decimal::Decimal::new(2, 0)), // 2 GB
                min_seeders: Some(1),
                release_group_whitelist: vec![],
                release_group_blacklist: vec![],
                upgrade_until: Some("720p".to_string()),
            })
            .await?,
        );

        // Any Quality profile
        profiles.push(
            self.create(CreateQualityProfile {
                user_id,
                name: "Any Quality".to_string(),
                preferred_resolution: Some("any".to_string()),
                min_resolution: Some("any".to_string()),
                preferred_codec: Some("any".to_string()),
                preferred_audio: Some("any".to_string()),
                require_hdr: false,
                hdr_types: vec![],
                preferred_language: None,
                max_size_gb: None,
                min_seeders: Some(1),
                release_group_whitelist: vec![],
                release_group_blacklist: vec![],
                upgrade_until: Some("any".to_string()),
            })
            .await?,
        );

        Ok(profiles)
    }

    /// Check if user has any quality profiles
    pub async fn has_profiles(&self, user_id: Uuid) -> Result<bool> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM quality_profiles WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0 > 0)
    }
}
