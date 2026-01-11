-- Create logs table for storing backend tracing logs
CREATE TABLE IF NOT EXISTS app_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    level TEXT NOT NULL CHECK (level IN ('TRACE', 'DEBUG', 'INFO', 'WARN', 'ERROR')),
    target TEXT NOT NULL,  -- The module/source path (e.g., librarian_backend::jobs::auto_download)
    message TEXT NOT NULL,
    fields JSONB,  -- Structured fields from tracing spans
    span_name TEXT,  -- Optional span name
    span_id TEXT,  -- Optional span ID for correlation
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_app_logs_timestamp ON app_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_app_logs_level ON app_logs(level);
CREATE INDEX IF NOT EXISTS idx_app_logs_target ON app_logs(target);
CREATE INDEX IF NOT EXISTS idx_app_logs_created_at ON app_logs(created_at DESC);

-- GIN index for JSONB fields for efficient field searches
CREATE INDEX IF NOT EXISTS idx_app_logs_fields ON app_logs USING GIN (fields);

-- Full-text search index on message
CREATE INDEX IF NOT EXISTS idx_app_logs_message_search ON app_logs USING GIN (to_tsvector('english', message));

-- Composite index for common queries (level + timestamp)
CREATE INDEX IF NOT EXISTS idx_app_logs_level_timestamp ON app_logs(level, timestamp DESC);

-- Composite index for filtering by target and timestamp
CREATE INDEX IF NOT EXISTS idx_app_logs_target_timestamp ON app_logs(target, timestamp DESC);

-- Add comment describing the table
COMMENT ON TABLE app_logs IS 'Stores backend tracing logs for debugging and monitoring';
COMMENT ON COLUMN app_logs.level IS 'Log level: TRACE, DEBUG, INFO, WARN, ERROR';
COMMENT ON COLUMN app_logs.target IS 'The Rust module path that generated the log (e.g., librarian_backend::services::torrent)';
COMMENT ON COLUMN app_logs.fields IS 'Structured fields from tracing spans in JSON format';
COMMENT ON COLUMN app_logs.span_name IS 'Name of the tracing span if within one';
COMMENT ON COLUMN app_logs.span_id IS 'Unique span ID for correlating logs within the same span';
