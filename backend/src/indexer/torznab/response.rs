//! Torznab XML response generation
//!
//! Generates RSS 2.0 XML with Torznab extensions.

use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};
use std::io::Cursor;

use crate::indexer::{ReleaseInfo, TorznabCapabilities};

/// Torznab response wrapper
pub enum TorznabResponse {
    Xml(String),
    Error(TorznabError),
}

impl TorznabResponse {
    /// Create a capabilities response
    pub fn capabilities(title: &str, caps: &TorznabCapabilities) -> Self {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        // XML declaration
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .ok();

        // <caps>
        let caps_elem = BytesStart::new("caps");
        writer.write_event(Event::Start(caps_elem)).ok();

        // <server title="..."/>
        let mut server = BytesStart::new("server");
        server.push_attribute(("title", title));
        writer.write_event(Event::Empty(server)).ok();

        // <limits default="..." max="..."/>
        if caps.limits_default.is_some() || caps.limits_max.is_some() {
            let mut limits = BytesStart::new("limits");
            if let Some(def) = caps.limits_default {
                limits.push_attribute(("default", def.to_string().as_str()));
            }
            if let Some(max) = caps.limits_max {
                limits.push_attribute(("max", max.to_string().as_str()));
            }
            writer.write_event(Event::Empty(limits)).ok();
        }

        // <searching>
        writer
            .write_event(Event::Start(BytesStart::new("searching")))
            .ok();

        // <search available="yes" supportedParams="q"/>
        write_search_element(&mut writer, "search", caps.search_available, "q");

        // <tv-search available="yes" supportedParams="q,season,ep,imdbid"/>
        let tv_params = build_tv_params(caps);
        write_search_element(
            &mut writer,
            "tv-search",
            caps.tv_search_available(),
            &tv_params,
        );

        // <movie-search available="yes" supportedParams="q,imdbid"/>
        let movie_params = build_movie_params(caps);
        write_search_element(
            &mut writer,
            "movie-search",
            caps.movie_search_available(),
            &movie_params,
        );

        // <music-search available="yes" supportedParams="q,album,artist"/>
        let music_params = build_music_params(caps);
        write_search_element(
            &mut writer,
            "music-search",
            caps.music_search_available(),
            &music_params,
        );

        // <book-search available="yes" supportedParams="q,title,author"/>
        let book_params = build_book_params(caps);
        write_search_element(
            &mut writer,
            "book-search",
            caps.book_search_available(),
            &book_params,
        );

        writer
            .write_event(Event::End(BytesEnd::new("searching")))
            .ok();

        // <categories>
        writer
            .write_event(Event::Start(BytesStart::new("categories")))
            .ok();

        // Write category mappings
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

        // </caps>
        writer.write_event(Event::End(BytesEnd::new("caps"))).ok();

        let xml = String::from_utf8(writer.into_inner().into_inner()).unwrap_or_default();
        TorznabResponse::Xml(xml)
    }

    /// Create a search results response
    pub fn search_results(
        title: &str,
        description: &str,
        link: &str,
        releases: Vec<ReleaseInfo>,
    ) -> Self {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        // XML declaration
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .ok();

        // <rss version="2.0" xmlns:atom="..." xmlns:torznab="...">
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
        write_text_element(&mut writer, "title", title);

        // <description>
        write_text_element(&mut writer, "description", description);

        // <link>
        write_text_element(&mut writer, "link", link);

        // <language>en-us</language>
        write_text_element(&mut writer, "language", "en-us");

        // Write each release as an <item>
        for release in releases {
            write_release_item(&mut writer, &release);
        }

        // </channel>
        writer
            .write_event(Event::End(BytesEnd::new("channel")))
            .ok();

        // </rss>
        writer.write_event(Event::End(BytesEnd::new("rss"))).ok();

        let xml = String::from_utf8(writer.into_inner().into_inner()).unwrap_or_default();
        TorznabResponse::Xml(xml)
    }
}

impl IntoResponse for TorznabResponse {
    fn into_response(self) -> Response {
        match self {
            TorznabResponse::Xml(xml) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
                xml,
            )
                .into_response(),
            TorznabResponse::Error(err) => err.into_response(),
        }
    }
}

/// Torznab error response
pub struct TorznabError {
    code: i32,
    description: String,
    status: StatusCode,
}

impl TorznabError {
    pub fn new(code: i32, description: impl Into<String>, status: StatusCode) -> Self {
        Self {
            code,
            description: description.into(),
            status,
        }
    }

    pub fn unauthorized(msg: &str) -> Self {
        Self::new(100, msg, StatusCode::UNAUTHORIZED)
    }

    pub fn not_found(msg: &str) -> Self {
        Self::new(201, msg, StatusCode::NOT_FOUND)
    }

    pub fn bad_request(msg: &str) -> Self {
        Self::new(201, msg, StatusCode::BAD_REQUEST)
    }

    pub fn function_not_available(msg: &str) -> Self {
        Self::new(203, msg, StatusCode::BAD_REQUEST)
    }

    pub fn indexer_error(msg: &str) -> Self {
        Self::new(900, msg, StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn to_xml(&self) -> String {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .ok();

        let mut error = BytesStart::new("error");
        error.push_attribute(("code", self.code.to_string().as_str()));
        error.push_attribute(("description", self.description.as_str()));
        writer.write_event(Event::Empty(error)).ok();

        String::from_utf8(writer.into_inner().into_inner()).unwrap_or_default()
    }
}

impl IntoResponse for TorznabError {
    fn into_response(self) -> Response {
        // For Torznab, we return the error as XML but with the appropriate HTTP status
        let xml = self.to_xml();
        (
            self.status,
            [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
            xml,
        )
            .into_response()
    }
}

// Helper functions for XML generation

fn write_text_element(writer: &mut Writer<Cursor<Vec<u8>>>, name: &str, text: &str) {
    writer.write_event(Event::Start(BytesStart::new(name))).ok();
    writer.write_event(Event::Text(BytesText::new(text))).ok();
    writer.write_event(Event::End(BytesEnd::new(name))).ok();
}

fn write_search_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    name: &str,
    available: bool,
    params: &str,
) {
    let mut elem = BytesStart::new(name);
    elem.push_attribute(("available", if available { "yes" } else { "no" }));
    elem.push_attribute(("supportedParams", params));
    writer.write_event(Event::Empty(elem)).ok();
}

fn write_torznab_attr(writer: &mut Writer<Cursor<Vec<u8>>>, name: &str, value: &str) {
    let mut attr = BytesStart::new("torznab:attr");
    attr.push_attribute(("name", name));
    attr.push_attribute(("value", value));
    writer.write_event(Event::Empty(attr)).ok();
}

fn write_release_item(writer: &mut Writer<Cursor<Vec<u8>>>, release: &ReleaseInfo) {
    writer
        .write_event(Event::Start(BytesStart::new("item")))
        .ok();

    // Basic elements
    write_text_element(writer, "title", &release.title);
    write_text_element(writer, "guid", &release.guid);

    if let Some(ref details) = release.details {
        write_text_element(writer, "comments", details);
    }

    // Publication date
    let pub_date = format_rfc2822(&release.publish_date);
    write_text_element(writer, "pubDate", &pub_date);

    // Size
    if let Some(size) = release.size {
        write_text_element(writer, "size", &size.to_string());
    }

    // Description
    if let Some(ref desc) = release.description {
        write_text_element(writer, "description", desc);
    }

    // Link (download URL or magnet)
    let link = release
        .link
        .as_ref()
        .or(release.magnet_uri.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("");
    write_text_element(writer, "link", link);

    // Categories
    for cat in &release.categories {
        write_text_element(writer, "category", &cat.to_string());
    }

    // Enclosure
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
        write_torznab_attr(writer, "category", &cat.to_string());
    }

    if let Some(seeders) = release.seeders {
        write_torznab_attr(writer, "seeders", &seeders.to_string());
    }

    if let Some(peers) = release.peers {
        write_torznab_attr(writer, "peers", &peers.to_string());
    }

    if let Some(size) = release.size {
        write_torznab_attr(writer, "size", &size.to_string());
    }

    if let Some(grabs) = release.grabs {
        write_torznab_attr(writer, "grabs", &grabs.to_string());
    }

    if let Some(imdb) = release.imdb {
        write_torznab_attr(writer, "imdb", &format!("tt{:07}", imdb));
        write_torznab_attr(writer, "imdbid", &format!("tt{:07}", imdb));
    }

    if let Some(tvdb) = release.tvdb_id {
        write_torznab_attr(writer, "tvdbid", &tvdb.to_string());
    }

    if let Some(tmdb) = release.tmdb {
        write_torznab_attr(writer, "tmdbid", &tmdb.to_string());
    }

    if let Some(ref info_hash) = release.info_hash {
        write_torznab_attr(writer, "infohash", info_hash);
    }

    if let Some(ref magnet) = release.magnet_uri {
        write_torznab_attr(writer, "magneturl", magnet);
    }

    if let Some(ref poster) = release.poster {
        write_torznab_attr(writer, "coverurl", poster);
    }

    write_torznab_attr(
        writer,
        "downloadvolumefactor",
        &release.download_volume_factor.to_string(),
    );
    write_torznab_attr(
        writer,
        "uploadvolumefactor",
        &release.upload_volume_factor.to_string(),
    );

    if let Some(ratio) = release.minimum_ratio {
        write_torznab_attr(writer, "minimumratio", &ratio.to_string());
    }

    if let Some(seed_time) = release.minimum_seed_time {
        write_torznab_attr(writer, "minimumseedtime", &seed_time.to_string());
    }

    writer.write_event(Event::End(BytesEnd::new("item"))).ok();
}

fn build_tv_params(caps: &TorznabCapabilities) -> String {
    use crate::indexer::TvSearchParam;

    let mut params = vec!["q"];

    for param in &caps.tv_search_params {
        match param {
            TvSearchParam::Season => params.push("season"),
            TvSearchParam::Ep => params.push("ep"),
            TvSearchParam::ImdbId => params.push("imdbid"),
            TvSearchParam::TvdbId => params.push("tvdbid"),
            TvSearchParam::RId => params.push("rid"),
            TvSearchParam::TmdbId => params.push("tmdbid"),
            TvSearchParam::TvmazeId => params.push("tvmazeid"),
            TvSearchParam::TraktId => params.push("traktid"),
            TvSearchParam::DoubanId => params.push("doubanid"),
            TvSearchParam::Year => params.push("year"),
            TvSearchParam::Genre => params.push("genre"),
            TvSearchParam::Q => {} // Already included
        }
    }

    params.join(",")
}

fn build_movie_params(caps: &TorznabCapabilities) -> String {
    use crate::indexer::MovieSearchParam;

    let mut params = vec!["q"];

    for param in &caps.movie_search_params {
        match param {
            MovieSearchParam::ImdbId => params.push("imdbid"),
            MovieSearchParam::TmdbId => params.push("tmdbid"),
            MovieSearchParam::TraktId => params.push("traktid"),
            MovieSearchParam::DoubanId => params.push("doubanid"),
            MovieSearchParam::Year => params.push("year"),
            MovieSearchParam::Genre => params.push("genre"),
            MovieSearchParam::Q => {}
        }
    }

    params.join(",")
}

fn build_music_params(caps: &TorznabCapabilities) -> String {
    use crate::indexer::MusicSearchParam;

    let mut params = vec!["q"];

    for param in &caps.music_search_params {
        match param {
            MusicSearchParam::Album => params.push("album"),
            MusicSearchParam::Artist => params.push("artist"),
            MusicSearchParam::Label => params.push("label"),
            MusicSearchParam::Track => params.push("track"),
            MusicSearchParam::Year => params.push("year"),
            MusicSearchParam::Genre => params.push("genre"),
            MusicSearchParam::Q => {}
        }
    }

    params.join(",")
}

fn build_book_params(caps: &TorznabCapabilities) -> String {
    use crate::indexer::BookSearchParam;

    let mut params = vec!["q"];

    for param in &caps.book_search_params {
        match param {
            BookSearchParam::Title => params.push("title"),
            BookSearchParam::Author => params.push("author"),
            BookSearchParam::Publisher => params.push("publisher"),
            BookSearchParam::Year => params.push("year"),
            BookSearchParam::Genre => params.push("genre"),
            BookSearchParam::Q => {}
        }
    }

    params.join(",")
}

fn format_rfc2822(dt: &DateTime<Utc>) -> String {
    dt.format("%a, %d %b %Y %H:%M:%S %z").to_string()
}
