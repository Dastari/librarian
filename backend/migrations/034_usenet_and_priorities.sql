-- Migration: Usenet Download Support and Source Priority Rules
-- This migration adds:
-- 1. Source priority rules for per-library-type source ordering
-- 2. Usenet server configuration
-- 3. Usenet downloads tracking
-- 4. download_type column to indexer_configs

-- ============================================================================
-- Source Priority Rules
-- ============================================================================
-- Allows users to specify which sources to use for different content types
-- and in what order. Supports:
-- - Global default (library_type = NULL, library_id = NULL)
-- - Per-library-type (library_type set, library_id = NULL)
-- - Per-library (library_id set)

CREATE TABLE IF NOT EXISTS source_priority_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    
    -- Scope: global (NULL) or per-library-type or per-library
    library_type VARCHAR(50),  -- 'movies', 'tv', 'music', 'audiobooks', NULL=default
    library_id UUID REFERENCES libraries(id) ON DELETE CASCADE,
    
    -- Priority list: ordered array of source references
    -- Format: [{"source_type": "torrent_indexer", "id": "uuid"}, {"source_type": "usenet_indexer", "id": "uuid"}, ...]
    priority_order JSONB NOT NULL DEFAULT '[]',
    
    -- Behavior settings
    search_all_sources BOOLEAN NOT NULL DEFAULT false,  -- If false, stop at first match
    enabled BOOLEAN NOT NULL DEFAULT true,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraint: library_type must be valid if set
    CONSTRAINT valid_library_type CHECK (
        library_type IS NULL OR 
        library_type IN ('movies', 'tv', 'music', 'audiobooks', 'other')
    ),
    
    -- Constraint: can't set library_id without library_type matching
    CONSTRAINT library_scope_consistency CHECK (
        (library_id IS NULL) OR 
        (library_type IS NOT NULL)
    )
);

-- Unique constraint: only one rule per scope
-- Using NULLS NOT DISTINCT to treat NULL values as equal
CREATE UNIQUE INDEX IF NOT EXISTS idx_source_priority_rules_unique_scope 
    ON source_priority_rules (user_id, COALESCE(library_type, ''), COALESCE(library_id, '00000000-0000-0000-0000-000000000000'::uuid));

CREATE INDEX IF NOT EXISTS idx_source_priority_rules_user ON source_priority_rules(user_id);
CREATE INDEX IF NOT EXISTS idx_source_priority_rules_library ON source_priority_rules(library_id) WHERE library_id IS NOT NULL;

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_source_priority_rules_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_source_priority_rules_updated_at ON source_priority_rules;
CREATE TRIGGER trigger_source_priority_rules_updated_at
    BEFORE UPDATE ON source_priority_rules
    FOR EACH ROW
    EXECUTE FUNCTION update_source_priority_rules_updated_at();

-- ============================================================================
-- Usenet Servers (NNTP Providers)
-- ============================================================================
-- Configuration for Usenet news servers that provide article downloads

CREATE TABLE IF NOT EXISTS usenet_servers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    host VARCHAR(255) NOT NULL,
    port INTEGER NOT NULL DEFAULT 563,
    use_ssl BOOLEAN NOT NULL DEFAULT true,
    username VARCHAR(255),
    -- Password stored encrypted (same pattern as indexer credentials)
    encrypted_password TEXT,
    password_nonce TEXT,
    -- Connection settings
    connections INTEGER NOT NULL DEFAULT 10,
    priority INTEGER NOT NULL DEFAULT 0,  -- Server priority for multi-server setups (lower = higher priority)
    enabled BOOLEAN NOT NULL DEFAULT true,
    -- Server capabilities
    retention_days INTEGER,  -- How many days of articles the server keeps
    -- Health tracking
    last_success_at TIMESTAMPTZ,
    last_error TEXT,
    error_count INTEGER NOT NULL DEFAULT 0,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_usenet_servers_user ON usenet_servers(user_id);
CREATE INDEX IF NOT EXISTS idx_usenet_servers_enabled ON usenet_servers(user_id, enabled) WHERE enabled = true;

-- Trigger for updated_at
DROP TRIGGER IF EXISTS trigger_usenet_servers_updated_at ON usenet_servers;
CREATE TRIGGER trigger_usenet_servers_updated_at
    BEFORE UPDATE ON usenet_servers
    FOR EACH ROW
    EXECUTE FUNCTION update_source_priority_rules_updated_at();

-- ============================================================================
-- Usenet Downloads
-- ============================================================================
-- Tracks NZB downloads (parallel to torrents table)

CREATE TABLE IF NOT EXISTS usenet_downloads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    
    -- NZB identification
    nzb_name VARCHAR(500) NOT NULL,
    nzb_hash VARCHAR(64),  -- Hash of the NZB file for deduplication
    
    -- Download state
    state VARCHAR(50) NOT NULL DEFAULT 'queued',  -- queued, downloading, paused, completed, failed, removed
    progress DECIMAL(5,2) DEFAULT 0,  -- 0.00 to 100.00
    
    -- Size tracking
    size_bytes BIGINT,
    downloaded_bytes BIGINT DEFAULT 0,
    
    -- Speed and ETA
    download_speed BIGINT DEFAULT 0,  -- bytes per second
    eta_seconds INTEGER,
    
    -- Error handling
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    
    -- File location
    download_path TEXT,
    
    -- Library linking (same pattern as torrents)
    library_id UUID REFERENCES libraries(id) ON DELETE SET NULL,
    episode_id UUID REFERENCES episodes(id) ON DELETE SET NULL,
    movie_id UUID REFERENCES movies(id) ON DELETE SET NULL,
    album_id UUID REFERENCES albums(id) ON DELETE SET NULL,
    audiobook_id UUID REFERENCES audiobooks(id) ON DELETE SET NULL,
    
    -- Source tracking
    indexer_id UUID REFERENCES indexer_configs(id) ON DELETE SET NULL,
    
    -- Post-processing status (same values as torrents)
    post_process_status VARCHAR(50),  -- pending, processing, matched, completed, unmatched, error
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    
    -- Constraint: state must be valid
    CONSTRAINT valid_usenet_state CHECK (
        state IN ('queued', 'downloading', 'paused', 'completed', 'failed', 'removed')
    )
);

CREATE INDEX IF NOT EXISTS idx_usenet_downloads_user ON usenet_downloads(user_id);
CREATE INDEX IF NOT EXISTS idx_usenet_downloads_state ON usenet_downloads(state);
CREATE INDEX IF NOT EXISTS idx_usenet_downloads_hash ON usenet_downloads(nzb_hash) WHERE nzb_hash IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_usenet_downloads_library ON usenet_downloads(library_id) WHERE library_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_usenet_downloads_post_process ON usenet_downloads(post_process_status) 
    WHERE post_process_status IS NOT NULL;

-- Trigger for updated_at
DROP TRIGGER IF EXISTS trigger_usenet_downloads_updated_at ON usenet_downloads;
CREATE TRIGGER trigger_usenet_downloads_updated_at
    BEFORE UPDATE ON usenet_downloads
    FOR EACH ROW
    EXECUTE FUNCTION update_source_priority_rules_updated_at();

-- ============================================================================
-- Usenet Download File Matches
-- ============================================================================
-- File-level matching for usenet downloads (parallel to torrent_file_matches)

CREATE TABLE IF NOT EXISTS usenet_file_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    usenet_download_id UUID NOT NULL REFERENCES usenet_downloads(id) ON DELETE CASCADE,
    
    -- File identification
    file_path TEXT NOT NULL,
    file_size BIGINT,
    
    -- Match targets (same pattern as torrent_file_matches)
    episode_id UUID REFERENCES episodes(id) ON DELETE SET NULL,
    movie_id UUID REFERENCES movies(id) ON DELETE SET NULL,
    album_id UUID REFERENCES albums(id) ON DELETE SET NULL,
    track_id UUID REFERENCES tracks(id) ON DELETE SET NULL,
    audiobook_id UUID REFERENCES audiobooks(id) ON DELETE SET NULL,
    
    -- Processing status
    processed BOOLEAN NOT NULL DEFAULT false,
    media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL,
    
    -- Match metadata
    match_confidence DECIMAL(5,2),  -- 0.00 to 100.00
    match_reason TEXT,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_usenet_file_matches_download ON usenet_file_matches(usenet_download_id);
CREATE INDEX IF NOT EXISTS idx_usenet_file_matches_unprocessed ON usenet_file_matches(usenet_download_id, processed) 
    WHERE processed = false;

-- ============================================================================
-- Add download_type to indexer_configs
-- ============================================================================
-- Distinguishes torrent indexers from usenet (Newznab) indexers

ALTER TABLE indexer_configs ADD COLUMN IF NOT EXISTS 
    download_type VARCHAR(20) DEFAULT 'torrent';

-- Add constraint if not exists (PostgreSQL doesn't have IF NOT EXISTS for constraints)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint 
        WHERE conname = 'valid_download_type' 
        AND conrelid = 'indexer_configs'::regclass
    ) THEN
        ALTER TABLE indexer_configs ADD CONSTRAINT valid_download_type 
            CHECK (download_type IN ('torrent', 'usenet'));
    END IF;
END $$;

-- Update existing indexers to be torrent type
UPDATE indexer_configs SET download_type = 'torrent' WHERE download_type IS NULL;

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON TABLE source_priority_rules IS 'User-defined rules for source priority ordering during hunting';
COMMENT ON COLUMN source_priority_rules.priority_order IS 'JSON array of {source_type, id} objects in priority order';
COMMENT ON COLUMN source_priority_rules.search_all_sources IS 'If false, stop searching after first source returns results';

COMMENT ON TABLE usenet_servers IS 'Usenet NNTP server configurations for downloading';
COMMENT ON COLUMN usenet_servers.priority IS 'Server priority (lower = higher priority) for multi-server setups';
COMMENT ON COLUMN usenet_servers.retention_days IS 'Number of days of article retention on this server';

COMMENT ON TABLE usenet_downloads IS 'Tracks NZB downloads, parallel to torrents table';
COMMENT ON COLUMN usenet_downloads.nzb_hash IS 'SHA-256 hash of NZB content for deduplication';
COMMENT ON COLUMN usenet_downloads.post_process_status IS 'Same values as torrents: pending, processing, matched, completed, unmatched, error';

COMMENT ON COLUMN indexer_configs.download_type IS 'Type of downloads this indexer provides: torrent or usenet';
