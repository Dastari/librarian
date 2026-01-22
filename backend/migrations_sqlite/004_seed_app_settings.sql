-- Seed additional app_settings that were in PostgreSQL migrations
-- but missing from SQLite initial schema

-- ============================================================================
-- LLM Parser Settings (from 030_llm_parser_settings.sql)
-- ============================================================================

INSERT OR IGNORE INTO app_settings (id, key, value, description, category) VALUES
    (lower(hex(randomblob(16))), 'llm.enabled', 'false', 'Enable LLM-based filename parsing as fallback when regex parser fails or has low confidence', 'llm'),
    (lower(hex(randomblob(16))), 'llm.ollama_url', '"http://localhost:11434"', 'URL of the Ollama API server', 'llm'),
    (lower(hex(randomblob(16))), 'llm.ollama_model', '"qwen2.5-coder:7b"', 'Ollama model to use for parsing', 'llm'),
    (lower(hex(randomblob(16))), 'llm.timeout_seconds', '30', 'Timeout in seconds for LLM API calls', 'llm'),
    (lower(hex(randomblob(16))), 'llm.temperature', '0.1', 'Temperature for LLM generation (lower = more deterministic)', 'llm'),
    (lower(hex(randomblob(16))), 'llm.prompt_template', 'null', 'Custom prompt template for LLM parsing (null = use default)', 'llm'),
    (lower(hex(randomblob(16))), 'llm.max_retries', '2', 'Maximum number of retries for failed LLM calls', 'llm'),
    (lower(hex(randomblob(16))), 'llm.use_for_ambiguous', 'true', 'Use LLM for ambiguous filenames even when regex succeeds with low confidence', 'llm');

-- ============================================================================
-- LLM Library Type Models (from 031_llm_library_type_models.sql)
-- ============================================================================

INSERT OR IGNORE INTO app_settings (id, key, value, description, category) VALUES
    (lower(hex(randomblob(16))), 'llm.model.movies', 'null', 'Ollama model for movie libraries (null = use default)', 'llm'),
    (lower(hex(randomblob(16))), 'llm.model.tv', 'null', 'Ollama model for TV show libraries (null = use default)', 'llm'),
    (lower(hex(randomblob(16))), 'llm.model.music', 'null', 'Ollama model for music libraries (null = use default)', 'llm'),
    (lower(hex(randomblob(16))), 'llm.model.audiobooks', 'null', 'Ollama model for audiobook libraries (null = use default)', 'llm'),
    (lower(hex(randomblob(16))), 'llm.prompt.movies', 'null', 'Prompt template for movie libraries (null = use default)', 'llm'),
    (lower(hex(randomblob(16))), 'llm.prompt.tv', 'null', 'Prompt template for TV show libraries (null = use default)', 'llm'),
    (lower(hex(randomblob(16))), 'llm.prompt.music', 'null', 'Prompt template for music libraries (null = use default)', 'llm'),
    (lower(hex(randomblob(16))), 'llm.prompt.audiobooks', 'null', 'Prompt template for audiobook libraries (null = use default)', 'llm');

-- ============================================================================
-- Playback Settings (from 023_watch_progress.sql)
-- ============================================================================

INSERT OR IGNORE INTO app_settings (id, key, value, description, category) VALUES
    (lower(hex(randomblob(16))), 'playback_sync_interval', '15', 'How often to sync watch progress to the database (in seconds)', 'playback');

-- ============================================================================
-- Metadata Provider Settings (commonly needed)
-- ============================================================================

INSERT OR IGNORE INTO app_settings (id, key, value, description, category) VALUES
    (lower(hex(randomblob(16))), 'metadata.tmdb_api_key', 'null', 'TMDB API key for movie/TV metadata', 'metadata'),
    (lower(hex(randomblob(16))), 'metadata.tvdb_api_key', 'null', 'TVDB API key for TV show metadata', 'metadata'),
    (lower(hex(randomblob(16))), 'metadata.auto_fetch', 'true', 'Automatically fetch metadata when adding new media', 'metadata'),
    (lower(hex(randomblob(16))), 'metadata.preferred_language', '"en"', 'Preferred language for metadata', 'metadata');

-- ============================================================================
-- Subtitle Settings (commonly needed)
-- ============================================================================

INSERT OR IGNORE INTO app_settings (id, key, value, description, category) VALUES
    (lower(hex(randomblob(16))), 'subtitles.auto_download', 'false', 'Automatically download subtitles for new media', 'subtitles'),
    (lower(hex(randomblob(16))), 'subtitles.preferred_languages', '["en"]', 'Preferred subtitle languages (JSON array)', 'subtitles'),
    (lower(hex(randomblob(16))), 'subtitles.opensubtitles_api_key', 'null', 'OpenSubtitles API key', 'subtitles');

-- ============================================================================
-- Organization Settings (commonly needed)
-- ============================================================================

INSERT OR IGNORE INTO app_settings (id, key, value, description, category) VALUES
    (lower(hex(randomblob(16))), 'organize.auto_organize', 'false', 'Automatically organize files after download completes', 'organize'),
    (lower(hex(randomblob(16))), 'organize.delete_empty_folders', 'true', 'Delete empty folders after organizing', 'organize'),
    (lower(hex(randomblob(16))), 'organize.copy_mode', '"copy"', 'File operation mode: copy, move, or hardlink', 'organize');
