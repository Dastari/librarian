-- Add auto_hunt column to libraries
-- When enabled, the system will actively search for missing episodes using Prowlarr/indexers

ALTER TABLE libraries 
ADD COLUMN IF NOT EXISTS auto_hunt BOOLEAN NOT NULL DEFAULT false;

-- Add auto_hunt_override to tv_shows
-- NULL = inherit from library, true/false = override
ALTER TABLE tv_shows 
ADD COLUMN IF NOT EXISTS auto_hunt_override BOOLEAN DEFAULT NULL;

-- Add comment for documentation
COMMENT ON COLUMN libraries.auto_hunt IS 'When enabled, actively search for missing episodes using indexers';
COMMENT ON COLUMN tv_shows.auto_hunt_override IS 'Override library auto_hunt setting (NULL = inherit)';
