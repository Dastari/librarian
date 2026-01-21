-- Add verification status columns for tracking metadata-based match verification
-- This allows tracking when matches have been verified using embedded metadata

-- Add verification columns to pending_file_matches
ALTER TABLE pending_file_matches
    ADD COLUMN IF NOT EXISTS verification_status VARCHAR(20),
    ADD COLUMN IF NOT EXISTS mismatch_details JSONB;

-- verification_status values:
-- NULL: Not yet verified
-- 'verified': Match confirmed by embedded metadata
-- 'corrected': Match was wrong, auto-corrected using metadata
-- 'flagged': Mismatch detected but no high-confidence alternative found

COMMENT ON COLUMN pending_file_matches.verification_status IS 
    'Result of metadata-based verification: verified, corrected, flagged, or NULL (not checked)';
COMMENT ON COLUMN pending_file_matches.mismatch_details IS 
    'JSON details about mismatches: {expected_album, found_album, expected_title, found_title, etc.}';

-- Add verification columns to media_files for library scan verification
ALTER TABLE media_files
    ADD COLUMN IF NOT EXISTS verification_status VARCHAR(20),
    ADD COLUMN IF NOT EXISTS mismatch_details JSONB,
    ADD COLUMN IF NOT EXISTS last_verified_at TIMESTAMPTZ;

COMMENT ON COLUMN media_files.verification_status IS 
    'Result of metadata-based verification during library scan';
COMMENT ON COLUMN media_files.mismatch_details IS 
    'JSON details about detected mismatches';
COMMENT ON COLUMN media_files.last_verified_at IS 
    'When this file was last verified using embedded metadata';

-- Index for finding flagged items that need review
CREATE INDEX IF NOT EXISTS idx_pending_file_matches_flagged 
    ON pending_file_matches(verification_status) 
    WHERE verification_status = 'flagged';

CREATE INDEX IF NOT EXISTS idx_media_files_flagged 
    ON media_files(verification_status) 
    WHERE verification_status = 'flagged';
