//! Naming patterns database repository
//!
//! Manages file naming pattern presets for library organization.

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Naming pattern record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NamingPatternRecord {
    pub id: Uuid,
    pub name: String,
    pub pattern: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub is_system: bool,
    pub library_type: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Input for creating a naming pattern
#[derive(Debug)]
pub struct CreateNamingPattern {
    pub name: String,
    pub pattern: String,
    pub description: Option<String>,
    pub library_type: String,
}

/// Input for updating a naming pattern
#[derive(Debug)]
pub struct UpdateNamingPattern {
    pub name: Option<String>,
    pub pattern: Option<String>,
    pub description: Option<String>,
}

/// Naming pattern repository
pub struct NamingPatternRepository {
    pool: PgPool,
}

impl NamingPatternRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// List all naming patterns
    pub async fn list_all(&self) -> Result<Vec<NamingPatternRecord>> {
        let records = sqlx::query_as::<_, NamingPatternRecord>(
            r#"
            SELECT id, name, pattern, description, is_default, is_system, library_type, created_at
            FROM naming_patterns
            ORDER BY library_type, is_default DESC, is_system DESC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List naming patterns by library type
    pub async fn list_by_type(&self, library_type: &str) -> Result<Vec<NamingPatternRecord>> {
        let records = sqlx::query_as::<_, NamingPatternRecord>(
            r#"
            SELECT id, name, pattern, description, is_default, is_system, library_type, created_at
            FROM naming_patterns
            WHERE library_type = $1
            ORDER BY is_default DESC, is_system DESC, name ASC
            "#,
        )
        .bind(library_type)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a naming pattern by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<NamingPatternRecord>> {
        let record = sqlx::query_as::<_, NamingPatternRecord>(
            r#"
            SELECT id, name, pattern, description, is_default, is_system, library_type, created_at
            FROM naming_patterns
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get the default naming pattern for a library type
    pub async fn get_default_for_type(
        &self,
        library_type: &str,
    ) -> Result<Option<NamingPatternRecord>> {
        let record = sqlx::query_as::<_, NamingPatternRecord>(
            r#"
            SELECT id, name, pattern, description, is_default, is_system, library_type, created_at
            FROM naming_patterns
            WHERE is_default = true AND library_type = $1
            LIMIT 1
            "#,
        )
        .bind(library_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get the default naming pattern (legacy - returns TV default)
    pub async fn get_default(&self) -> Result<Option<NamingPatternRecord>> {
        self.get_default_for_type("tv").await
    }

    /// Get the default pattern string for a library type
    pub async fn get_default_pattern_for_type(&self, library_type: &str) -> Result<String> {
        if let Some(record) = self.get_default_for_type(library_type).await? {
            Ok(record.pattern)
        } else {
            // Fallback to hardcoded defaults by type
            Ok(match library_type {
                "movies" => "{title} ({year})/{title} ({year}).{ext}".to_string(),
                "music" => "{artist}/{album} ({year})/{track:02} - {title}.{ext}".to_string(),
                "audiobooks" => "{author}/{title}/{chapter:02} - {chapter_title}.{ext}".to_string(),
                "other" => "{name}.{ext}".to_string(),
                _ => "{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}"
                    .to_string(),
            })
        }
    }

    /// Get the default pattern string (legacy - returns TV pattern)
    pub async fn get_default_pattern(&self) -> Result<String> {
        self.get_default_pattern_for_type("tv").await
    }

    /// Create a custom naming pattern (user-created, not system)
    pub async fn create(&self, input: CreateNamingPattern) -> Result<NamingPatternRecord> {
        let record = sqlx::query_as::<_, NamingPatternRecord>(
            r#"
            INSERT INTO naming_patterns (name, pattern, description, is_default, is_system, library_type)
            VALUES ($1, $2, $3, false, false, $4)
            RETURNING id, name, pattern, description, is_default, is_system, library_type, created_at
            "#,
        )
        .bind(&input.name)
        .bind(&input.pattern)
        .bind(&input.description)
        .bind(&input.library_type)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a naming pattern (only non-system patterns can be deleted)
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM naming_patterns 
            WHERE id = $1 AND is_system = false
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update a naming pattern (only non-system patterns can be updated)
    pub async fn update(&self, id: Uuid, input: UpdateNamingPattern) -> Result<Option<NamingPatternRecord>> {
        // Build dynamic update query
        let mut set_clauses = Vec::new();
        let mut param_idx = 2; // $1 is id

        if input.name.is_some() {
            set_clauses.push(format!("name = ${}", param_idx));
            param_idx += 1;
        }
        if input.pattern.is_some() {
            set_clauses.push(format!("pattern = ${}", param_idx));
            param_idx += 1;
        }
        if input.description.is_some() {
            set_clauses.push(format!("description = ${}", param_idx));
        }

        if set_clauses.is_empty() {
            // Nothing to update, just return the existing record
            return self.get_by_id(id).await;
        }

        let query = format!(
            r#"
            UPDATE naming_patterns
            SET {}
            WHERE id = $1 AND is_system = false
            RETURNING id, name, pattern, description, is_default, is_system, library_type, created_at
            "#,
            set_clauses.join(", ")
        );

        let mut query_builder = sqlx::query_as::<_, NamingPatternRecord>(&query).bind(id);

        if let Some(name) = &input.name {
            query_builder = query_builder.bind(name);
        }
        if let Some(pattern) = &input.pattern {
            query_builder = query_builder.bind(pattern);
        }
        if let Some(description) = &input.description {
            query_builder = query_builder.bind(description);
        }

        let record = query_builder.fetch_optional(&self.pool).await?;
        Ok(record)
    }

    /// Set a pattern as the default for its library type (unsets any existing default for that type)
    pub async fn set_default(&self, id: Uuid) -> Result<bool> {
        // Get the pattern's library type first
        let pattern = self.get_by_id(id).await?;
        let library_type = match pattern {
            Some(p) => p.library_type.unwrap_or_else(|| "tv".to_string()),
            None => return Ok(false),
        };

        // Unset defaults for this library type
        sqlx::query(
            "UPDATE naming_patterns SET is_default = false WHERE is_default = true AND library_type = $1",
        )
        .bind(&library_type)
        .execute(&self.pool)
        .await?;

        // Set the new default
        let result = sqlx::query("UPDATE naming_patterns SET is_default = true WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
