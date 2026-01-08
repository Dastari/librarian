-- ============================================================================
-- Disable RLS for tables accessed by backend
-- ============================================================================
-- The backend uses direct SQLx connections, not Supabase's PostgREST.
-- Supabase's auth.uid() function only works through their API layer,
-- returning NULL for direct PostgreSQL connections.
-- 
-- Since the backend manually filters by user_id in all queries, RLS is
-- redundant and actually blocks operations.
-- ============================================================================

-- Disable RLS on all tables that the backend accesses directly
ALTER TABLE IF EXISTS libraries DISABLE ROW LEVEL SECURITY;
ALTER TABLE IF EXISTS media_items DISABLE ROW LEVEL SECURITY;
ALTER TABLE IF EXISTS media_files DISABLE ROW LEVEL SECURITY;
ALTER TABLE IF EXISTS artwork DISABLE ROW LEVEL SECURITY;
ALTER TABLE IF EXISTS torrents DISABLE ROW LEVEL SECURITY;
ALTER TABLE IF EXISTS app_settings DISABLE ROW LEVEL SECURITY;
ALTER TABLE IF EXISTS tv_shows DISABLE ROW LEVEL SECURITY;
ALTER TABLE IF EXISTS episodes DISABLE ROW LEVEL SECURITY;
ALTER TABLE IF EXISTS quality_profiles DISABLE ROW LEVEL SECURITY;
ALTER TABLE IF EXISTS rss_feeds DISABLE ROW LEVEL SECURITY;
ALTER TABLE IF EXISTS jobs DISABLE ROW LEVEL SECURITY;
ALTER TABLE IF EXISTS unmatched_files DISABLE ROW LEVEL SECURITY;

-- Note: Security is enforced at the application layer via JWT auth
-- and explicit user_id filtering in all database queries.
