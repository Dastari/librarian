//! Cardigann YAML-based indexer engine
//!
//! The Cardigann engine allows defining indexers using YAML configuration files,
//! similar to Jackett's approach. This enables support for hundreds of indexers
//! without writing native Rust code for each.
//!
//! # YAML Definition Format
//!
//! ```yaml
//! id: example-tracker
//! name: Example Tracker
//! description: "An example private tracker"
//! language: en-US
//! type: private
//! encoding: UTF-8
//! links:
//!   - https://example-tracker.com/
//!
//! caps:
//!   categorymappings:
//!     - {id: 1, cat: Movies/HD, desc: "HD Movies"}
//!     - {id: 2, cat: TV/HD, desc: "HD TV Shows"}
//!   modes:
//!     search: [q]
//!     tv-search: [q, season, ep]
//!     movie-search: [q, imdbid]
//!
//! settings:
//!   - name: username
//!     type: text
//!     label: Username
//!   - name: password
//!     type: password
//!     label: Password
//!
//! login:
//!   path: login.php
//!   method: form
//!   inputs:
//!     username: "{{ .Config.username }}"
//!     password: "{{ .Config.password }}"
//!
//! search:
//!   path: browse.php
//!   inputs:
//!     search: "{{ .Keywords }}"
//!   rows:
//!     selector: table.torrents > tbody > tr
//!   fields:
//!     title:
//!       selector: a.title
//!     download:
//!       selector: a[href^="download.php"]
//!       attribute: href
//! ```
//!
//! # Status
//!
//! This is a placeholder for future implementation. The full Cardigann engine
//! requires:
//! - YAML parsing with serde_yaml
//! - Template rendering (similar to Go templates)
//! - CSS selector execution
//! - Login flow handling
//! - Various filter functions

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use async_graphql::async_trait::async_trait;
use serde::Deserialize;

use crate::indexer::{
    Indexer, IndexerType, ReleaseInfo, TorznabCapabilities, TorznabQuery, TrackerType,
};

/// Cardigann YAML indexer definition
#[derive(Debug, Clone, Deserialize)]
pub struct IndexerDefinition {
    pub id: String,
    #[serde(default)]
    pub replaces: Vec<String>,
    pub name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    #[serde(rename = "type")]
    pub tracker_type: Option<String>,
    pub encoding: Option<String>,
    #[serde(rename = "requestDelay")]
    pub request_delay: Option<f64>,
    pub links: Vec<String>,
    #[serde(default)]
    pub legacylinks: Vec<String>,
    pub caps: Option<CapabilitiesBlock>,
    pub settings: Option<Vec<SettingsField>>,
    pub login: Option<LoginBlock>,
    pub search: Option<SearchBlock>,
    pub download: Option<DownloadBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CapabilitiesBlock {
    pub categories: Option<HashMap<String, String>>,
    pub categorymappings: Option<Vec<CategoryMappingDef>>,
    pub modes: Option<HashMap<String, Vec<String>>>,
    pub allowrawsearch: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CategoryMappingDef {
    pub id: String,
    pub cat: String,
    pub desc: Option<String>,
    #[serde(default)]
    pub default: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SettingsField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: Option<String>,
    pub label: Option<String>,
    pub default: Option<String>,
    pub options: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginBlock {
    pub path: Option<String>,
    pub submitpath: Option<String>,
    pub method: Option<String>,
    pub form: Option<String>,
    pub inputs: Option<HashMap<String, String>>,
    pub test: Option<TestBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TestBlock {
    pub path: Option<String>,
    pub selector: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchBlock {
    pub path: Option<String>,
    pub paths: Option<Vec<SearchPath>>,
    pub inputs: Option<HashMap<String, String>>,
    pub rows: Option<RowsBlock>,
    pub fields: Option<HashMap<String, FieldSelector>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchPath {
    pub path: String,
    pub categories: Option<Vec<String>>,
    pub response: Option<ResponseBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseBlock {
    #[serde(rename = "type")]
    pub response_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RowsBlock {
    pub selector: String,
    pub after: Option<i32>,
    pub count: Option<CountBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CountBlock {
    pub selector: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldSelector {
    pub selector: Option<String>,
    pub text: Option<String>,
    pub attribute: Option<String>,
    pub optional: Option<bool>,
    pub default: Option<String>,
    pub filters: Option<Vec<FilterDef>>,
    #[serde(rename = "case")]
    pub case_map: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FilterDef {
    pub name: String,
    pub args: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadBlock {
    pub selectors: Option<Vec<SelectorField>>,
    pub method: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SelectorField {
    pub selector: String,
    pub attribute: Option<String>,
    pub filters: Option<Vec<FilterDef>>,
}

/// Cardigann indexer instance
pub struct CardigannIndexer {
    id: String,
    definition: IndexerDefinition,
    config: HashMap<String, String>,
    credentials: HashMap<String, String>,
    client: reqwest::Client,
    capabilities: TorznabCapabilities,
    site_link: String,
}

impl CardigannIndexer {
    /// Create a new Cardigann indexer from a YAML definition
    pub fn new(
        instance_id: String,
        definition: IndexerDefinition,
        credentials: HashMap<String, String>,
        config: HashMap<String, String>,
    ) -> Result<Self> {
        let site_link = definition
            .links
            .first()
            .ok_or_else(|| anyhow!("Definition has no links"))?
            .clone();

        let capabilities = Self::build_capabilities(&definition)?;

        let client = reqwest::Client::builder()
            .cookie_store(true)
            .gzip(true)
            .build()?;

        Ok(Self {
            id: instance_id,
            definition,
            config,
            credentials,
            client,
            capabilities,
            site_link,
        })
    }

    /// Load a definition from YAML content
    pub fn load_definition(yaml_content: &str) -> Result<IndexerDefinition> {
        let definition: IndexerDefinition = serde_yaml::from_str(yaml_content)?;
        Ok(definition)
    }

    /// Build capabilities from the definition
    fn build_capabilities(definition: &IndexerDefinition) -> Result<TorznabCapabilities> {
        let mut caps = TorznabCapabilities::new();

        if let Some(ref caps_block) = definition.caps {
            // Parse category mappings
            if let Some(ref mappings) = caps_block.categorymappings {
                for mapping in mappings {
                    // Parse Torznab category from string (e.g., "Movies/HD" -> 2040)
                    let torznab_cat = Self::parse_category_string(&mapping.cat)?;
                    caps.add_category(
                        mapping.id.parse().unwrap_or(0),
                        torznab_cat,
                        mapping.desc.as_deref().unwrap_or(&mapping.cat),
                    );
                }
            }

            // Parse search modes
            if let Some(ref modes) = caps_block.modes {
                if modes.contains_key("tv-search") {
                    caps.tv_search_params = Self::parse_tv_params(modes.get("tv-search"));
                }
                if modes.contains_key("movie-search") {
                    caps.movie_search_params = Self::parse_movie_params(modes.get("movie-search"));
                }
            }
        }

        Ok(caps)
    }

    /// Parse a category string like "Movies/HD" to a Torznab category ID
    fn parse_category_string(cat_str: &str) -> Result<i32> {
        use crate::indexer::categories::cats;

        let cat = match cat_str.to_lowercase().as_str() {
            "movies" => cats::MOVIES,
            "movies/hd" => cats::MOVIES_HD,
            "movies/sd" => cats::MOVIES_SD,
            "movies/uhd" | "movies/4k" => cats::MOVIES_UHD,
            "movies/bluray" => cats::MOVIES_BLURAY,
            "movies/dvd" => cats::MOVIES_DVD,
            "movies/web-dl" => cats::MOVIES_WEBDL,
            "movies/3d" => cats::MOVIES_3D,
            "movies/foreign" => cats::MOVIES_FOREIGN,
            "movies/other" => cats::MOVIES_OTHER,

            "tv" => cats::TV,
            "tv/hd" => cats::TV_HD,
            "tv/sd" => cats::TV_SD,
            "tv/uhd" | "tv/4k" => cats::TV_UHD,
            "tv/web-dl" => cats::TV_WEBDL,
            "tv/foreign" => cats::TV_FOREIGN,
            "tv/anime" => cats::TV_ANIME,
            "tv/documentary" | "tv/documentaries" => cats::TV_DOCUMENTARY,
            "tv/sport" | "sports" => cats::TV_SPORT,
            "tv/other" => cats::TV_OTHER,

            "audio" | "music" => cats::AUDIO,
            "audio/mp3" => cats::AUDIO_MP3,
            "audio/lossless" | "audio/flac" => cats::AUDIO_LOSSLESS,
            "audio/audiobook" | "audiobook" | "audiobooks" => cats::AUDIO_AUDIOBOOK,
            "audio/video" => cats::AUDIO_VIDEO,

            "books" => cats::BOOKS,
            "books/ebook" | "ebooks" => cats::BOOKS_EBOOK,
            "books/comics" | "comics" => cats::BOOKS_COMICS,
            "books/mags" | "magazines" => cats::BOOKS_MAGS,

            "pc" | "apps" | "applications" => cats::PC,
            "pc/games" => cats::PC_GAMES,
            "pc/0day" => cats::PC_0DAY,
            "pc/mac" => cats::PC_MAC,

            "console" | "games" => cats::CONSOLE,

            _ => 8000, // Other
        };

        Ok(cat)
    }

    fn parse_tv_params(params: Option<&Vec<String>>) -> Vec<crate::indexer::TvSearchParam> {
        use crate::indexer::TvSearchParam;

        let mut result = vec![TvSearchParam::Q];

        if let Some(params) = params {
            for p in params {
                match p.to_lowercase().as_str() {
                    "season" => result.push(TvSearchParam::Season),
                    "ep" => result.push(TvSearchParam::Ep),
                    "imdbid" => result.push(TvSearchParam::ImdbId),
                    "tvdbid" => result.push(TvSearchParam::TvdbId),
                    "tmdbid" => result.push(TvSearchParam::TmdbId),
                    _ => {}
                }
            }
        }

        result
    }

    fn parse_movie_params(params: Option<&Vec<String>>) -> Vec<crate::indexer::MovieSearchParam> {
        use crate::indexer::MovieSearchParam;

        let mut result = vec![MovieSearchParam::Q];

        if let Some(params) = params {
            for p in params {
                match p.to_lowercase().as_str() {
                    "imdbid" => result.push(MovieSearchParam::ImdbId),
                    "tmdbid" => result.push(MovieSearchParam::TmdbId),
                    _ => {}
                }
            }
        }

        result
    }
}

#[async_trait]
impl Indexer for CardigannIndexer {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.definition.name
    }

    fn description(&self) -> &str {
        self.definition.description.as_deref().unwrap_or("")
    }

    fn indexer_type(&self) -> IndexerType {
        IndexerType::Cardigann
    }

    fn site_link(&self) -> &str {
        &self.site_link
    }

    fn tracker_type(&self) -> TrackerType {
        match self.definition.tracker_type.as_deref() {
            Some("public") => TrackerType::Public,
            Some("semi-private") => TrackerType::SemiPrivate,
            _ => TrackerType::Private,
        }
    }

    fn language(&self) -> &str {
        self.definition.language.as_deref().unwrap_or("en-US")
    }

    fn capabilities(&self) -> &TorznabCapabilities {
        &self.capabilities
    }

    fn is_configured(&self) -> bool {
        // Check if required credentials are present
        if let Some(ref settings) = self.definition.settings {
            for setting in settings {
                if setting.field_type.as_deref() == Some("password")
                    || setting.field_type.as_deref() == Some("text")
                {
                    if !self.credentials.contains_key(&setting.name)
                        && !self.config.contains_key(&setting.name)
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn supports_pagination(&self) -> bool {
        // TODO: Check definition for pagination support
        false
    }

    async fn test_connection(&self) -> Result<bool> {
        // TODO: Implement login and test using definition
        Err(anyhow!("Cardigann engine not yet implemented"))
    }

    async fn search(&self, _query: &TorznabQuery) -> Result<Vec<ReleaseInfo>> {
        // TODO: Implement search using definition
        Err(anyhow!("Cardigann engine not yet implemented"))
    }

    async fn download(&self, _link: &str) -> Result<Vec<u8>> {
        // TODO: Implement download using definition
        Err(anyhow!("Cardigann engine not yet implemented"))
    }
}
