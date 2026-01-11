-- Add scanning state to libraries table
-- This tracks whether a library scan is currently in progress

ALTER TABLE libraries ADD COLUMN IF NOT EXISTS scanning BOOLEAN NOT NULL DEFAULT FALSE;

-- Add index for efficient queries on scanning state
CREATE INDEX IF NOT EXISTS idx_libraries_scanning ON libraries(scanning) WHERE scanning = TRUE;
