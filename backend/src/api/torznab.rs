//! Torznab REST API endpoint
//!
//! Provides Torznab-compatible API for external applications like Sonarr and Radarr.
//!
//! # Endpoints
//!
//! - `GET /api/torznab/{indexer_id}?apikey=...&t=caps` - Get indexer capabilities
//! - `GET /api/torznab/{indexer_id}?apikey=...&t=search&q=...` - General search
//! - `GET /api/torznab/{indexer_id}?apikey=...&t=tvsearch&q=...` - TV search
//! - `GET /api/torznab/{indexer_id}?apikey=...&t=movie&q=...` - Movie search

use std::collections::HashMap;

use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};
use serde::Deserialize;
use std::io::Cursor;
use uuid::Uuid;

use crate::indexer::{
    Indexer, QueryType, ReleaseInfo, TorznabCapabilities, TorznabQuery,
    definitions::iptorrents::IPTorrentsIndexer, encryption::CredentialEncryption,
};

/// Torznab query parameters
#[derive(Debug, Deserialize, Default)]
pub struct TorznabParams {
    pub apikey: Option<String>,
    pub t: Option<String>,
    pub q: Option<String>,
    pub cat: Option<String>,
    pub limit: Option<String>,
    pub offset: Option<String>,
    pub season: Option<String>,
    pub ep: Option<String>,
    pub imdbid: Option<String>,
    pub tvdbid: Option<String>,
    pub tmdbid: Option<String>,
}

use crate::AppState;

/// Create the Torznab router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/torznab/{indexer_id}", get(torznab_handler))
        .route("/torznab/{indexer_id}/api", get(torznab_handler))
}

/// Main Torznab endpoint handler
async fn torznab_handler(
    State(state): State<AppState>,
    Path(indexer_id): Path<String>,
    Query(params): Query<TorznabParams>,
) -> Response {
    // Parse indexer ID
    let config_id = match Uuid::parse_str(&indexer_id) {
        Ok(id) => id,
        Err(_) => {
            return error_response(201, "Invalid indexer ID");
        }
    };

    // Get the indexer config
    let config = match state.db.indexers().get(config_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return error_response(201, "Indexer not found"),
        Err(e) => return error_response(900, &format!("Database error: {}", e)),
    };

    // Get credentials and decrypt them
    let credentials = match state.db.indexers().get_credentials(config_id).await {
        Ok(c) => c,
        Err(e) => return error_response(900, &format!("Failed to get credentials: {}", e)),
    };

    let settings = state
        .db
        .indexers()
        .get_settings(config_id)
        .await
        .unwrap_or_default();

    // Get encryption key from database
    let encryption_key = match state
        .db
        .settings()
        .get_or_create_indexer_encryption_key()
        .await
    {
        Ok(key) => key,
        Err(e) => return error_response(900, &format!("Failed to get encryption key: {}", e)),
    };

    let encryption = match CredentialEncryption::from_base64_key(&encryption_key) {
        Ok(e) => e,
        Err(e) => return error_response(900, &format!("Encryption error: {}", e)),
    };

    let mut decrypted_creds: HashMap<String, String> = HashMap::new();
    for cred in credentials {
        if let Ok(value) = encryption.decrypt(&cred.encrypted_value, &cred.nonce) {
            decrypted_creds.insert(cred.credential_type, value);
        }
    }

    let settings_map: HashMap<String, String> = settings
        .into_iter()
        .map(|s| (s.setting_key, s.setting_value))
        .collect();

    // Handle request based on type
    let query_type = params.t.as_deref().unwrap_or("search");

    match query_type {
        "caps" | "capabilities" => {
            // Return capabilities
            match config.indexer_type.as_str() {
                "iptorrents" => {
                    let cookie = decrypted_creds.get("cookie").cloned().unwrap_or_default();
                    let user_agent = decrypted_creds
                        .get("user_agent")
                        .cloned()
                        .unwrap_or_default();

                    match IPTorrentsIndexer::new(
                        config_id.to_string(),
                        config.name.clone(),
                        config.site_url.clone(),
                        &cookie,
                        &user_agent,
                        settings_map,
                    ) {
                        Ok(indexer) => {
                            let caps = indexer.capabilities();
                            caps_response(&config.name, caps)
                        }
                        Err(e) => error_response(900, &format!("Failed to create indexer: {}", e)),
                    }
                }
                _ => error_response(
                    201,
                    &format!("Unsupported indexer type: {}", config.indexer_type),
                ),
            }
        }
        "search" | "tvsearch" | "movie" | "music" | "book" => {
            // Perform search
            let query = TorznabQuery {
                query_type: match query_type {
                    "tvsearch" => QueryType::TvSearch,
                    "movie" => QueryType::MovieSearch,
                    "music" => QueryType::MusicSearch,
                    "book" => QueryType::BookSearch,
                    _ => QueryType::Search,
                },
                search_term: params.q.clone(),
                categories: params
                    .cat
                    .as_ref()
                    .map(|c| c.split(',').filter_map(|s| s.parse().ok()).collect())
                    .unwrap_or_default(),
                season: params.season.as_ref().and_then(|s| s.parse().ok()),
                episode: params.ep.clone(),
                imdb_id: params.imdbid.clone(),
                tvdb_id: params.tvdbid.as_ref().and_then(|s| s.parse().ok()),
                tmdb_id: params.tmdbid.as_ref().and_then(|s| s.parse().ok()),
                limit: params.limit.as_ref().and_then(|s| s.parse().ok()),
                offset: params.offset.as_ref().and_then(|s| s.parse().ok()),
                cache: true,
                ..Default::default()
            };

            match config.indexer_type.as_str() {
                "iptorrents" => {
                    let cookie = decrypted_creds.get("cookie").cloned().unwrap_or_default();
                    let user_agent = decrypted_creds
                        .get("user_agent")
                        .cloned()
                        .unwrap_or_default();

                    match IPTorrentsIndexer::new(
                        config_id.to_string(),
                        config.name.clone(),
                        config.site_url.clone(),
                        &cookie,
                        &user_agent,
                        settings_map,
                    ) {
                        Ok(indexer) => match indexer.search(&query).await {
                            Ok(releases) => {
                                let _ = state.db.indexers().record_success(config_id).await;
                                search_response(
                                    &config.name,
                                    "IPTorrents",
                                    &indexer.site_link(),
                                    releases,
                                )
                            }
                            Err(e) => {
                                let _ = state
                                    .db
                                    .indexers()
                                    .record_error(config_id, &e.to_string())
                                    .await;
                                error_response(900, &e.to_string())
                            }
                        },
                        Err(e) => error_response(900, &format!("Failed to create indexer: {}", e)),
                    }
                }
                _ => error_response(
                    201,
                    &format!("Unsupported indexer type: {}", config.indexer_type),
                ),
            }
        }
        _ => error_response(201, &format!("Unknown query type: {}", query_type)),
    }
}

/// Generate an error response in Torznab XML format
fn error_response(code: i32, description: &str) -> Response {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .ok();

    let mut error = BytesStart::new("error");
    error.push_attribute(("code", code.to_string().as_str()));
    error.push_attribute(("description", description));
    writer.write_event(Event::Empty(error)).ok();

    let xml = String::from_utf8(writer.into_inner().into_inner()).unwrap_or_default();

    (
        StatusCode::OK, // Torznab returns 200 even for errors
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

/// Generate capabilities response
fn caps_response(title: &str, caps: &TorznabCapabilities) -> Response {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .ok();

    // <caps>
    writer
        .write_event(Event::Start(BytesStart::new("caps")))
        .ok();

    // <server>
    let mut server = BytesStart::new("server");
    server.push_attribute(("title", title));
    writer.write_event(Event::Empty(server)).ok();

    // <limits>
    let mut limits = BytesStart::new("limits");
    limits.push_attribute((
        "default",
        caps.limits_default.unwrap_or(100).to_string().as_str(),
    ));
    limits.push_attribute(("max", caps.limits_max.unwrap_or(100).to_string().as_str()));
    writer.write_event(Event::Empty(limits)).ok();

    // <searching>
    writer
        .write_event(Event::Start(BytesStart::new("searching")))
        .ok();

    // <search>
    let mut search = BytesStart::new("search");
    search.push_attribute((
        "available",
        if caps.search_available { "yes" } else { "no" },
    ));
    search.push_attribute(("supportedParams", "q"));
    writer.write_event(Event::Empty(search)).ok();

    // <tv-search>
    let mut tv = BytesStart::new("tv-search");
    tv.push_attribute((
        "available",
        if caps.tv_search_available() {
            "yes"
        } else {
            "no"
        },
    ));
    tv.push_attribute(("supportedParams", "q,season,ep,imdbid"));
    writer.write_event(Event::Empty(tv)).ok();

    // <movie-search>
    let mut movie = BytesStart::new("movie-search");
    movie.push_attribute((
        "available",
        if caps.movie_search_available() {
            "yes"
        } else {
            "no"
        },
    ));
    movie.push_attribute(("supportedParams", "q,imdbid"));
    writer.write_event(Event::Empty(movie)).ok();

    writer
        .write_event(Event::End(BytesEnd::new("searching")))
        .ok();

    // <categories>
    writer
        .write_event(Event::Start(BytesStart::new("categories")))
        .ok();
    for cat in &caps.categories {
        let mut elem = BytesStart::new("category");
        elem.push_attribute(("id", cat.torznab_cat.to_string().as_str()));
        if let Some(ref desc) = cat.description {
            elem.push_attribute(("name", desc.as_str()));
        }
        writer.write_event(Event::Empty(elem)).ok();
    }
    writer
        .write_event(Event::End(BytesEnd::new("categories")))
        .ok();

    writer.write_event(Event::End(BytesEnd::new("caps"))).ok();

    let xml = String::from_utf8(writer.into_inner().into_inner()).unwrap_or_default();

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

/// Generate search results response
fn search_response(
    title: &str,
    description: &str,
    link: &str,
    releases: Vec<ReleaseInfo>,
) -> Response {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .ok();

    // <rss>
    let mut rss = BytesStart::new("rss");
    rss.push_attribute(("version", "2.0"));
    rss.push_attribute(("xmlns:atom", "http://www.w3.org/2005/Atom"));
    rss.push_attribute(("xmlns:torznab", "http://torznab.com/schemas/2015/feed"));
    writer.write_event(Event::Start(rss)).ok();

    // <channel>
    writer
        .write_event(Event::Start(BytesStart::new("channel")))
        .ok();

    // <title>
    writer
        .write_event(Event::Start(BytesStart::new("title")))
        .ok();
    writer.write_event(Event::Text(BytesText::new(title))).ok();
    writer.write_event(Event::End(BytesEnd::new("title"))).ok();

    // <description>
    writer
        .write_event(Event::Start(BytesStart::new("description")))
        .ok();
    writer
        .write_event(Event::Text(BytesText::new(description)))
        .ok();
    writer
        .write_event(Event::End(BytesEnd::new("description")))
        .ok();

    // <link>
    writer
        .write_event(Event::Start(BytesStart::new("link")))
        .ok();
    writer.write_event(Event::Text(BytesText::new(link))).ok();
    writer.write_event(Event::End(BytesEnd::new("link"))).ok();

    // Items
    for release in releases {
        write_item(&mut writer, &release);
    }

    writer
        .write_event(Event::End(BytesEnd::new("channel")))
        .ok();
    writer.write_event(Event::End(BytesEnd::new("rss"))).ok();

    let xml = String::from_utf8(writer.into_inner().into_inner()).unwrap_or_default();

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

fn write_item(writer: &mut Writer<Cursor<Vec<u8>>>, release: &ReleaseInfo) {
    writer
        .write_event(Event::Start(BytesStart::new("item")))
        .ok();

    // <title>
    writer
        .write_event(Event::Start(BytesStart::new("title")))
        .ok();
    writer
        .write_event(Event::Text(BytesText::new(&release.title)))
        .ok();
    writer.write_event(Event::End(BytesEnd::new("title"))).ok();

    // <guid>
    writer
        .write_event(Event::Start(BytesStart::new("guid")))
        .ok();
    writer
        .write_event(Event::Text(BytesText::new(&release.guid)))
        .ok();
    writer.write_event(Event::End(BytesEnd::new("guid"))).ok();

    // <pubDate>
    let pub_date = release
        .publish_date
        .format("%a, %d %b %Y %H:%M:%S %z")
        .to_string();
    writer
        .write_event(Event::Start(BytesStart::new("pubDate")))
        .ok();
    writer
        .write_event(Event::Text(BytesText::new(&pub_date)))
        .ok();
    writer
        .write_event(Event::End(BytesEnd::new("pubDate")))
        .ok();

    // <link>
    let link = release
        .link
        .as_ref()
        .or(release.magnet_uri.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("");
    writer
        .write_event(Event::Start(BytesStart::new("link")))
        .ok();
    writer.write_event(Event::Text(BytesText::new(link))).ok();
    writer.write_event(Event::End(BytesEnd::new("link"))).ok();

    // <size>
    if let Some(size) = release.size {
        writer
            .write_event(Event::Start(BytesStart::new("size")))
            .ok();
        writer
            .write_event(Event::Text(BytesText::new(&size.to_string())))
            .ok();
        writer.write_event(Event::End(BytesEnd::new("size"))).ok();
    }

    // <enclosure>
    if !link.is_empty() {
        let mut enclosure = BytesStart::new("enclosure");
        enclosure.push_attribute(("url", link));
        if let Some(size) = release.size {
            enclosure.push_attribute(("length", size.to_string().as_str()));
        }
        enclosure.push_attribute(("type", "application/x-bittorrent"));
        writer.write_event(Event::Empty(enclosure)).ok();
    }

    // Torznab attributes
    for cat in &release.categories {
        let mut attr = BytesStart::new("torznab:attr");
        attr.push_attribute(("name", "category"));
        attr.push_attribute(("value", cat.to_string().as_str()));
        writer.write_event(Event::Empty(attr)).ok();
    }

    if let Some(seeders) = release.seeders {
        let mut attr = BytesStart::new("torznab:attr");
        attr.push_attribute(("name", "seeders"));
        attr.push_attribute(("value", seeders.to_string().as_str()));
        writer.write_event(Event::Empty(attr)).ok();
    }

    if let Some(peers) = release.peers {
        let mut attr = BytesStart::new("torznab:attr");
        attr.push_attribute(("name", "peers"));
        attr.push_attribute(("value", peers.to_string().as_str()));
        writer.write_event(Event::Empty(attr)).ok();
    }

    if let Some(size) = release.size {
        let mut attr = BytesStart::new("torznab:attr");
        attr.push_attribute(("name", "size"));
        attr.push_attribute(("value", size.to_string().as_str()));
        writer.write_event(Event::Empty(attr)).ok();
    }

    // downloadvolumefactor
    let mut attr = BytesStart::new("torznab:attr");
    attr.push_attribute(("name", "downloadvolumefactor"));
    attr.push_attribute(("value", release.download_volume_factor.to_string().as_str()));
    writer.write_event(Event::Empty(attr)).ok();

    // uploadvolumefactor
    let mut attr = BytesStart::new("torznab:attr");
    attr.push_attribute(("name", "uploadvolumefactor"));
    attr.push_attribute(("value", release.upload_volume_factor.to_string().as_str()));
    writer.write_event(Event::Empty(attr)).ok();

    writer.write_event(Event::End(BytesEnd::new("item"))).ok();
}
