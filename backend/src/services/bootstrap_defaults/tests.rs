use std::collections::HashSet;

use crate::db::Database;
use crate::services::graphql::entities::{AppSetting, NamingPattern, TorznabCategory};
use crate::services::{DatabaseService, Service};

use super::seed_defaults;

async fn fresh_database() -> Database {
    let pool = graphql_orm::DbPool::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite database should connect");
    Database::new(pool)
}

#[tokio::test]
async fn database_start_seeds_legacy_default_data_through_orm() {
    let db = fresh_database().await;
    let service = DatabaseService::new(db.clone());

    service
        .start()
        .await
        .expect("database service should start and seed defaults");

    let settings = AppSetting::query(db.pool())
        .fetch_all()
        .await
        .expect("app settings should query");
    let setting_keys = settings
        .iter()
        .map(|setting| setting.key.as_str())
        .collect::<HashSet<_>>();
    assert!(setting_keys.contains("torrent.download_dir"));
    assert!(setting_keys.contains("llm.enabled"));
    assert!(setting_keys.contains("metadata.auto_fetch"));
    assert!(setting_keys.contains("organize.copy_mode"));

    let patterns = NamingPattern::query(db.pool())
        .fetch_all()
        .await
        .expect("naming patterns should query");
    assert!(patterns.iter().any(|pattern| {
        pattern.library_type == "movies" && pattern.name == "Movie Standard" && pattern.is_default
    }));
    assert!(patterns.iter().any(|pattern| {
        pattern.library_type == "tv" && pattern.name == "Standard" && pattern.is_system
    }));

    let categories = TorznabCategory::query(db.pool())
        .fetch_all()
        .await
        .expect("torznab categories should query");
    assert!(categories.iter().any(|category| category.id == "2000"));
    assert!(categories.iter().any(|category| {
        category.id == "2040" && category.parent_id.as_deref() == Some("2000")
    }));
}

#[tokio::test]
async fn default_seed_is_idempotent() {
    let db = fresh_database().await;
    let service = DatabaseService::new(db.clone());

    service
        .start()
        .await
        .expect("database service should start and seed defaults");

    let initial_settings = AppSetting::query(db.pool())
        .fetch_all()
        .await
        .expect("app settings should query")
        .len();
    let initial_patterns = NamingPattern::query(db.pool())
        .fetch_all()
        .await
        .expect("naming patterns should query")
        .len();
    let initial_categories = TorznabCategory::query(db.pool())
        .fetch_all()
        .await
        .expect("torznab categories should query")
        .len();

    seed_defaults(&db)
        .await
        .expect("default seed should be repeatable");

    assert_eq!(
        AppSetting::query(db.pool())
            .fetch_all()
            .await
            .expect("app settings should query")
            .len(),
        initial_settings
    );
    assert_eq!(
        NamingPattern::query(db.pool())
            .fetch_all()
            .await
            .expect("naming patterns should query")
            .len(),
        initial_patterns
    );
    assert_eq!(
        TorznabCategory::query(db.pool())
            .fetch_all()
            .await
            .expect("torznab categories should query")
            .len(),
        initial_categories
    );
}
