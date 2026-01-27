//! GraphQL DataLoaders for batching database queries
//!
//! DataLoaders solve the N+1 problem by collecting multiple requests
//! for related entities and executing them in a single batch query.
//!
//! # Architecture
//!
//! Relations marked with `#[relation(...)]` in entity structs automatically
//! use DataLoaders via the generated `ComplexObject` resolvers.
//!
//! The pattern works as follows:
//! 1. When GraphQL resolves `Libraries { Shows { ... } }`, each Library's
//!    Shows resolver calls `loader.load_one(library_id)`
//! 2. DataLoader batches these calls within the same request tick
//! 3. A single SQL query fetches all Shows for all Libraries:
//!    `SELECT * FROM shows WHERE library_id IN (...)`
//! 4. Results are grouped by library_id and returned to each resolver
//!
//! # Adding a New Relation
//!
//! 1. Add `#[relation(...)]` attribute to the parent entity field
//! 2. Implement `HasForeignKey` for the child entity type (or use the derive macro)
//! 3. Register a `DataLoader<RelationLoader<ChildType>>` in the schema

use std::collections::HashMap;
use std::sync::Arc;

use async_graphql::dataloader::Loader;

use crate::db::Database;
use crate::services::graphql::orm::{DatabaseEntity, FromSqlRow};

// ============================================================================
// Generic Relation Loader
// ============================================================================

/// A generic loader that batches queries for a one-to-many relation.
///
/// Given a set of parent IDs, loads all child entities where `fk_column = parent_id`
/// and groups them by the foreign key value.
///
/// # Type Parameters
/// - `T`: The child entity type (must implement `DatabaseEntity` + `FromSqlRow` + `HasForeignKey`)
///
/// # Example
///
/// ```ignore
/// // Register in schema
/// let shows_loader = DataLoader::new(
///     RelationLoader::<Show>::new(db.clone(), "library_id"),
///     tokio::spawn,
/// );
/// schema.data(shows_loader);
///
/// // Use in resolver (automatically done by macro)
/// let loader = ctx.data_unchecked::<DataLoader<RelationLoader<Show>>>();
/// let shows = loader.load_one(library_id).await?.unwrap_or_default();
/// ```
pub struct RelationLoader<T: DatabaseEntity + FromSqlRow + Send + Sync + Clone + HasForeignKey> {
    pub db: Database,
    pub fk_column: &'static str,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: DatabaseEntity + FromSqlRow + Send + Sync + Clone + HasForeignKey> RelationLoader<T> {
    pub fn new(db: Database, fk_column: &'static str) -> Self {
        Self {
            db,
            fk_column,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Trait for entities that have a foreign key field we can extract.
///
/// This is used by DataLoader to group batch-loaded entities by their parent ID.
/// Implement this for any entity that can be loaded as a relation.
pub trait HasForeignKey {
    /// Get the value of a foreign key column.
    /// Returns None if the column doesn't exist on this entity.
    fn get_fk_value(&self, fk_column: &str) -> Option<String>;
}

impl<T> Loader<String> for RelationLoader<T>
where
    T: DatabaseEntity + FromSqlRow + Send + Sync + Clone + HasForeignKey + 'static,
{
    type Value = Vec<T>;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        if keys.is_empty() {
            return Ok(HashMap::new());
        }

        tracing::debug!(
            entity = T::TABLE_NAME,
            fk_column = self.fk_column,
            parent_count = keys.len(),
            "Batch loading {} entities for {} parents",
            T::TABLE_NAME,
            keys.len()
        );

        // Build a query with IN clause for all parent IDs
        let placeholders: Vec<String> = (1..=keys.len()).map(|i| format!("?{}", i)).collect();
        let sql = format!(
            "SELECT {} FROM {} WHERE {} IN ({}) ORDER BY {} {}",
            T::column_names().join(", "),
            T::TABLE_NAME,
            self.fk_column,
            placeholders.join(", "),
            T::DEFAULT_SORT,
            T::DEFAULT_SORT_DIR
        );

        let mut query = sqlx::query(&sql);
        for key in keys {
            query = query.bind(key);
        }

        let rows = query.fetch_all(&self.db).await.map_err(Arc::new)?;
        let total_loaded = rows.len();

        // Group results by FK column value
        let mut result: HashMap<String, Vec<T>> =
            keys.iter().map(|k| (k.clone(), Vec::new())).collect();

        for row in rows {
            let entity = T::from_row(&row).map_err(Arc::new)?;
            if let Some(fk_value) = entity.get_fk_value(self.fk_column) {
                if let Some(entities) = result.get_mut(&fk_value) {
                    entities.push(entity);
                }
            }
        }

        tracing::debug!(
            entity = T::TABLE_NAME,
            total_loaded = total_loaded,
            "Batch load complete"
        );

        Ok(result)
    }
}

// ============================================================================
// HasForeignKey Implementations
// ============================================================================
//
// Each entity that can be loaded as a relation needs to implement HasForeignKey.
// This tells the DataLoader how to group results by parent ID.

use crate::services::graphql::entities::Show;
use crate::services::graphql::entities::Episode;
use crate::services::graphql::entities::Movie;
use crate::services::graphql::entities::MediaFile;
use crate::services::graphql::entities::Album;
use crate::services::graphql::entities::Track;
use crate::services::graphql::entities::Artist;
use crate::services::graphql::entities::Audiobook;
use crate::services::graphql::entities::Chapter;
use crate::services::graphql::entities::TorrentFile;

impl HasForeignKey for Show {
    fn get_fk_value(&self, fk_column: &str) -> Option<String> {
        match fk_column {
            "library_id" => Some(self.library_id.clone()),
            "user_id" => Some(self.user_id.clone()),
            _ => None,
        }
    }
}

impl HasForeignKey for Episode {
    fn get_fk_value(&self, fk_column: &str) -> Option<String> {
        match fk_column {
            "show_id" => Some(self.show_id.clone()),
            _ => None,
        }
    }
}

impl HasForeignKey for Movie {
    fn get_fk_value(&self, fk_column: &str) -> Option<String> {
        match fk_column {
            "library_id" => Some(self.library_id.clone()),
            "user_id" => Some(self.user_id.clone()),
            _ => None,
        }
    }
}

impl HasForeignKey for MediaFile {
    fn get_fk_value(&self, fk_column: &str) -> Option<String> {
        match fk_column {
            "library_id" => Some(self.library_id.clone()),
            _ => None,
        }
    }
}

impl HasForeignKey for Album {
    fn get_fk_value(&self, fk_column: &str) -> Option<String> {
        match fk_column {
            "library_id" => Some(self.library_id.clone()),
            "artist_id" => Some(self.artist_id.clone()),
            _ => None,
        }
    }
}

impl HasForeignKey for Track {
    fn get_fk_value(&self, fk_column: &str) -> Option<String> {
        match fk_column {
            "album_id" => Some(self.album_id.clone()),
            "library_id" => Some(self.library_id.clone()),
            _ => None,
        }
    }
}

impl HasForeignKey for Artist {
    fn get_fk_value(&self, fk_column: &str) -> Option<String> {
        match fk_column {
            "library_id" => Some(self.library_id.clone()),
            _ => None,
        }
    }
}

impl HasForeignKey for Audiobook {
    fn get_fk_value(&self, fk_column: &str) -> Option<String> {
        match fk_column {
            "library_id" => Some(self.library_id.clone()),
            _ => None,
        }
    }
}

impl HasForeignKey for Chapter {
    fn get_fk_value(&self, fk_column: &str) -> Option<String> {
        match fk_column {
            "audiobook_id" => Some(self.audiobook_id.clone()),
            _ => None,
        }
    }
}

impl HasForeignKey for TorrentFile {
    fn get_fk_value(&self, fk_column: &str) -> Option<String> {
        match fk_column {
            "torrent_id" => Some(self.torrent_id.clone()),
            _ => None,
        }
    }
}

// ============================================================================
// In-Memory Filtering Utilities
// ============================================================================

/// Apply a StringFilter to a string value.
pub fn matches_string_filter(
    value: &str,
    filter: &crate::services::graphql::filters::StringFilter,
) -> bool {
    if let Some(ref eq) = filter.eq {
        if value != eq {
            return false;
        }
    }
    if let Some(ref ne) = filter.ne {
        if value == ne {
            return false;
        }
    }
    if let Some(ref contains) = filter.contains {
        if !value.to_lowercase().contains(&contains.to_lowercase()) {
            return false;
        }
    }
    if let Some(ref starts_with) = filter.starts_with {
        if !value.to_lowercase().starts_with(&starts_with.to_lowercase()) {
            return false;
        }
    }
    if let Some(ref ends_with) = filter.ends_with {
        if !value.to_lowercase().ends_with(&ends_with.to_lowercase()) {
            return false;
        }
    }
    if let Some(ref in_list) = filter.in_list {
        if !in_list.contains(&value.to_string()) {
            return false;
        }
    }
    if let Some(ref not_in_list) = filter.not_in {
        if not_in_list.contains(&value.to_string()) {
            return false;
        }
    }
    true
}

/// Apply an IntFilter to an i32 value.
pub fn matches_int_filter(value: i32, filter: &crate::services::graphql::filters::IntFilter) -> bool {
    if let Some(eq) = filter.eq {
        if value != eq {
            return false;
        }
    }
    if let Some(ne) = filter.ne {
        if value == ne {
            return false;
        }
    }
    if let Some(lt) = filter.lt {
        if value >= lt {
            return false;
        }
    }
    if let Some(lte) = filter.lte {
        if value > lte {
            return false;
        }
    }
    if let Some(gt) = filter.gt {
        if value <= gt {
            return false;
        }
    }
    if let Some(gte) = filter.gte {
        if value < gte {
            return false;
        }
    }
    true
}

/// Apply a BoolFilter to a bool value.
pub fn matches_bool_filter(value: bool, filter: &crate::services::graphql::filters::BoolFilter) -> bool {
    if let Some(eq) = filter.eq {
        if value != eq {
            return false;
        }
    }
    true
}

// ============================================================================
// Generic Entity Filtering and Sorting
// ============================================================================

/// Trait for filter types that can filter entities in memory.
///
/// Implemented by generated `*WhereInput` types to support in-memory filtering
/// after batch loading via DataLoader.
pub trait InMemoryFilter<T> {
    /// Check if an entity matches this filter.
    fn matches(&self, entity: &T) -> bool;
}

/// Trait for order types that can sort entities in memory.
///
/// Implemented by generated `*OrderByInput` types to support in-memory sorting
/// after batch loading via DataLoader.
pub trait InMemorySort<T> {
    /// Compare two entities for sorting.
    fn compare(&self, a: &T, b: &T) -> std::cmp::Ordering;
}

/// Filter entities in memory using the provided filter.
pub fn filter_entities<T, F: InMemoryFilter<T>>(entities: Vec<T>, filter: &F) -> Vec<T> {
    entities.into_iter().filter(|e| filter.matches(e)).collect()
}

/// Sort entities in memory using the provided order.
pub fn sort_entities<T, O: InMemorySort<T>>(entities: &mut [T], order: Option<&O>) {
    if let Some(order) = order {
        entities.sort_by(|a, b| order.compare(a, b));
    }
}

// ============================================================================
// Pagination Helpers
// ============================================================================

/// Create a Connection from a Vec of entities with pagination.
pub fn paginate_entities<T, E, C>(
    entities: Vec<T>,
    offset: usize,
    limit: usize,
    make_edge: impl Fn(T, usize) -> E,
    make_connection: impl Fn(Vec<E>, crate::services::graphql::pagination::PageInfo) -> C,
) -> C {
    let total = entities.len() as i64;
    let has_previous_page = offset > 0;
    let has_next_page = offset + limit < entities.len();

    // Slice the results
    let paginated: Vec<T> = entities.into_iter().skip(offset).take(limit).collect();

    let edges: Vec<E> = paginated
        .into_iter()
        .enumerate()
        .map(|(i, entity)| make_edge(entity, offset + i))
        .collect();

    let page_info = crate::services::graphql::pagination::PageInfo {
        has_next_page,
        has_previous_page,
        start_cursor: if edges.is_empty() {
            None
        } else {
            Some(crate::services::graphql::pagination::encode_cursor(offset as i64))
        },
        end_cursor: if edges.is_empty() {
            None
        } else {
            Some(crate::services::graphql::pagination::encode_cursor(
                (offset + edges.len().saturating_sub(1)) as i64,
            ))
        },
        total_count: Some(total),
    };

    make_connection(edges, page_info)
}
