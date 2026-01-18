-- Indexer configurations for torrent tracker integration
-- This enables Jackett-like functionality directly in the Librarian backend

-- Create the update_updated_at_column function if it doesn't exist
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Main indexer configuration table
CREATE TABLE indexer_configs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    indexer_type VARCHAR(50) NOT NULL,  -- 'iptorrents', 'torrentleech', 'cardigann', etc.
    definition_id VARCHAR(100),  -- For cardigann: the YAML definition id
    name VARCHAR(255) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    priority INTEGER NOT NULL DEFAULT 50,  -- Higher priority = searched first
    
    -- Site configuration
    site_url VARCHAR(500),  -- Override default site URL
    
    -- Torznab capability flags (cached from indexer)
    supports_search BOOLEAN DEFAULT true,
    supports_tv_search BOOLEAN DEFAULT true,
    supports_movie_search BOOLEAN DEFAULT true,
    supports_music_search BOOLEAN DEFAULT false,
    supports_book_search BOOLEAN DEFAULT false,
    supports_imdb_search BOOLEAN DEFAULT false,
    supports_tvdb_search BOOLEAN DEFAULT false,
    
    -- Cached capabilities as JSON (full Torznab caps)
    capabilities JSONB,
    
    -- Health tracking
    last_error TEXT,
    error_count INTEGER NOT NULL DEFAULT 0,
    last_success_at TIMESTAMPTZ,
    last_error_at TIMESTAMPTZ,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index for user lookups
CREATE INDEX idx_indexer_configs_user_id ON indexer_configs(user_id);
CREATE INDEX idx_indexer_configs_type ON indexer_configs(indexer_type);
CREATE INDEX idx_indexer_configs_enabled ON indexer_configs(enabled) WHERE enabled = true;

-- Encrypted credentials storage
-- Credentials are encrypted at rest using AES-GCM with a server-side key
CREATE TABLE indexer_credentials (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    indexer_config_id UUID NOT NULL REFERENCES indexer_configs(id) ON DELETE CASCADE,
    credential_type VARCHAR(50) NOT NULL,  -- 'cookie', 'api_key', 'user_agent', 'username', 'password', 'passkey'
    encrypted_value TEXT NOT NULL,  -- AES-GCM encrypted value
    nonce VARCHAR(32) NOT NULL,  -- Encryption nonce (base64)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Each credential type should be unique per indexer
    UNIQUE(indexer_config_id, credential_type)
);

CREATE INDEX idx_indexer_credentials_config ON indexer_credentials(indexer_config_id);

-- Indexer-specific settings (non-sensitive configuration)
CREATE TABLE indexer_settings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    indexer_config_id UUID NOT NULL REFERENCES indexer_configs(id) ON DELETE CASCADE,
    setting_key VARCHAR(100) NOT NULL,
    setting_value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(indexer_config_id, setting_key)
);

CREATE INDEX idx_indexer_settings_config ON indexer_settings(indexer_config_id);

-- Search result cache for rate limiting and performance
CREATE TABLE indexer_search_cache (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    indexer_config_id UUID NOT NULL REFERENCES indexer_configs(id) ON DELETE CASCADE,
    query_hash VARCHAR(64) NOT NULL,  -- SHA-256 hash of normalized query
    query_type VARCHAR(20) NOT NULL,  -- 'search', 'tvsearch', 'movie', 'music', 'book'
    results JSONB NOT NULL,  -- Cached ReleaseInfo array
    result_count INTEGER NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Unique constraint to prevent duplicate cache entries
    UNIQUE(indexer_config_id, query_hash)
);

CREATE INDEX idx_indexer_search_cache_expires ON indexer_search_cache(expires_at);
CREATE INDEX idx_indexer_search_cache_lookup ON indexer_search_cache(indexer_config_id, query_hash);

-- Torznab category mappings (standard categories)
-- This table stores the standard Torznab category definitions
CREATE TABLE torznab_categories (
    id INTEGER PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    parent_id INTEGER REFERENCES torznab_categories(id),
    description TEXT
);

-- Insert standard Torznab categories
INSERT INTO torznab_categories (id, name, parent_id, description) VALUES
    -- Main categories
    (1000, 'Console', NULL, 'Console games'),
    (2000, 'Movies', NULL, 'Movies'),
    (3000, 'Audio', NULL, 'Audio/Music'),
    (4000, 'PC', NULL, 'PC software and games'),
    (5000, 'TV', NULL, 'TV shows'),
    (6000, 'XXX', NULL, 'Adult content'),
    (7000, 'Books', NULL, 'Books and comics'),
    (8000, 'Other', NULL, 'Other/Misc'),
    
    -- Movies subcategories
    (2010, 'Movies/Foreign', 2000, 'Foreign movies'),
    (2020, 'Movies/Other', 2000, 'Other movies'),
    (2030, 'Movies/SD', 2000, 'SD quality movies'),
    (2040, 'Movies/HD', 2000, 'HD quality movies'),
    (2045, 'Movies/UHD', 2000, '4K/UHD movies'),
    (2050, 'Movies/BluRay', 2000, 'BluRay movies'),
    (2060, 'Movies/3D', 2000, '3D movies'),
    (2070, 'Movies/DVD', 2000, 'DVD movies'),
    (2080, 'Movies/WEB-DL', 2000, 'WEB-DL movies'),
    
    -- TV subcategories
    (5010, 'TV/WEB-DL', 5000, 'WEB-DL TV shows'),
    (5020, 'TV/Foreign', 5000, 'Foreign TV shows'),
    (5030, 'TV/SD', 5000, 'SD TV shows'),
    (5040, 'TV/HD', 5000, 'HD TV shows'),
    (5045, 'TV/UHD', 5000, '4K/UHD TV shows'),
    (5050, 'TV/Other', 5000, 'Other TV shows'),
    (5060, 'TV/Sport', 5000, 'Sports'),
    (5070, 'TV/Anime', 5000, 'Anime'),
    (5080, 'TV/Documentary', 5000, 'Documentaries'),
    
    -- Audio subcategories
    (3010, 'Audio/MP3', 3000, 'MP3 audio'),
    (3020, 'Audio/Video', 3000, 'Music videos'),
    (3030, 'Audio/Audiobook', 3000, 'Audiobooks'),
    (3040, 'Audio/Lossless', 3000, 'Lossless audio'),
    (3050, 'Audio/Other', 3000, 'Other audio'),
    (3060, 'Audio/Foreign', 3000, 'Foreign audio'),
    
    -- Books subcategories
    (7010, 'Books/Mags', 7000, 'Magazines'),
    (7020, 'Books/EBook', 7000, 'E-Books'),
    (7030, 'Books/Comics', 7000, 'Comics'),
    (7040, 'Books/Technical', 7000, 'Technical books'),
    (7050, 'Books/Other', 7000, 'Other books'),
    (7060, 'Books/Foreign', 7000, 'Foreign books'),
    
    -- PC subcategories
    (4010, 'PC/0day', 4000, '0-day releases'),
    (4020, 'PC/ISO', 4000, 'ISO images'),
    (4030, 'PC/Mac', 4000, 'Mac software'),
    (4040, 'PC/Mobile-Other', 4000, 'Mobile software'),
    (4050, 'PC/Games', 4000, 'PC games'),
    (4060, 'PC/Mobile-iOS', 4000, 'iOS apps'),
    (4070, 'PC/Mobile-Android', 4000, 'Android apps'),
    
    -- Console subcategories
    (1010, 'Console/NDS', 1000, 'Nintendo DS'),
    (1020, 'Console/PSP', 1000, 'PlayStation Portable'),
    (1030, 'Console/Wii', 1000, 'Nintendo Wii'),
    (1040, 'Console/XBox', 1000, 'Xbox'),
    (1050, 'Console/XBox 360', 1000, 'Xbox 360'),
    (1060, 'Console/WiiWare', 1000, 'WiiWare'),
    (1070, 'Console/XBox 360 DLC', 1000, 'Xbox 360 DLC'),
    (1080, 'Console/PS3', 1000, 'PlayStation 3'),
    (1090, 'Console/Other', 1000, 'Other consoles'),
    (1110, 'Console/3DS', 1000, 'Nintendo 3DS'),
    (1120, 'Console/PS Vita', 1000, 'PlayStation Vita'),
    (1130, 'Console/WiiU', 1000, 'Wii U'),
    (1140, 'Console/XBox One', 1000, 'Xbox One'),
    (1150, 'Console/PS4', 1000, 'PlayStation 4'),
    (1180, 'Console/Switch', 1000, 'Nintendo Switch');

-- Enable RLS
ALTER TABLE indexer_configs ENABLE ROW LEVEL SECURITY;
ALTER TABLE indexer_credentials ENABLE ROW LEVEL SECURITY;
ALTER TABLE indexer_settings ENABLE ROW LEVEL SECURITY;
ALTER TABLE indexer_search_cache ENABLE ROW LEVEL SECURITY;

-- RLS policies for indexer_configs
CREATE POLICY "Users can view own indexer configs"
    ON indexer_configs FOR SELECT
    USING (auth.uid() = user_id);

CREATE POLICY "Users can create own indexer configs"
    ON indexer_configs FOR INSERT
    WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update own indexer configs"
    ON indexer_configs FOR UPDATE
    USING (auth.uid() = user_id);

CREATE POLICY "Users can delete own indexer configs"
    ON indexer_configs FOR DELETE
    USING (auth.uid() = user_id);

-- RLS policies for indexer_credentials (via parent indexer_config)
CREATE POLICY "Users can view own indexer credentials"
    ON indexer_credentials FOR SELECT
    USING (EXISTS (
        SELECT 1 FROM indexer_configs 
        WHERE indexer_configs.id = indexer_credentials.indexer_config_id 
        AND indexer_configs.user_id = auth.uid()
    ));

CREATE POLICY "Users can create own indexer credentials"
    ON indexer_credentials FOR INSERT
    WITH CHECK (EXISTS (
        SELECT 1 FROM indexer_configs 
        WHERE indexer_configs.id = indexer_credentials.indexer_config_id 
        AND indexer_configs.user_id = auth.uid()
    ));

CREATE POLICY "Users can update own indexer credentials"
    ON indexer_credentials FOR UPDATE
    USING (EXISTS (
        SELECT 1 FROM indexer_configs 
        WHERE indexer_configs.id = indexer_credentials.indexer_config_id 
        AND indexer_configs.user_id = auth.uid()
    ));

CREATE POLICY "Users can delete own indexer credentials"
    ON indexer_credentials FOR DELETE
    USING (EXISTS (
        SELECT 1 FROM indexer_configs 
        WHERE indexer_configs.id = indexer_credentials.indexer_config_id 
        AND indexer_configs.user_id = auth.uid()
    ));

-- RLS policies for indexer_settings (via parent indexer_config)
CREATE POLICY "Users can view own indexer settings"
    ON indexer_settings FOR SELECT
    USING (EXISTS (
        SELECT 1 FROM indexer_configs 
        WHERE indexer_configs.id = indexer_settings.indexer_config_id 
        AND indexer_configs.user_id = auth.uid()
    ));

CREATE POLICY "Users can create own indexer settings"
    ON indexer_settings FOR INSERT
    WITH CHECK (EXISTS (
        SELECT 1 FROM indexer_configs 
        WHERE indexer_configs.id = indexer_settings.indexer_config_id 
        AND indexer_configs.user_id = auth.uid()
    ));

CREATE POLICY "Users can update own indexer settings"
    ON indexer_settings FOR UPDATE
    USING (EXISTS (
        SELECT 1 FROM indexer_configs 
        WHERE indexer_configs.id = indexer_settings.indexer_config_id 
        AND indexer_configs.user_id = auth.uid()
    ));

CREATE POLICY "Users can delete own indexer settings"
    ON indexer_settings FOR DELETE
    USING (EXISTS (
        SELECT 1 FROM indexer_configs 
        WHERE indexer_configs.id = indexer_settings.indexer_config_id 
        AND indexer_configs.user_id = auth.uid()
    ));

-- RLS policies for indexer_search_cache (via parent indexer_config)
CREATE POLICY "Users can view own indexer cache"
    ON indexer_search_cache FOR SELECT
    USING (EXISTS (
        SELECT 1 FROM indexer_configs 
        WHERE indexer_configs.id = indexer_search_cache.indexer_config_id 
        AND indexer_configs.user_id = auth.uid()
    ));

CREATE POLICY "Users can create own indexer cache"
    ON indexer_search_cache FOR INSERT
    WITH CHECK (EXISTS (
        SELECT 1 FROM indexer_configs 
        WHERE indexer_configs.id = indexer_search_cache.indexer_config_id 
        AND indexer_configs.user_id = auth.uid()
    ));

CREATE POLICY "Users can delete own indexer cache"
    ON indexer_search_cache FOR DELETE
    USING (EXISTS (
        SELECT 1 FROM indexer_configs 
        WHERE indexer_configs.id = indexer_search_cache.indexer_config_id 
        AND indexer_configs.user_id = auth.uid()
    ));

-- Updated_at trigger for indexer_configs
CREATE TRIGGER update_indexer_configs_updated_at
    BEFORE UPDATE ON indexer_configs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Updated_at trigger for indexer_credentials
CREATE TRIGGER update_indexer_credentials_updated_at
    BEFORE UPDATE ON indexer_credentials
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Updated_at trigger for indexer_settings
CREATE TRIGGER update_indexer_settings_updated_at
    BEFORE UPDATE ON indexer_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Function to clean expired cache entries
CREATE OR REPLACE FUNCTION clean_expired_indexer_cache()
RETURNS void AS $$
BEGIN
    DELETE FROM indexer_search_cache WHERE expires_at < NOW();
END;
$$ LANGUAGE plpgsql;
