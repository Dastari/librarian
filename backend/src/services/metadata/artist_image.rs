//! Where a music artist's picture comes from, without any API key.
//!
//! 1. **Exact match by MusicBrainz id.** MusicBrainz links an artist to a Wikidata item, and
//!    Wikidata's `P18` property names a Wikimedia Commons photo. No names are compared.
//! 2. **Deezer, for artists with no Wikidata photo.** Deezer's public search finds artists by name,
//!    so only a result whose name equals the artist's (ignoring case, punctuation and typographic
//!    apostrophes) is accepted, and Deezer's grey "no picture" placeholder never is.
//!
//! The parsing and matching here is pure so it can be tested against fixture JSON. The network
//! calls go through the shared outbound HTTP policy.

use anyhow::{Context, Result};
use serde_json::Value;
use tracing::{debug, info};
use url::Url;

use crate::services::http_client::{
    METADATA_RESPONSE_LIMIT, OutboundHttpProfile, outbound_client, response_json_limited,
    send_with_policy,
};
use crate::services::release_match::normalize_words;

/// Metadata cache operation for an image found through an artist's MusicBrainz id.
pub const IMAGE_BY_MBID_OPERATION: &str = "GetArtistImage";
/// Metadata cache operation for an image found by exact artist name.
pub const IMAGE_BY_NAME_OPERATION: &str = "GetArtistImageByName";

/// The widest Commons rendition asked for. Artwork serving resizes down from it per layout.
const COMMONS_IMAGE_WIDTH: &str = "1000";
const COMMONS_FILE_PATH: &str = "https://commons.wikimedia.org/wiki/Special:FilePath/";
const WIKIDATA_API: &str = "https://www.wikidata.org/w/api.php";
const DEEZER_ARTIST_SEARCH: &str = "https://api.deezer.com/search/artist";
/// Deezer fills an artist it has no picture for with the picture of an empty string.
const DEEZER_EMPTY_PICTURE_HASH: &str = "d41d8cd98f00b204e9800998ecf8427e";

/// The Wikidata item (`Q11885`) in a MusicBrainz artist's URL relations.
pub fn wikidata_id_from_relations(body: &Value) -> Option<String> {
    body.get("relations")?
        .as_array()?
        .iter()
        .filter(|relation| relation.get("type").and_then(Value::as_str) == Some("wikidata"))
        .filter_map(|relation| relation.pointer("/url/resource")?.as_str())
        .filter_map(|resource| Url::parse(resource).ok())
        .filter(|url| matches!(url.host_str(), Some("www.wikidata.org" | "wikidata.org")))
        .filter_map(|url| url.path_segments()?.next_back().map(str::to_string))
        .find(|id| is_wikidata_item_id(id))
}

fn is_wikidata_item_id(id: &str) -> bool {
    id.strip_prefix('Q')
        .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit()))
}

/// The Commons file name an entity's `P18` (image) claim points at. A claim marked preferred wins
/// over a normal one, and a deprecated claim is never used.
pub fn commons_file_from_claims(body: &Value) -> Option<String> {
    let claims = body.pointer("/claims/P18")?.as_array()?;
    let file_of = |rank: &str| {
        claims
            .iter()
            .filter(|claim| {
                claim
                    .get("rank")
                    .and_then(Value::as_str)
                    .unwrap_or("normal")
                    == rank
            })
            .filter(|claim| {
                claim.pointer("/mainsnak/snaktype").and_then(Value::as_str) == Some("value")
                    || claim.pointer("/mainsnak/snaktype").is_none()
            })
            .find_map(|claim| {
                claim
                    .pointer("/mainsnak/datavalue/value")?
                    .as_str()
                    .map(str::trim)
                    .filter(|file| !file.is_empty())
                    .map(str::to_string)
            })
    };
    file_of("preferred").or_else(|| file_of("normal"))
}

/// A sized rendition of a Commons file. `Special:FilePath` answers with a redirect to the
/// thumbnail on `upload.wikimedia.org`, and doesn't upscale a file narrower than the width asked.
pub fn commons_image_url(file_name: &str) -> Option<String> {
    let mut url = Url::parse(COMMONS_FILE_PATH).ok()?;
    url.path_segments_mut()
        .ok()?
        .pop_if_empty()
        .push(&file_name.trim().replace(' ', "_"));
    url.query_pairs_mut()
        .append_pair("width", COMMONS_IMAGE_WIDTH);
    Some(url.into())
}

/// Whether a Deezer picture URL is its stand-in for "no picture".
pub fn is_deezer_placeholder(picture_url: &str) -> bool {
    picture_url.trim().is_empty()
        || picture_url.contains("/artist//")
        || picture_url.contains(DEEZER_EMPTY_PICTURE_HASH)
}

/// The picture of the first Deezer search result named exactly `artist_name`.
pub fn deezer_artist_image(body: &Value, artist_name: &str) -> Option<String> {
    let wanted = normalize_words(artist_name);
    if wanted.is_empty() {
        return None;
    }
    body.get("data")?
        .as_array()?
        .iter()
        .filter(|artist| {
            artist
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| normalize_words(name) == wanted)
        })
        .filter_map(|artist| artist.get("picture_xl").and_then(Value::as_str))
        .find(|picture| !is_deezer_placeholder(picture))
        .map(str::to_string)
}

/// The cache key for a name: spelling differences share one entry.
pub fn name_cache_key(artist_name: &str) -> String {
    normalize_words(artist_name)
}

/// The Commons photo Wikidata holds for an item, as a sized image URL.
pub async fn wikidata_image(user_agent: &str, wikidata_id: &str) -> Result<Option<String>> {
    let client = outbound_client(OutboundHttpProfile::Metadata)?;
    let request = client
        .get(WIKIDATA_API)
        .header("User-Agent", user_agent)
        .query(&[
            ("action", "wbgetclaims"),
            ("entity", wikidata_id),
            ("property", "P18"),
            ("format", "json"),
        ]);
    let response = send_with_policy(request, "wikidata", "wbgetclaims").await?;
    if !response.status().is_success() {
        anyhow::bail!(
            "Wikidata returned HTTP {} for the image of {wikidata_id}",
            response.status()
        );
    }
    let body: Value = response_json_limited(response, METADATA_RESPONSE_LIMIT)
        .await
        .with_context(|| format!("Failed to read the Wikidata image claim of {wikidata_id}"))?;
    Ok(commons_file_from_claims(&body).and_then(|file| commons_image_url(&file)))
}

/// Deezer's picture for the artist with exactly this name.
pub async fn deezer_image(user_agent: &str, artist_name: &str) -> Result<Option<String>> {
    let client = outbound_client(OutboundHttpProfile::Metadata)?;
    let request = client
        .get(DEEZER_ARTIST_SEARCH)
        .header("User-Agent", user_agent)
        .query(&[("q", artist_name), ("limit", "10")]);
    let response = send_with_policy(request, "deezer", "search_artist").await?;
    if !response.status().is_success() {
        anyhow::bail!(
            "Deezer returned HTTP {} searching for artist '{artist_name}'",
            response.status()
        );
    }
    let body: Value = response_json_limited(response, METADATA_RESPONSE_LIMIT)
        .await
        .with_context(|| format!("Failed to read Deezer's search for artist '{artist_name}'"))?;
    let image = deezer_artist_image(&body, artist_name);
    match &image {
        Some(url) => {
            info!(artist = %artist_name, image_url = %url, "Deezer has a picture for artist '{artist_name}'");
        }
        None => {
            debug!(artist = %artist_name, "Deezer has no exact-name picture for artist '{artist_name}'");
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn wikidata_item_is_found_in_url_relations() {
        let body = json!({
            "id": "eeb1195b-f213-4ce1-b28c-8565211f8e43",
            "relations": [
                { "type": "official homepage", "url": { "resource": "https://www.gunsnroses.com/" } },
                { "type": "wikidata", "url": { "resource": "https://www.wikidata.org/wiki/Q11885" } },
                { "type": "wikidata", "url": { "resource": "https://www.wikidata.org/wiki/Q999" } }
            ]
        });
        assert_eq!(wikidata_id_from_relations(&body).as_deref(), Some("Q11885"));
    }

    #[test]
    fn wikidata_relation_must_be_a_wikidata_item() {
        let body = json!({ "relations": [
            { "type": "wikidata", "url": { "resource": "https://example.com/wiki/Q11885" } },
            { "type": "wikidata", "url": { "resource": "https://www.wikidata.org/wiki/Special:Search" } },
            { "type": "wikidata", "url": { "resource": "not a url" } },
            { "type": "wikipedia", "url": { "resource": "https://www.wikidata.org/wiki/Q5" } }
        ]});
        assert_eq!(wikidata_id_from_relations(&body), None);
        assert_eq!(wikidata_id_from_relations(&json!({})), None);
        assert_eq!(
            wikidata_id_from_relations(&json!({ "relations": null })),
            None
        );
    }

    fn claim(rank: &str, file: &str) -> Value {
        json!({
            "rank": rank,
            "mainsnak": {
                "snaktype": "value",
                "property": "P18",
                "datavalue": { "value": file, "type": "string" }
            }
        })
    }

    #[test]
    fn p18_prefers_the_preferred_claim_and_skips_deprecated_ones() {
        let body = json!({ "claims": { "P18": [
            claim("deprecated", "Wrong.jpg"),
            claim("normal", "Normal.jpg"),
            claim("preferred", "Best.jpg"),
        ]}});
        assert_eq!(commons_file_from_claims(&body).as_deref(), Some("Best.jpg"));

        let body = json!({ "claims": { "P18": [
            claim("deprecated", "Wrong.jpg"),
            claim("normal", "Normal.jpg"),
        ]}});
        assert_eq!(
            commons_file_from_claims(&body).as_deref(),
            Some("Normal.jpg")
        );

        let only_deprecated = json!({ "claims": { "P18": [claim("deprecated", "Wrong.jpg")] }});
        assert_eq!(commons_file_from_claims(&only_deprecated), None);
    }

    #[test]
    fn p18_is_absent_or_unset_without_an_image() {
        assert_eq!(commons_file_from_claims(&json!({ "claims": {} })), None);
        assert_eq!(commons_file_from_claims(&json!({})), None);
        let somevalue = json!({ "claims": { "P18": [{
            "rank": "normal",
            "mainsnak": { "snaktype": "somevalue", "property": "P18" }
        }]}});
        assert_eq!(commons_file_from_claims(&somevalue), None);
    }

    #[test]
    fn commons_url_is_a_sized_file_path_with_the_name_encoded() {
        assert_eq!(
            commons_image_url("Guns N' Roses 2010.jpg").as_deref(),
            Some(
                "https://commons.wikimedia.org/wiki/Special:FilePath/Guns_N'_Roses_2010.jpg?width=1000"
            )
        );
        let url = commons_image_url("Foo Fighters – Live #1 (100%?).jpg").unwrap();
        assert!(url.starts_with("https://commons.wikimedia.org/wiki/Special:FilePath/"));
        assert!(url.ends_with("?width=1000"));
        assert!(!url.contains(' '));
        // A name can't escape the path or add its own query.
        assert_eq!(url.matches('?').count(), 1);
        assert_eq!(url.matches('#').count(), 0);
        let parsed = Url::parse(&url).unwrap();
        assert_eq!(parsed.host_str(), Some("commons.wikimedia.org"));
    }

    #[test]
    fn deezer_placeholder_pictures_are_recognised() {
        assert!(is_deezer_placeholder(
            "https://cdn-images.dzcdn.net/images/artist//1000x1000-000000-80-0-0.jpg"
        ));
        assert!(is_deezer_placeholder(
            "https://e-cdns-images.dzcdn.net/images/artist/d41d8cd98f00b204e9800998ecf8427e/1000x1000-000000-80-0-0.jpg"
        ));
        assert!(is_deezer_placeholder(""));
        assert!(!is_deezer_placeholder(
            "https://cdn-images.dzcdn.net/images/artist/0a1b2c3d4e5f60718293a4b5c6d7e8f9/1000x1000-000000-80-0-0.jpg"
        ));
    }

    const REAL: &str = "https://cdn-images.dzcdn.net/images/artist/0a1b2c3d4e5f60718293a4b5c6d7e8f9/1000x1000-000000-80-0-0.jpg";
    const PLACEHOLDER: &str =
        "https://cdn-images.dzcdn.net/images/artist//1000x1000-000000-80-0-0.jpg";

    #[test]
    fn deezer_name_must_match_exactly_after_normalising() {
        let body = json!({ "data": [
            { "id": 1, "name": "Guns N' Roses Tribute", "picture_xl": PLACEHOLDER },
            { "id": 2, "name": "Guns N’ Roses", "picture_xl": REAL },
        ]});
        // Typographic and straight apostrophes are the same name; case and punctuation don't matter.
        assert_eq!(
            deezer_artist_image(&body, "Guns N' Roses").as_deref(),
            Some(REAL)
        );
        assert_eq!(
            deezer_artist_image(&body, "GUNS N ROSES!").as_deref(),
            Some(REAL)
        );
        assert_eq!(deezer_artist_image(&body, "Guns").as_deref(), None);
        assert_eq!(deezer_artist_image(&body, "Roses").as_deref(), None);
    }

    #[test]
    fn deezer_near_matches_and_placeholders_give_nothing() {
        let near = json!({ "data": [
            { "name": "Foo Fighter", "picture_xl": REAL },
            { "name": "Foo Fighters Tribute Band", "picture_xl": REAL },
        ]});
        assert_eq!(deezer_artist_image(&near, "Foo Fighters"), None);

        let placeholder = json!({ "data": [{ "name": "Foo Fighters", "picture_xl": PLACEHOLDER }]});
        assert_eq!(deezer_artist_image(&placeholder, "Foo Fighters"), None);

        assert_eq!(
            deezer_artist_image(&json!({ "data": [] }), "Foo Fighters"),
            None
        );
        assert_eq!(deezer_artist_image(&json!({}), "Foo Fighters"), None);
        assert_eq!(deezer_artist_image(&near, "  "), None);
    }

    #[test]
    fn deezer_skips_an_exact_match_without_a_picture_for_a_later_one() {
        let body = json!({ "data": [
            { "name": "Yes", "picture_xl": PLACEHOLDER },
            { "name": "Yes", "picture_xl": REAL },
        ]});
        assert_eq!(deezer_artist_image(&body, "yes").as_deref(), Some(REAL));
    }

    #[test]
    fn name_cache_key_ignores_spelling_differences() {
        assert_eq!(
            name_cache_key("Guns N’ Roses"),
            name_cache_key("guns n' roses")
        );
    }
}
