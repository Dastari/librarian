-- User notifications table for alerts, action requests, and system messages
-- Supports real-time updates via GraphQL subscriptions

CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    
    -- Notification content
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    notification_type VARCHAR(50) NOT NULL,  -- 'info', 'warning', 'error', 'action_required'
    category VARCHAR(50) NOT NULL,           -- 'matching', 'processing', 'quality', 'storage', 'extraction'
    
    -- Related entities (optional, for linking to relevant items)
    library_id UUID REFERENCES libraries(id) ON DELETE SET NULL,
    torrent_id UUID REFERENCES torrents(id) ON DELETE SET NULL,
    media_file_id UUID REFERENCES media_files(id) ON DELETE SET NULL,
    pending_match_id UUID REFERENCES pending_file_matches(id) ON DELETE SET NULL,
    
    -- Action handling
    action_type VARCHAR(50),                 -- 'confirm_upgrade', 'manual_match', 'retry', 'dismiss'
    action_data JSONB,                       -- Context for action (e.g., { "new_file_id": "...", "existing_file_id": "..." })
    
    -- Status
    read_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    resolution VARCHAR(50),                  -- 'accepted', 'rejected', 'dismissed', 'auto_resolved'
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for unread notifications (most common query)
CREATE INDEX idx_notifications_user_unread ON notifications(user_id) WHERE read_at IS NULL;

-- Index for unresolved action-required notifications
CREATE INDEX idx_notifications_user_unresolved ON notifications(user_id) WHERE resolved_at IS NULL;

-- Index for listing notifications by creation time
CREATE INDEX idx_notifications_created ON notifications(user_id, created_at DESC);

-- Index for category filtering
CREATE INDEX idx_notifications_category ON notifications(user_id, category);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_notifications_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_notifications_updated_at ON notifications;
CREATE TRIGGER trigger_notifications_updated_at
    BEFORE UPDATE ON notifications
    FOR EACH ROW
    EXECUTE FUNCTION update_notifications_updated_at();

-- Add comments
COMMENT ON TABLE notifications IS 'User notifications for alerts, action requests, and system messages';
COMMENT ON COLUMN notifications.notification_type IS 'info, warning, error, or action_required';
COMMENT ON COLUMN notifications.category IS 'matching, processing, quality, storage, or extraction';
COMMENT ON COLUMN notifications.action_type IS 'confirm_upgrade, manual_match, retry, or dismiss';
COMMENT ON COLUMN notifications.action_data IS 'JSON context for executing the action';
COMMENT ON COLUMN notifications.resolution IS 'accepted, rejected, dismissed, or auto_resolved';
