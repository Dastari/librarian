//! Pre-seed data for initial database setup.
//!
//! Runs after schema sync to insert default rows for app_settings,
//! cast_settings, naming_patterns, and torznab_categories. Uses
//! INSERT OR IGNORE so re-runs are idempotent (existing rows are preserved).

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Result of running seed operations.
#[derive(Debug, Default)]
pub struct SeedResult {
    pub tables_seeded: Vec<String>,
    pub errors: Vec<String>,
}

/// Seed default application settings (torrent, LLM, playback, metadata, etc.).
async fn seed_app_settings(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    #[derive(Debug)]
    struct SettingRow {
        key: &'static str,
        value: &'static str,
        description: &'static str,
        category: &'static str,
    }

    let rows: &[SettingRow] = &[
        // Torrent
        SettingRow {
            key: "torrent.download_dir",
            value: "\"/data/downloads\"",
            description: "Directory where torrents are downloaded to",
            category: "torrent",
        },
        SettingRow {
            key: "torrent.session_dir",
            value: "\"/data/session\"",
            description: "Directory for torrent session data",
            category: "torrent",
        },
        SettingRow {
            key: "torrent.enable_dht",
            value: "true",
            description: "Enable DHT for peer discovery",
            category: "torrent",
        },
        SettingRow {
            key: "torrent.listen_port",
            value: "6881",
            description: "Port for incoming torrent connections",
            category: "torrent",
        },
        SettingRow {
            key: "torrent.max_concurrent",
            value: "5",
            description: "Maximum concurrent downloads",
            category: "torrent",
        },
        SettingRow {
            key: "torrent.upload_limit",
            value: "0",
            description: "Upload speed limit in bytes/sec (0 = unlimited)",
            category: "torrent",
        },
        SettingRow {
            key: "torrent.download_limit",
            value: "0",
            description: "Download speed limit in bytes/sec (0 = unlimited)",
            category: "torrent",
        },
        // LLM
        SettingRow {
            key: "llm.enabled",
            value: "false",
            description: "Enable LLM-based filename parsing as fallback when regex parser fails or has low confidence",
            category: "llm",
        },
        SettingRow {
            key: "llm.ollama_url",
            value: "\"http://localhost:11434\"",
            description: "URL of the Ollama API server",
            category: "llm",
        },
        SettingRow {
            key: "llm.ollama_model",
            value: "\"qwen2.5-coder:7b\"",
            description: "Ollama model to use for parsing",
            category: "llm",
        },
        SettingRow {
            key: "llm.timeout_seconds",
            value: "30",
            description: "Timeout in seconds for LLM API calls",
            category: "llm",
        },
        SettingRow {
            key: "llm.temperature",
            value: "0.1",
            description: "Temperature for LLM generation (lower = more deterministic)",
            category: "llm",
        },
        SettingRow {
            key: "llm.prompt_template",
            value: "null",
            description: "Custom prompt template for LLM parsing (null = use default)",
            category: "llm",
        },
        SettingRow {
            key: "llm.max_retries",
            value: "2",
            description: "Maximum number of retries for failed LLM calls",
            category: "llm",
        },
        SettingRow {
            key: "llm.use_for_ambiguous",
            value: "true",
            description: "Use LLM for ambiguous filenames even when regex succeeds with low confidence",
            category: "llm",
        },
        // LLM per-library
        SettingRow {
            key: "llm.model.movies",
            value: "null",
            description: "Ollama model for movie libraries (null = use default)",
            category: "llm",
        },
        SettingRow {
            key: "llm.model.tv",
            value: "null",
            description: "Ollama model for TV show libraries (null = use default)",
            category: "llm",
        },
        SettingRow {
            key: "llm.model.music",
            value: "null",
            description: "Ollama model for music libraries (null = use default)",
            category: "llm",
        },
        SettingRow {
            key: "llm.model.audiobooks",
            value: "null",
            description: "Ollama model for audiobook libraries (null = use default)",
            category: "llm",
        },
        SettingRow {
            key: "llm.prompt.movies",
            value: "null",
            description: "Prompt template for movie libraries (null = use default)",
            category: "llm",
        },
        SettingRow {
            key: "llm.prompt.tv",
            value: "null",
            description: "Prompt template for TV show libraries (null = use default)",
            category: "llm",
        },
        SettingRow {
            key: "llm.prompt.music",
            value: "null",
            description: "Prompt template for music libraries (null = use default)",
            category: "llm",
        },
        SettingRow {
            key: "llm.prompt.audiobooks",
            value: "null",
            description: "Prompt template for audiobook libraries (null = use default)",
            category: "llm",
        },
        // Playback
        SettingRow {
            key: "playback_sync_interval",
            value: "15",
            description: "How often to sync watch progress to the database (in seconds)",
            category: "playback",
        },
        // Metadata
        SettingRow {
            key: "metadata.tmdb_api_key",
            value: "null",
            description: "TMDB API key for movie/TV metadata",
            category: "metadata",
        },
        SettingRow {
            key: "metadata.tvdb_api_key",
            value: "null",
            description: "TVDB API key for TV show metadata",
            category: "metadata",
        },
        SettingRow {
            key: "metadata.auto_fetch",
            value: "true",
            description: "Automatically fetch metadata when adding new media",
            category: "metadata",
        },
        SettingRow {
            key: "metadata.preferred_language",
            value: "\"en\"",
            description: "Preferred language for metadata",
            category: "metadata",
        },
        // Subtitles
        SettingRow {
            key: "subtitles.auto_download",
            value: "false",
            description: "Automatically download subtitles for new media",
            category: "subtitles",
        },
        SettingRow {
            key: "subtitles.preferred_languages",
            value: "[\"en\"]",
            description: "Preferred subtitle languages (JSON array)",
            category: "subtitles",
        },
        SettingRow {
            key: "subtitles.opensubtitles_api_key",
            value: "null",
            description: "OpenSubtitles API key",
            category: "subtitles",
        },
        // Organize
        SettingRow {
            key: "organize.auto_organize",
            value: "false",
            description: "Automatically organize files after download completes",
            category: "organize",
        },
        SettingRow {
            key: "organize.delete_empty_folders",
            value: "true",
            description: "Delete empty folders after organizing",
            category: "organize",
        },
        SettingRow {
            key: "organize.copy_mode",
            value: "\"copy\"",
            description: "File operation mode: copy, move, or hardlink",
            category: "organize",
        },
    ];

    let mut inserted = 0u64;

    for row in rows {
        let id = Uuid::new_v4().to_string();
        let r = sqlx::query(
            r#"INSERT OR IGNORE INTO app_settings (id, key, value, description, category)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(row.key)
        .bind(row.value)
        .bind(row.description)
        .bind(row.category)
        .execute(pool)
        .await?;

        if r.rows_affected() > 0 {
            inserted += 1;
        }
    }

    Ok(inserted)
}

/// Fixed ID for the single default cast_settings row so INSERT OR IGNORE is idempotent.
const DEFAULT_CAST_SETTINGS_ID: &str = "00000000-0000-0000-0000-000000000001";

/// Seed default cast settings (one row).
async fn seed_cast_settings(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let r = sqlx::query(
        r#"INSERT OR IGNORE INTO cast_settings
           (id, auto_discovery_enabled, discovery_interval_seconds, default_volume, transcode_incompatible, preferred_quality)
           VALUES (?, 1, 30, 1.0, 1, '1080p')"#,
    )
    .bind(DEFAULT_CAST_SETTINGS_ID)
    .execute(pool)
    .await?;

    Ok(r.rows_affected())
}

/// Seed default naming patterns (system patterns for tv, movies, music, audiobooks, other).
async fn seed_naming_patterns(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    #[derive(Debug)]
    struct PatternRow {
        id: &'static str,
        library_type: &'static str,
        name: &'static str,
        pattern: &'static str,
        description: &'static str,
        is_default: bool,
    }

    let rows: &[PatternRow] = &[
        // TV
        PatternRow {
            id: "00000000-0000-0000-0000-000000000001",
            library_type: "tv",
            name: "Standard",
            pattern: "{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}",
            description: "Show/Season 01/Show - S01E01 - Title.ext",
            is_default: true,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000002",
            library_type: "tv",
            name: "Plex Style",
            pattern: "{show}/Season {season:02}/{show} - s{season:02}e{episode:02} - {title}.{ext}",
            description: "Lowercase season/episode (Plex compatible)",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000003",
            library_type: "tv",
            name: "Compact",
            pattern: "{show}/S{season:02}/{show}.S{season:02}E{episode:02}.{ext}",
            description: "Compact format without episode title",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000004",
            library_type: "tv",
            name: "Scene Style",
            pattern: "{show}/Season {season:02}/{show}.S{season:02}E{episode:02}.{title}.{ext}",
            description: "Dots instead of spaces (scene style)",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000005",
            library_type: "tv",
            name: "Jellyfin",
            pattern: "{show}/Season {season}/{show} S{season:02}E{episode:02} {title}.{ext}",
            description: "Jellyfin recommended format",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000006",
            library_type: "tv",
            name: "Simple",
            pattern: "{show}/Season {season:02}/{season:02}x{episode:02} - {title}.{ext}",
            description: "Simple 01x01 format",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000007",
            library_type: "tv",
            name: "Flat",
            pattern: "{show} - S{season:02}E{episode:02} - {title}.{ext}",
            description: "All files in show folder (no season folders)",
            is_default: false,
        },
        // Movies
        PatternRow {
            id: "00000000-0000-0000-0000-000000000011",
            library_type: "movies",
            name: "Movie Standard",
            pattern: "{title} ({year})/{title} ({year}).{ext}",
            description: "Title (Year)/Title (Year).ext",
            is_default: true,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000012",
            library_type: "movies",
            name: "Movie with Quality",
            pattern: "{title} ({year})/{title} ({year}) - {quality}.{ext}",
            description: "Include quality in filename",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000013",
            library_type: "movies",
            name: "Flat Movies",
            pattern: "{title} ({year}).{ext}",
            description: "All movies in root folder",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000014",
            library_type: "movies",
            name: "Plex Movie",
            pattern: "{title} ({year})/{title} ({year}) [{quality}].{ext}",
            description: "Plex style with quality",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000015",
            library_type: "movies",
            name: "Jellyfin Movie",
            pattern: "{title} ({year})/{title}.{ext}",
            description: "Jellyfin recommended format",
            is_default: false,
        },
        // Music
        PatternRow {
            id: "00000000-0000-0000-0000-000000000021",
            library_type: "music",
            name: "Music Standard",
            pattern: "{artist}/{album} ({year})/{track:02} - {title}.{ext}",
            description: "Artist/Album (Year)/01 - Title.ext",
            is_default: true,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000022",
            library_type: "music",
            name: "Music with Disc",
            pattern: "{artist}/{album} ({year})/Disc {disc}/{track:02} - {title}.{ext}",
            description: "Include disc number folder",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000023",
            library_type: "music",
            name: "Artist Only",
            pattern: "{artist}/{track:02} - {title}.{ext}",
            description: "All tracks in artist folder",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000024",
            library_type: "music",
            name: "Album Only",
            pattern: "{album} ({year})/{track:02} - {title}.{ext}",
            description: "Albums in root, no artist folder",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000025",
            library_type: "music",
            name: "Full Track Info",
            pattern: "{artist}/{album} ({year})/{track:02} - {artist} - {title}.{ext}",
            description: "Include artist in track filename",
            is_default: false,
        },
        // Audiobooks
        PatternRow {
            id: "00000000-0000-0000-0000-000000000031",
            library_type: "audiobooks",
            name: "Audiobook Standard",
            pattern: "{author}/{title}/{chapter:02} - {chapter_title}.{ext}",
            description: "Author/Title/01 - Chapter.ext",
            is_default: true,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000032",
            library_type: "audiobooks",
            name: "Audiobook Series",
            pattern: "{author}/{series} {series_position} - {title}/{chapter:02} - {chapter_title}.{ext}",
            description: "Include series info",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000033",
            library_type: "audiobooks",
            name: "Audiobook Simple",
            pattern: "{author}/{title}/{chapter:02}.{ext}",
            description: "Simple chapter numbering",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000034",
            library_type: "audiobooks",
            name: "Audiobook Flat",
            pattern: "{author}/{title}.{ext}",
            description: "Single file audiobooks",
            is_default: false,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000035",
            library_type: "audiobooks",
            name: "Plex Audiobook",
            pattern: "{author}/{title}/{title} - Chapter {chapter:02}.{ext}",
            description: "Plex audiobook format",
            is_default: false,
        },
        // Other
        PatternRow {
            id: "00000000-0000-0000-0000-000000000041",
            library_type: "other",
            name: "Generic Preserve",
            pattern: "{name}.{ext}",
            description: "Keep original filename",
            is_default: true,
        },
        PatternRow {
            id: "00000000-0000-0000-0000-000000000042",
            library_type: "other",
            name: "Generic Folder",
            pattern: "{name}/{name}.{ext}",
            description: "Each file in its own folder",
            is_default: false,
        },
    ];

    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let user_id = "system";
    let mut inserted = 0u64;

    for row in rows {
        let r = sqlx::query(
            r#"INSERT OR IGNORE INTO naming_patterns
               (id, user_id, library_type, name, pattern, description, is_default, is_system, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?)"#,
        )
        .bind(row.id)
        .bind(user_id)
        .bind(row.library_type)
        .bind(row.name)
        .bind(row.pattern)
        .bind(row.description)
        .bind(if row.is_default { 1i32 } else { 0i32 })
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        if r.rows_affected() > 0 {
            inserted += 1;
        }
    }

    Ok(inserted)
}

/// Seed Torznab categories (Newznab-style category tree).
async fn seed_torznab_categories(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    #[derive(Debug)]
    struct CategoryRow {
        id: i32,
        name: &'static str,
        parent_id: Option<i32>,
        description: Option<&'static str>,
    }

    let rows: &[CategoryRow] = &[
        CategoryRow {
            id: 1000,
            name: "Console",
            parent_id: None,
            description: Some("Console games"),
        },
        CategoryRow {
            id: 2000,
            name: "Movies",
            parent_id: None,
            description: Some("Movies"),
        },
        CategoryRow {
            id: 3000,
            name: "Audio",
            parent_id: None,
            description: Some("Audio/Music"),
        },
        CategoryRow {
            id: 4000,
            name: "PC",
            parent_id: None,
            description: Some("PC software and games"),
        },
        CategoryRow {
            id: 5000,
            name: "TV",
            parent_id: None,
            description: Some("TV shows"),
        },
        CategoryRow {
            id: 6000,
            name: "XXX",
            parent_id: None,
            description: Some("Adult content"),
        },
        CategoryRow {
            id: 7000,
            name: "Books",
            parent_id: None,
            description: Some("Books and comics"),
        },
        CategoryRow {
            id: 8000,
            name: "Other",
            parent_id: None,
            description: Some("Other/Misc"),
        },
        CategoryRow {
            id: 2010,
            name: "Movies/Foreign",
            parent_id: Some(2000),
            description: Some("Foreign movies"),
        },
        CategoryRow {
            id: 2020,
            name: "Movies/Other",
            parent_id: Some(2000),
            description: Some("Other movies"),
        },
        CategoryRow {
            id: 2030,
            name: "Movies/SD",
            parent_id: Some(2000),
            description: Some("SD quality movies"),
        },
        CategoryRow {
            id: 2040,
            name: "Movies/HD",
            parent_id: Some(2000),
            description: Some("HD quality movies"),
        },
        CategoryRow {
            id: 2045,
            name: "Movies/UHD",
            parent_id: Some(2000),
            description: Some("4K/UHD movies"),
        },
        CategoryRow {
            id: 2050,
            name: "Movies/BluRay",
            parent_id: Some(2000),
            description: Some("BluRay movies"),
        },
        CategoryRow {
            id: 2060,
            name: "Movies/3D",
            parent_id: Some(2000),
            description: Some("3D movies"),
        },
        CategoryRow {
            id: 2070,
            name: "Movies/DVD",
            parent_id: Some(2000),
            description: Some("DVD movies"),
        },
        CategoryRow {
            id: 2080,
            name: "Movies/WEB-DL",
            parent_id: Some(2000),
            description: Some("WEB-DL movies"),
        },
        CategoryRow {
            id: 5010,
            name: "TV/WEB-DL",
            parent_id: Some(5000),
            description: Some("WEB-DL TV shows"),
        },
        CategoryRow {
            id: 5020,
            name: "TV/Foreign",
            parent_id: Some(5000),
            description: Some("Foreign TV shows"),
        },
        CategoryRow {
            id: 5030,
            name: "TV/SD",
            parent_id: Some(5000),
            description: Some("SD TV shows"),
        },
        CategoryRow {
            id: 5040,
            name: "TV/HD",
            parent_id: Some(5000),
            description: Some("HD TV shows"),
        },
        CategoryRow {
            id: 5045,
            name: "TV/UHD",
            parent_id: Some(5000),
            description: Some("4K/UHD TV shows"),
        },
        CategoryRow {
            id: 5050,
            name: "TV/Other",
            parent_id: Some(5000),
            description: Some("Other TV shows"),
        },
        CategoryRow {
            id: 5060,
            name: "TV/Sport",
            parent_id: Some(5000),
            description: Some("Sports"),
        },
        CategoryRow {
            id: 5070,
            name: "TV/Anime",
            parent_id: Some(5000),
            description: Some("Anime"),
        },
        CategoryRow {
            id: 5080,
            name: "TV/Documentary",
            parent_id: Some(5000),
            description: Some("Documentaries"),
        },
        CategoryRow {
            id: 3010,
            name: "Audio/MP3",
            parent_id: Some(3000),
            description: Some("MP3 audio"),
        },
        CategoryRow {
            id: 3020,
            name: "Audio/Video",
            parent_id: Some(3000),
            description: Some("Music videos"),
        },
        CategoryRow {
            id: 3030,
            name: "Audio/Audiobook",
            parent_id: Some(3000),
            description: Some("Audiobooks"),
        },
        CategoryRow {
            id: 3040,
            name: "Audio/Lossless",
            parent_id: Some(3000),
            description: Some("Lossless audio"),
        },
        CategoryRow {
            id: 3050,
            name: "Audio/Other",
            parent_id: Some(3000),
            description: Some("Other audio"),
        },
        CategoryRow {
            id: 3060,
            name: "Audio/Foreign",
            parent_id: Some(3000),
            description: Some("Foreign audio"),
        },
        CategoryRow {
            id: 7010,
            name: "Books/Mags",
            parent_id: Some(7000),
            description: Some("Magazines"),
        },
        CategoryRow {
            id: 7020,
            name: "Books/EBook",
            parent_id: Some(7000),
            description: Some("E-Books"),
        },
        CategoryRow {
            id: 7030,
            name: "Books/Comics",
            parent_id: Some(7000),
            description: Some("Comics"),
        },
        CategoryRow {
            id: 7040,
            name: "Books/Technical",
            parent_id: Some(7000),
            description: Some("Technical books"),
        },
        CategoryRow {
            id: 7050,
            name: "Books/Other",
            parent_id: Some(7000),
            description: Some("Other books"),
        },
        CategoryRow {
            id: 7060,
            name: "Books/Foreign",
            parent_id: Some(7000),
            description: Some("Foreign books"),
        },
        CategoryRow {
            id: 4010,
            name: "PC/0day",
            parent_id: Some(4000),
            description: Some("0-day releases"),
        },
        CategoryRow {
            id: 4020,
            name: "PC/ISO",
            parent_id: Some(4000),
            description: Some("ISO images"),
        },
        CategoryRow {
            id: 4030,
            name: "PC/Mac",
            parent_id: Some(4000),
            description: Some("Mac software"),
        },
        CategoryRow {
            id: 4040,
            name: "PC/Mobile-Other",
            parent_id: Some(4000),
            description: Some("Mobile software"),
        },
        CategoryRow {
            id: 4050,
            name: "PC/Games",
            parent_id: Some(4000),
            description: Some("PC games"),
        },
        CategoryRow {
            id: 4060,
            name: "PC/Mobile-iOS",
            parent_id: Some(4000),
            description: Some("iOS apps"),
        },
        CategoryRow {
            id: 4070,
            name: "PC/Mobile-Android",
            parent_id: Some(4000),
            description: Some("Android apps"),
        },
        CategoryRow {
            id: 1010,
            name: "Console/NDS",
            parent_id: Some(1000),
            description: Some("Nintendo DS"),
        },
        CategoryRow {
            id: 1020,
            name: "Console/PSP",
            parent_id: Some(1000),
            description: Some("PlayStation Portable"),
        },
        CategoryRow {
            id: 1030,
            name: "Console/Wii",
            parent_id: Some(1000),
            description: Some("Nintendo Wii"),
        },
        CategoryRow {
            id: 1040,
            name: "Console/XBox",
            parent_id: Some(1000),
            description: Some("Xbox"),
        },
        CategoryRow {
            id: 1050,
            name: "Console/XBox 360",
            parent_id: Some(1000),
            description: Some("Xbox 360"),
        },
        CategoryRow {
            id: 1060,
            name: "Console/WiiWare",
            parent_id: Some(1000),
            description: Some("WiiWare"),
        },
        CategoryRow {
            id: 1070,
            name: "Console/XBox 360 DLC",
            parent_id: Some(1000),
            description: Some("Xbox 360 DLC"),
        },
        CategoryRow {
            id: 1080,
            name: "Console/PS3",
            parent_id: Some(1000),
            description: Some("PlayStation 3"),
        },
        CategoryRow {
            id: 1090,
            name: "Console/Other",
            parent_id: Some(1000),
            description: Some("Other consoles"),
        },
        CategoryRow {
            id: 1110,
            name: "Console/3DS",
            parent_id: Some(1000),
            description: Some("Nintendo 3DS"),
        },
        CategoryRow {
            id: 1120,
            name: "Console/PS Vita",
            parent_id: Some(1000),
            description: Some("PlayStation Vita"),
        },
        CategoryRow {
            id: 1130,
            name: "Console/WiiU",
            parent_id: Some(1000),
            description: Some("Wii U"),
        },
        CategoryRow {
            id: 1140,
            name: "Console/XBox One",
            parent_id: Some(1000),
            description: Some("Xbox One"),
        },
        CategoryRow {
            id: 1150,
            name: "Console/PS4",
            parent_id: Some(1000),
            description: Some("PlayStation 4"),
        },
        CategoryRow {
            id: 1180,
            name: "Console/Switch",
            parent_id: Some(1000),
            description: Some("Nintendo Switch"),
        },
    ];

    let mut inserted = 0u64;
    for row in rows {
        let r = sqlx::query(
            r#"INSERT OR IGNORE INTO torznab_categories (id, name, parent_id, description) VALUES (?, ?, ?, ?)"#,
        )
        .bind(row.id)
        .bind(row.name)
        .bind(row.parent_id)
        .bind(row.description)
        .execute(pool)
        .await?;

        if r.rows_affected() > 0 {
            inserted += 1;
        }
    }

    Ok(inserted)
}

/// Run all seed routines. Safe to call multiple times (uses INSERT OR IGNORE).
pub async fn run_seeds(pool: &SqlitePool) -> SeedResult {
    let mut result = SeedResult::default();

    for (table, count) in [
        ("app_settings", seed_app_settings(pool).await),
        ("cast_settings", seed_cast_settings(pool).await),
        ("naming_patterns", seed_naming_patterns(pool).await),
        ("torznab_categories", seed_torznab_categories(pool).await),
    ] {
        match count {
            Ok(n) => {
                if n > 0 {
                    debug!(table = table, count = n, "Seeded table");
                    result.tables_seeded.push(format!("{} ({} rows)", table, n));
                }
            }
            Err(e) => {
                let msg = format!("Seed {}: {}", table, e);
                warn!("{}", msg);
                result.errors.push(msg);
            }
        }
    }

    if !result.tables_seeded.is_empty() {
        info!(tables = ?result.tables_seeded, "Pre-seed data applied");
    }

    result
}
