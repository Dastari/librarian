-- Ensure cast_settings.default_volume is REAL (schema_sync may have created it as INTEGER).
-- SQLite does not support ALTER COLUMN type; recreate the table.

CREATE TABLE IF NOT EXISTS cast_settings_new (
    id TEXT PRIMARY KEY,
    auto_discovery_enabled INTEGER NOT NULL DEFAULT 1,
    discovery_interval_seconds INTEGER NOT NULL DEFAULT 30,
    default_volume REAL NOT NULL DEFAULT 1.0,
    transcode_incompatible INTEGER NOT NULL DEFAULT 1,
    preferred_quality TEXT DEFAULT '1080p',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO cast_settings_new (id, auto_discovery_enabled, discovery_interval_seconds, default_volume, transcode_incompatible, preferred_quality, created_at, updated_at)
SELECT id, auto_discovery_enabled, discovery_interval_seconds, CAST(default_volume AS REAL), transcode_incompatible, preferred_quality, created_at, updated_at
FROM cast_settings;

DROP TABLE cast_settings;
ALTER TABLE cast_settings_new RENAME TO cast_settings;
