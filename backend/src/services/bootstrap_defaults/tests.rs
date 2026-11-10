use std::collections::HashSet;

use crate::db::Database;
use crate::services::graphql::entities::{
    AppSetting, CreateLibraryInput, CreateNamingPatternInput, CreateUserInput, Library,
    NamingPattern, TorznabCategory, UpdateNamingPatternInput, User,
};
use crate::services::library_scan::{
    TV_DEFAULT_NAMING_PATTERN, TV_PREVIOUS_DEFAULT_NAMING_PATTERN,
};
use crate::services::{DatabaseService, Service};

use super::{migrate_previous_tv_default, seed_defaults};

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

async fn library_with(db: &Database, user_id: &str, library_type: &str, pattern: &str) -> Library {
    Library::insert(
        db,
        CreateLibraryInput {
            user_id: user_id.to_string(),
            name: format!("{library_type} {pattern}"),
            path: format!("/srv/{}", uuid::Uuid::new_v4()),
            library_type: library_type.to_string(),
            icon: None,
            color: None,
            auto_scan: false,
            auto_organize: true,
            auto_resolve_duplicates: None,
            naming_pattern: pattern.to_string(),
            scan_interval_minutes: 60,
            watch_for_changes: false,
            watch_status: None,
            watch_detail: None,
            watch_status_since: None,
            scanning: false,
            last_scanned_at: None,
            quality_profile_id: None,
        },
    )
    .await
    .expect("library should insert")
}

/// Q145: only a stored pattern exactly equal to the previous TV default moves to the new one.
#[tokio::test]
async fn only_the_exact_previous_tv_default_is_moved_to_the_one_with_the_year() {
    let service = DatabaseService::new(fresh_database().await);
    service
        .start()
        .await
        .expect("database service should start and seed defaults");
    // With the service's policies, as startup seeding runs.
    let db = service.pool().clone();
    let user = User::insert(
        &db,
        CreateUserInput {
            username: "pattern-owner".to_string(),
            email: None,
            password_hash: "test-only".to_string(),
            role: "Admin".to_string(),
            display_name: None,
            avatar_url: None,
            is_active: true,
            last_login_at: None,
        },
    )
    .await
    .expect("user should insert");

    let previous = library_with(&db, &user.id, "tv", TV_PREVIOUS_DEFAULT_NAMING_PATTERN).await;
    let upper = library_with(&db, &user.id, "TV", TV_PREVIOUS_DEFAULT_NAMING_PATTERN).await;
    let custom_pattern =
        "{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title} - Custom.{ext}";
    let custom = library_with(&db, &user.id, "tv", custom_pattern).await;
    let unset = library_with(&db, &user.id, "tv", "").await;
    // The same text on a library that is not TV is not the TV default.
    let other = library_with(&db, &user.id, "other", TV_PREVIOUS_DEFAULT_NAMING_PATTERN).await;
    let movies = library_with(
        &db,
        &user.id,
        "movies",
        "{title} ({year})/{title} ({year}).{ext}",
    )
    .await;

    // A database seeded before the change: the built-in pattern still has the previous text, and
    // somebody's own pattern happens to have it too.
    let standard = NamingPattern::query(db.pool())
        .fetch_all()
        .await
        .unwrap()
        .into_iter()
        .find(|pattern| pattern.library_type == "tv" && pattern.is_system && pattern.is_default)
        .expect("the built-in TV default is seeded");
    assert_eq!(
        standard.pattern, TV_DEFAULT_NAMING_PATTERN,
        "new databases get the new default"
    );
    NamingPattern::update_by_id(
        &db,
        &standard.id,
        UpdateNamingPatternInput {
            pattern: Some(TV_PREVIOUS_DEFAULT_NAMING_PATTERN.to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let own = NamingPattern::insert(
        &db,
        CreateNamingPatternInput {
            user_id: user.id.clone(),
            library_type: "tv".to_string(),
            name: "Mine".to_string(),
            pattern: TV_PREVIOUS_DEFAULT_NAMING_PATTERN.to_string(),
            description: None,
            is_default: false,
            is_system: false,
        },
    )
    .await
    .unwrap();

    migrate_previous_tv_default(&db).await.unwrap();

    let pattern_of = |library: &Library| {
        let db = db.clone();
        let id = library.id.clone();
        async move {
            Library::get(db.pool(), &id)
                .await
                .unwrap()
                .unwrap()
                .naming_pattern
        }
    };
    assert_eq!(pattern_of(&previous).await, TV_DEFAULT_NAMING_PATTERN);
    assert_eq!(pattern_of(&upper).await, TV_DEFAULT_NAMING_PATTERN);
    assert_eq!(pattern_of(&custom).await, custom_pattern);
    assert_eq!(pattern_of(&unset).await, "");
    assert_eq!(pattern_of(&other).await, TV_PREVIOUS_DEFAULT_NAMING_PATTERN);
    assert_eq!(
        pattern_of(&movies).await,
        "{title} ({year})/{title} ({year}).{ext}"
    );
    let stored = |id: String| {
        let db = db.clone();
        async move {
            NamingPattern::get(db.pool(), &id)
                .await
                .unwrap()
                .unwrap()
                .pattern
        }
    };
    assert_eq!(stored(standard.id.clone()).await, TV_DEFAULT_NAMING_PATTERN);
    assert_eq!(
        stored(own.id.clone()).await,
        TV_PREVIOUS_DEFAULT_NAMING_PATTERN,
        "a pattern somebody made is theirs"
    );

    // Once is enough: a second start finds nothing left to move.
    seed_defaults(&db).await.unwrap();
    assert_eq!(pattern_of(&previous).await, TV_DEFAULT_NAMING_PATTERN);
    assert_eq!(pattern_of(&custom).await, custom_pattern);
}
