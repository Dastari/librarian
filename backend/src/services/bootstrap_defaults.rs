use std::collections::HashSet;

use anyhow::Result;

use crate::db::Database;
use crate::services::graphql::entities::*;

struct AppSettingSeed {
    key: &'static str,
    value: &'static str,
    description: &'static str,
    category: &'static str,
}

struct NamingPatternSeed {
    user_id: &'static str,
    library_type: &'static str,
    name: &'static str,
    pattern: &'static str,
    description: &'static str,
    is_default: bool,
}

struct TorznabCategorySeed {
    id: &'static str,
    name: &'static str,
    parent_id: Option<&'static str>,
    description: &'static str,
}

const APP_SETTINGS: &[AppSettingSeed] = &[
    AppSettingSeed {
        key: "torrent.download_dir",
        value: "\"/data/downloads\"",
        description: "Directory where torrents are downloaded to",
        category: "torrent",
    },
    AppSettingSeed {
        key: "torrent.session_dir",
        value: "\"/data/session\"",
        description: "Directory for torrent session data",
        category: "torrent",
    },
    AppSettingSeed {
        key: "torrent.enable_dht",
        value: "true",
        description: "Enable DHT for peer discovery",
        category: "torrent",
    },
    AppSettingSeed {
        key: "torrent.listen_port",
        value: "6881",
        description: "Port for incoming torrent connections",
        category: "torrent",
    },
    AppSettingSeed {
        key: "torrent.max_concurrent",
        value: "5",
        description: "Maximum concurrent downloads",
        category: "torrent",
    },
    AppSettingSeed {
        key: "torrent.upload_limit",
        value: "0",
        description: "Upload speed limit in bytes/sec (0 = unlimited)",
        category: "torrent",
    },
    AppSettingSeed {
        key: "torrent.download_limit",
        value: "0",
        description: "Download speed limit in bytes/sec (0 = unlimited)",
        category: "torrent",
    },
    AppSettingSeed {
        key: "extract.max_unpacked_gb",
        value: "60",
        description: "Largest total size an archive from a download may unpack to, in GB",
        category: "torrent",
    },
    AppSettingSeed {
        key: "extract.max_entries",
        value: "20000",
        description: "Maximum number of entries an archive from a download may contain",
        category: "torrent",
    },
    AppSettingSeed {
        key: "torrent.seed_ratio_limit",
        value: "1.0",
        description: "Stop seeding once this share ratio is reached (0 = no ratio limit)",
        category: "torrent",
    },
    AppSettingSeed {
        key: "torrent.seed_time_minutes",
        value: "0",
        description: "Stop seeding after this many minutes of seeding (0 = no time limit)",
        category: "torrent",
    },
    AppSettingSeed {
        key: "torrent.remove_after_import",
        value: "false",
        description: "Remove the torrent and delete its downloaded files once the import completed and the seeding rules are satisfied",
        category: "torrent",
    },
    AppSettingSeed {
        key: "llm.enabled",
        value: "false",
        description: "Enable LLM-based filename parsing as fallback when regex parser fails or has low confidence",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.ollama_url",
        value: "\"http://localhost:11434\"",
        description: "URL of the Ollama API server",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.ollama_model",
        value: "\"qwen2.5-coder:7b\"",
        description: "Ollama model to use for parsing",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.timeout_seconds",
        value: "30",
        description: "Timeout in seconds for LLM API calls",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.temperature",
        value: "0.1",
        description: "Temperature for LLM generation (lower = more deterministic)",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.max_tokens",
        value: "256",
        description: "Maximum number of tokens to request from the LLM parser",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.confidence_threshold",
        value: "0.7",
        description: "Minimum deterministic parser confidence before LLM fallback is skipped",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.prompt_template",
        value: "null",
        description: "Custom prompt template for LLM parsing (null = use default)",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.max_retries",
        value: "2",
        description: "Maximum number of retries for failed LLM calls",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.use_for_ambiguous",
        value: "true",
        description: "Use LLM for ambiguous filenames even when regex succeeds with low confidence",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.model.movies",
        value: "null",
        description: "Ollama model for movie libraries (null = use default)",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.model.tv",
        value: "null",
        description: "Ollama model for TV show libraries (null = use default)",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.model.music",
        value: "null",
        description: "Ollama model for music libraries (null = use default)",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.model.audiobooks",
        value: "null",
        description: "Ollama model for audiobook libraries (null = use default)",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.prompt.movies",
        value: "null",
        description: "Prompt template for movie libraries (null = use default)",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.prompt.tv",
        value: "null",
        description: "Prompt template for TV show libraries (null = use default)",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.prompt.music",
        value: "null",
        description: "Prompt template for music libraries (null = use default)",
        category: "llm",
    },
    AppSettingSeed {
        key: "llm.prompt.audiobooks",
        value: "null",
        description: "Prompt template for audiobook libraries (null = use default)",
        category: "llm",
    },
    AppSettingSeed {
        key: "playback_sync_interval",
        value: "15",
        description: "How often to sync watch progress to the database (in seconds)",
        category: "playback",
    },
    AppSettingSeed {
        key: "metadata.tmdb_api_key",
        value: "null",
        description: "TMDB API key for movie/TV metadata",
        category: "metadata",
    },
    AppSettingSeed {
        key: "metadata.tvdb_api_key",
        value: "null",
        description: "TVDB API key for TV show metadata",
        category: "metadata",
    },
    AppSettingSeed {
        key: "metadata.auto_fetch",
        value: "true",
        description: "Automatically fetch metadata when adding new media",
        category: "metadata",
    },
    AppSettingSeed {
        key: "metadata.preferred_language",
        value: "\"en\"",
        description: "Preferred language for metadata",
        category: "metadata",
    },
    AppSettingSeed {
        key: "subtitles.preferred_languages",
        value: "[\"en\"]",
        description: "Preferred languages for cataloged subtitle streams (JSON array)",
        category: "subtitles",
    },
    AppSettingSeed {
        key: "organize.auto_organize",
        value: "false",
        description: "Automatically organize files after download completes",
        category: "organize",
    },
    AppSettingSeed {
        key: "organize.delete_empty_folders",
        value: "true",
        description: "Delete empty folders after organizing",
        category: "organize",
    },
    AppSettingSeed {
        key: "organize.copy_mode",
        value: "\"copy\"",
        description: "File operation mode: copy, move, or hardlink",
        category: "organize",
    },
];

const NAMING_PATTERNS: &[NamingPatternSeed] = &[
    NamingPatternSeed {
        user_id: "system",
        library_type: "tv",
        name: "Standard",
        pattern: "{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}",
        description: "Show/Season 01/Show - S01E01 - Title.ext",
        is_default: true,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "tv",
        name: "Plex Style",
        pattern: "{show}/Season {season:02}/{show} - s{season:02}e{episode:02} - {title}.{ext}",
        description: "Lowercase season/episode (Plex compatible)",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "tv",
        name: "Compact",
        pattern: "{show}/S{season:02}/{show}.S{season:02}E{episode:02}.{ext}",
        description: "Compact format without episode title",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "tv",
        name: "Scene Style",
        pattern: "{show}/Season {season:02}/{show}.S{season:02}E{episode:02}.{title}.{ext}",
        description: "Dots instead of spaces (scene style)",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "tv",
        name: "Jellyfin",
        pattern: "{show}/Season {season}/{show} S{season:02}E{episode:02} {title}.{ext}",
        description: "Jellyfin recommended format",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "tv",
        name: "Simple",
        pattern: "{show}/Season {season:02}/{season:02}x{episode:02} - {title}.{ext}",
        description: "Simple 01x01 format",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "tv",
        name: "Flat",
        pattern: "{show} - S{season:02}E{episode:02} - {title}.{ext}",
        description: "All files in show folder (no season folders)",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "movies",
        name: "Movie Standard",
        pattern: "{title} ({year})/{title} ({year}).{ext}",
        description: "Title (Year)/Title (Year).ext",
        is_default: true,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "movies",
        name: "Movie with Quality",
        pattern: "{title} ({year})/{title} ({year}) - {quality}.{ext}",
        description: "Include quality in filename",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "movies",
        name: "Flat Movies",
        pattern: "{title} ({year}).{ext}",
        description: "All movies in root folder",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "movies",
        name: "Plex Movie",
        pattern: "{title} ({year})/{title} ({year}) [{quality}].{ext}",
        description: "Plex style with quality",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "movies",
        name: "Jellyfin Movie",
        pattern: "{title} ({year})/{title}.{ext}",
        description: "Jellyfin recommended format",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "music",
        name: "Music Standard",
        pattern: "{artist}/{album} ({year})/{track:02} - {title}.{ext}",
        description: "Artist/Album (Year)/01 - Title.ext",
        is_default: true,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "music",
        name: "Music with Disc",
        pattern: "{artist}/{album} ({year})/Disc {disc}/{track:02} - {title}.{ext}",
        description: "Include disc number folder",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "music",
        name: "Artist Only",
        pattern: "{artist}/{track:02} - {title}.{ext}",
        description: "All tracks in artist folder",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "music",
        name: "Album Only",
        pattern: "{album} ({year})/{track:02} - {title}.{ext}",
        description: "Albums in root, no artist folder",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "music",
        name: "Full Track Info",
        pattern: "{artist}/{album} ({year})/{track:02} - {artist} - {title}.{ext}",
        description: "Include artist in track filename",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "audiobooks",
        name: "Audiobook Standard",
        pattern: "{author}/{title}/{chapter:02} - {chapter_title}.{ext}",
        description: "Author/Title/01 - Chapter.ext",
        is_default: true,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "audiobooks",
        name: "Audiobook Series",
        pattern: "{author}/{series} {series_position} - {title}/{chapter:02} - {chapter_title}.{ext}",
        description: "Include series info",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "audiobooks",
        name: "Audiobook Simple",
        pattern: "{author}/{title}/{chapter:02}.{ext}",
        description: "Simple chapter numbering",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "audiobooks",
        name: "Audiobook Flat",
        pattern: "{author}/{title}.{ext}",
        description: "Single file audiobooks",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "audiobooks",
        name: "Plex Audiobook",
        pattern: "{author}/{title}/{title} - Chapter {chapter:02}.{ext}",
        description: "Plex audiobook format",
        is_default: false,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "other",
        name: "Generic Preserve",
        pattern: "{name}.{ext}",
        description: "Keep original filename",
        is_default: true,
    },
    NamingPatternSeed {
        user_id: "system",
        library_type: "other",
        name: "Generic Folder",
        pattern: "{name}/{name}.{ext}",
        description: "Each file in its own folder",
        is_default: false,
    },
];

const TORZNAB_CATEGORIES: &[TorznabCategorySeed] = &[
    TorznabCategorySeed {
        id: "1000",
        name: "Console",
        parent_id: None,
        description: "Console games",
    },
    TorznabCategorySeed {
        id: "2000",
        name: "Movies",
        parent_id: None,
        description: "Movies",
    },
    TorznabCategorySeed {
        id: "3000",
        name: "Audio",
        parent_id: None,
        description: "Audio/Music",
    },
    TorznabCategorySeed {
        id: "4000",
        name: "PC",
        parent_id: None,
        description: "PC software and games",
    },
    TorznabCategorySeed {
        id: "5000",
        name: "TV",
        parent_id: None,
        description: "TV shows",
    },
    TorznabCategorySeed {
        id: "6000",
        name: "XXX",
        parent_id: None,
        description: "Adult content",
    },
    TorznabCategorySeed {
        id: "7000",
        name: "Books",
        parent_id: None,
        description: "Books and comics",
    },
    TorznabCategorySeed {
        id: "8000",
        name: "Other",
        parent_id: None,
        description: "Other/Misc",
    },
    TorznabCategorySeed {
        id: "2010",
        name: "Movies/Foreign",
        parent_id: Some("2000"),
        description: "Foreign movies",
    },
    TorznabCategorySeed {
        id: "2020",
        name: "Movies/Other",
        parent_id: Some("2000"),
        description: "Other movies",
    },
    TorznabCategorySeed {
        id: "2030",
        name: "Movies/SD",
        parent_id: Some("2000"),
        description: "SD quality movies",
    },
    TorznabCategorySeed {
        id: "2040",
        name: "Movies/HD",
        parent_id: Some("2000"),
        description: "HD quality movies",
    },
    TorznabCategorySeed {
        id: "2045",
        name: "Movies/UHD",
        parent_id: Some("2000"),
        description: "4K/UHD movies",
    },
    TorznabCategorySeed {
        id: "2050",
        name: "Movies/BluRay",
        parent_id: Some("2000"),
        description: "BluRay movies",
    },
    TorznabCategorySeed {
        id: "2060",
        name: "Movies/3D",
        parent_id: Some("2000"),
        description: "3D movies",
    },
    TorznabCategorySeed {
        id: "2070",
        name: "Movies/DVD",
        parent_id: Some("2000"),
        description: "DVD movies",
    },
    TorznabCategorySeed {
        id: "2080",
        name: "Movies/WEB-DL",
        parent_id: Some("2000"),
        description: "WEB-DL movies",
    },
    TorznabCategorySeed {
        id: "5010",
        name: "TV/WEB-DL",
        parent_id: Some("5000"),
        description: "WEB-DL TV shows",
    },
    TorznabCategorySeed {
        id: "5020",
        name: "TV/Foreign",
        parent_id: Some("5000"),
        description: "Foreign TV shows",
    },
    TorznabCategorySeed {
        id: "5030",
        name: "TV/SD",
        parent_id: Some("5000"),
        description: "SD TV shows",
    },
    TorznabCategorySeed {
        id: "5040",
        name: "TV/HD",
        parent_id: Some("5000"),
        description: "HD TV shows",
    },
    TorznabCategorySeed {
        id: "5045",
        name: "TV/UHD",
        parent_id: Some("5000"),
        description: "4K/UHD TV shows",
    },
    TorznabCategorySeed {
        id: "5050",
        name: "TV/Other",
        parent_id: Some("5000"),
        description: "Other TV shows",
    },
    TorznabCategorySeed {
        id: "5060",
        name: "TV/Sport",
        parent_id: Some("5000"),
        description: "Sports",
    },
    TorznabCategorySeed {
        id: "5070",
        name: "TV/Anime",
        parent_id: Some("5000"),
        description: "Anime",
    },
    TorznabCategorySeed {
        id: "5080",
        name: "TV/Documentary",
        parent_id: Some("5000"),
        description: "Documentaries",
    },
    TorznabCategorySeed {
        id: "3010",
        name: "Audio/MP3",
        parent_id: Some("3000"),
        description: "MP3 audio",
    },
    TorznabCategorySeed {
        id: "3020",
        name: "Audio/Video",
        parent_id: Some("3000"),
        description: "Music videos",
    },
    TorznabCategorySeed {
        id: "3030",
        name: "Audio/Audiobook",
        parent_id: Some("3000"),
        description: "Audiobooks",
    },
    TorznabCategorySeed {
        id: "3040",
        name: "Audio/Lossless",
        parent_id: Some("3000"),
        description: "Lossless audio",
    },
    TorznabCategorySeed {
        id: "3050",
        name: "Audio/Other",
        parent_id: Some("3000"),
        description: "Other audio",
    },
    TorznabCategorySeed {
        id: "3060",
        name: "Audio/Foreign",
        parent_id: Some("3000"),
        description: "Foreign audio",
    },
    TorznabCategorySeed {
        id: "7010",
        name: "Books/Mags",
        parent_id: Some("7000"),
        description: "Magazines",
    },
    TorznabCategorySeed {
        id: "7020",
        name: "Books/EBook",
        parent_id: Some("7000"),
        description: "E-Books",
    },
    TorznabCategorySeed {
        id: "7030",
        name: "Books/Comics",
        parent_id: Some("7000"),
        description: "Comics",
    },
    TorznabCategorySeed {
        id: "7040",
        name: "Books/Technical",
        parent_id: Some("7000"),
        description: "Technical books",
    },
    TorznabCategorySeed {
        id: "7050",
        name: "Books/Other",
        parent_id: Some("7000"),
        description: "Other books",
    },
    TorznabCategorySeed {
        id: "7060",
        name: "Books/Foreign",
        parent_id: Some("7000"),
        description: "Foreign books",
    },
    TorznabCategorySeed {
        id: "4010",
        name: "PC/0day",
        parent_id: Some("4000"),
        description: "0-day releases",
    },
    TorznabCategorySeed {
        id: "4020",
        name: "PC/ISO",
        parent_id: Some("4000"),
        description: "ISO images",
    },
    TorznabCategorySeed {
        id: "4030",
        name: "PC/Mac",
        parent_id: Some("4000"),
        description: "Mac software",
    },
    TorznabCategorySeed {
        id: "4040",
        name: "PC/Mobile-Other",
        parent_id: Some("4000"),
        description: "Mobile software",
    },
    TorznabCategorySeed {
        id: "4050",
        name: "PC/Games",
        parent_id: Some("4000"),
        description: "PC games",
    },
    TorznabCategorySeed {
        id: "4060",
        name: "PC/Mobile-iOS",
        parent_id: Some("4000"),
        description: "iOS apps",
    },
    TorznabCategorySeed {
        id: "4070",
        name: "PC/Mobile-Android",
        parent_id: Some("4000"),
        description: "Android apps",
    },
    TorznabCategorySeed {
        id: "1010",
        name: "Console/NDS",
        parent_id: Some("1000"),
        description: "Nintendo DS",
    },
    TorznabCategorySeed {
        id: "1020",
        name: "Console/PSP",
        parent_id: Some("1000"),
        description: "PlayStation Portable",
    },
    TorznabCategorySeed {
        id: "1030",
        name: "Console/Wii",
        parent_id: Some("1000"),
        description: "Nintendo Wii",
    },
    TorznabCategorySeed {
        id: "1040",
        name: "Console/XBox",
        parent_id: Some("1000"),
        description: "Xbox",
    },
    TorznabCategorySeed {
        id: "1050",
        name: "Console/XBox 360",
        parent_id: Some("1000"),
        description: "Xbox 360",
    },
    TorznabCategorySeed {
        id: "1060",
        name: "Console/WiiWare",
        parent_id: Some("1000"),
        description: "WiiWare",
    },
    TorznabCategorySeed {
        id: "1070",
        name: "Console/XBox 360 DLC",
        parent_id: Some("1000"),
        description: "Xbox 360 DLC",
    },
    TorznabCategorySeed {
        id: "1080",
        name: "Console/PS3",
        parent_id: Some("1000"),
        description: "PlayStation 3",
    },
    TorznabCategorySeed {
        id: "1090",
        name: "Console/Other",
        parent_id: Some("1000"),
        description: "Other consoles",
    },
    TorznabCategorySeed {
        id: "1110",
        name: "Console/3DS",
        parent_id: Some("1000"),
        description: "Nintendo 3DS",
    },
    TorznabCategorySeed {
        id: "1120",
        name: "Console/PS Vita",
        parent_id: Some("1000"),
        description: "PlayStation Vita",
    },
    TorznabCategorySeed {
        id: "1130",
        name: "Console/WiiU",
        parent_id: Some("1000"),
        description: "Wii U",
    },
    TorznabCategorySeed {
        id: "1140",
        name: "Console/XBox One",
        parent_id: Some("1000"),
        description: "Xbox One",
    },
    TorznabCategorySeed {
        id: "1150",
        name: "Console/PS4",
        parent_id: Some("1000"),
        description: "PlayStation 4",
    },
    TorznabCategorySeed {
        id: "1180",
        name: "Console/Switch",
        parent_id: Some("1000"),
        description: "Nintendo Switch",
    },
];

pub async fn seed_defaults(db: &Database) -> Result<()> {
    seed_app_settings(db).await?;
    seed_cast_settings(db).await?;
    seed_naming_patterns(db).await?;
    seed_torznab_categories(db).await?;
    seed_quality_profiles(db).await?;
    Ok(())
}

#[cfg(test)]
#[path = "bootstrap_defaults/tests.rs"]
mod tests;

async fn seed_app_settings(db: &Database) -> Result<()> {
    let existing = AppSetting::query(db.pool())
        .fetch_all()
        .await?
        .into_iter()
        .map(|setting| setting.key)
        .collect::<HashSet<_>>();

    for seed in APP_SETTINGS {
        if existing.contains(seed.key) {
            continue;
        }

        AppSetting::insert(
            db,
            CreateAppSettingInput {
                key: seed.key.to_string(),
                value: seed.value.to_string(),
                description: Some(seed.description.to_string()),
                category: seed.category.to_string(),
            },
        )
        .await?;
    }

    Ok(())
}

async fn seed_cast_settings(db: &Database) -> Result<()> {
    if !CastSetting::query(db.pool()).fetch_all().await?.is_empty() {
        return Ok(());
    }

    CastSetting::insert(
        db,
        CreateCastSettingInput {
            auto_discovery_enabled: true,
            discovery_interval_seconds: 30,
            default_volume: 1.0,
            transcode_incompatible: true,
            preferred_quality: Some("1080p".to_string()),
        },
    )
    .await?;

    Ok(())
}

async fn seed_naming_patterns(db: &Database) -> Result<()> {
    let existing = NamingPattern::query(db.pool())
        .fetch_all()
        .await?
        .into_iter()
        .map(|pattern| (pattern.library_type, pattern.name))
        .collect::<HashSet<_>>();

    for seed in NAMING_PATTERNS {
        let key = (seed.library_type.to_string(), seed.name.to_string());
        if existing.contains(&key) {
            continue;
        }

        NamingPattern::insert(
            db,
            CreateNamingPatternInput {
                user_id: seed.user_id.to_string(),
                library_type: seed.library_type.to_string(),
                name: seed.name.to_string(),
                pattern: seed.pattern.to_string(),
                description: Some(seed.description.to_string()),
                is_default: seed.is_default,
                is_system: true,
            },
        )
        .await?;
    }

    Ok(())
}

/// Seed the default "Any Quality" profile (docs/tier1-features-plan.md §2):
/// a `QualityProfile` with every allow-list empty (`allows_any()` → always
/// `Optimal`, design.md Q26), used as the last resort when neither a
/// per-entity override nor `Library.qualityProfileId` resolves to a real
/// profile.
///
/// The acquisition-side rules are seeded to their safe defaults: `minSeeders`
/// 1 (never grab a dead torrent), `allowSeasonPacks` true and
/// `preferProperRepack` true. Everything else is unrestricted.
async fn seed_quality_profiles(db: &Database) -> Result<()> {
    let has_default = QualityProfile::query(db.pool())
        .fetch_all()
        .await?
        .into_iter()
        .any(|profile| profile.is_default);
    if has_default {
        return Ok(());
    }

    QualityProfile::insert(
        db,
        CreateQualityProfileInput {
            name: "Any Quality".to_string(),
            media_kind: MediaKind::Video,
            allowed_resolutions: Vec::new(),
            allowed_video_codecs: Vec::new(),
            allowed_audio_formats: Vec::new(),
            allowed_hdr_types: Vec::new(),
            allowed_sources: Vec::new(),
            release_group_blacklist: Vec::new(),
            release_group_whitelist: Vec::new(),
            require_hdr: false,
            // Language/size/age stay unrestricted so the default profile keeps
            // its "allows anything" contract (design.md Q26). The three rules
            // that are sensible defaults for *every* library are: skip
            // dead torrents, allow season packs, and prefer PROPER/REPACK.
            preferred_languages: Vec::new(),
            require_language_match: false,
            min_size_mb: None,
            max_size_mb: None,
            min_seeders: 1,
            max_release_age_days: None,
            preferred_release_groups: Vec::new(),
            allow_season_packs: true,
            prefer_proper_repack: true,
            resolution_preference: Vec::new(),
            cutoff_resolution: None,
            upgrade_until_cutoff: false,
            is_default: true,
        },
    )
    .await?;

    Ok(())
}

async fn seed_torznab_categories(db: &Database) -> Result<()> {
    let existing = TorznabCategory::query(db.pool())
        .fetch_all()
        .await?
        .into_iter()
        .map(|category| category.id)
        .collect::<HashSet<_>>();

    for seed in TORZNAB_CATEGORIES {
        if existing.contains(seed.id) {
            continue;
        }

        TorznabCategory::insert(
            db,
            CreateTorznabCategoryInput {
                id: seed.id.to_string(),
                name: seed.name.to_string(),
                parent_id: seed.parent_id.map(ToString::to_string),
                description: Some(seed.description.to_string()),
            },
        )
        .await?;
    }

    Ok(())
}
