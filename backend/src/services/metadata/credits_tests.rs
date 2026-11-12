//! Parsing recorded provider documents (`tests/fixtures/credits/`); nothing here calls a provider.

use super::*;

fn fixture(name: &str) -> Value {
    let path = format!(
        "{}/tests/fixtures/credits/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(&path).expect("fixture should exist"))
        .expect("fixture should be JSON")
}

fn find<'a>(credits: &'a [CreditEntry], name: &str, job: Option<&str>) -> Vec<&'a CreditEntry> {
    credits
        .iter()
        .filter(|c| c.person.name == name && (job.is_none() || c.job.as_deref() == job))
        .collect()
}

#[test]
fn movie_document_gives_cast_crew_and_every_fact() {
    let title = parse_tmdb_movie(&fixture("tmdb_movie_603.json"));

    let cast: Vec<&CreditEntry> = title
        .credits
        .iter()
        .filter(|c| c.kind == CreditKind::Cast)
        .collect();
    // The blank name is dropped and billing order wins over document order.
    assert_eq!(cast.len(), 4);
    assert_eq!(cast[0].person.name, "Keanu Reeves");
    assert_eq!(
        cast[0].character.as_deref(),
        Some("Thomas A. Anderson / Neo")
    );
    assert_eq!(cast[0].person.tmdb_id, Some(6384));
    assert_eq!(
        cast[0].person.profile_url.as_deref(),
        Some("https://image.tmdb.org/t/p/original/4D0PpNI0kmP58hgrwGC3wCjxhnm.jpg")
    );
    assert_eq!(
        cast[0].provider_credit_id.as_deref(),
        Some("52fe425bc3a36847f80181bd")
    );
    assert!(
        cast.iter()
            .all(|c| !c.guest && c.department.as_deref() == Some("Acting"))
    );

    let crew: Vec<&CreditEntry> = title
        .credits
        .iter()
        .filter(|c| c.kind == CreditKind::Crew)
        .collect();
    // Joel Silver is listed twice as Producer; one credit is kept.
    assert_eq!(
        find(&title.credits, "Joel Silver", Some("Producer")).len(),
        1
    );
    // Lilly Wachowski directed and wrote: two credits, one per job.
    assert_eq!(find(&title.credits, "Lilly Wachowski", None).len(), 2);
    // Directors lead the crew, then writers, then producers.
    assert_eq!(crew[0].job.as_deref(), Some("Director"));
    assert_eq!(crew[1].job.as_deref(), Some("Director"));
    assert_eq!(crew[2].job.as_deref(), Some("Writer"));
    assert!(matches!(
        crew[3].job.as_deref(),
        Some("Producer" | "Executive Producer")
    ));
    assert_eq!(
        find(&title.credits, "Barrie M. Osborne", None)[0]
            .job
            .as_deref(),
        Some("Executive Producer")
    );
    let orders: Vec<i32> = crew.iter().map(|c| c.order.unwrap()).collect();
    assert!(
        orders.windows(2).all(|w| w[0] < w[1]),
        "crew orders follow the list"
    );

    let facts = &title.facts;
    assert_eq!(facts.provider_id.as_deref(), Some("movie/603"));
    assert_eq!(
        facts.studios,
        vec![
            "Village Roadshow Pictures",
            "Groucho II Film Partnership",
            "Silver Pictures",
            "Warner Bros. Pictures"
        ]
    );
    assert_eq!(
        facts.keywords,
        vec![
            "saving the world",
            "artificial intelligence (a.i.)",
            "dystopia"
        ]
    );
    assert_eq!(facts.origin_countries, vec!["US"]);
    assert_eq!(
        facts.homepage.as_deref(),
        Some("http://www.warnerbros.com/matrix")
    );
    // The theatrical certification wins over the premiere's empty one; a country with none is left out.
    let ratings: Vec<(&str, &str, Option<&str>)> = facts
        .content_ratings
        .iter()
        .map(|r| (r.country.as_str(), r.rating.as_str(), r.kind.as_deref()))
        .collect();
    assert_eq!(
        ratings,
        vec![
            ("US", "R", Some("theatrical")),
            ("GB", "15", Some("theatrical")),
            ("DE", "16", Some("physical"))
        ]
    );
    assert_eq!(facts.alternative_titles.len(), 3);
    assert_eq!(facts.alternative_titles[1].title, "マトリックス");
    assert_eq!(
        facts.alternative_titles[2].kind.as_deref(),
        Some("informal")
    );
    // Official trailers first; a video without a key is dropped; YouTube and Vimeo get links.
    assert_eq!(facts.videos.len(), 4);
    assert_eq!(facts.videos[0].key, "vKQi3bBA1y8");
    assert_eq!(
        facts.videos[0].url.as_deref(),
        Some("https://www.youtube.com/watch?v=vKQi3bBA1y8")
    );
    assert_eq!(facts.videos[1].key, "fanKey");
    assert!(
        facts
            .videos
            .iter()
            .any(|v| v.url.as_deref() == Some("https://vimeo.com/123456"))
    );
    assert_eq!(
        facts.external_ids.get("tmdb").map(String::as_str),
        Some("603")
    );
    assert_eq!(
        facts.external_ids.get("imdb").map(String::as_str),
        Some("tt0133093")
    );
    assert_eq!(
        facts.external_ids.get("wikidata").map(String::as_str),
        Some("Q83495")
    );
    assert_eq!(
        facts.external_ids.get("facebook").map(String::as_str),
        Some("TheMatrixMovie")
    );
    assert!(!facts.external_ids.contains_key("instagram"));
}

#[test]
fn show_document_gives_aggregate_credits_creators_and_networks() {
    let title = parse_tmdb_show(&fixture("tmdb_show_1399.json"));
    let facts = &title.facts;
    assert_eq!(facts.provider_id.as_deref(), Some("tv/1399"));
    assert_eq!(facts.networks, vec!["HBO"]);
    assert_eq!(facts.studios.len(), 3);
    assert_eq!(facts.keywords, vec!["based on novel or book", "dragon"]);
    assert_eq!(
        facts.content_ratings.len(),
        2,
        "an empty rating is left out"
    );
    assert_eq!(facts.content_ratings[0].rating, "TV-MA");
    assert_eq!(facts.alternative_titles[0].title, "Le Trône de fer");
    assert_eq!(
        facts.external_ids.get("tvdb").map(String::as_str),
        Some("121361")
    );
    assert_eq!(
        facts.external_ids.get("imdb").map(String::as_str),
        Some("tt0944947")
    );
    assert_eq!(facts.videos[0].name, "Season 1 Trailer");

    // One cast credit per character played, each with its own episode count.
    let barristan = find(&title.credits, "Ian McElhinney", None);
    assert_eq!(barristan.len(), 2);
    assert_eq!(barristan[0].episode_count, Some(20));
    assert_eq!(
        find(&title.credits, "Peter Dinklage", None)[0].episode_count,
        Some(67)
    );

    // One crew credit per job; creators are added from `created_by` and lead the crew.
    let benioff = find(&title.credits, "David Benioff", None);
    let jobs: Vec<&str> = benioff.iter().filter_map(|c| c.job.as_deref()).collect();
    assert!(jobs.contains(&"Executive Producer"));
    assert!(jobs.contains(&"Writer"));
    assert!(jobs.contains(&"Creator"));
    let weiss = find(&title.credits, "D. B. Weiss", Some("Creator"));
    assert_eq!(weiss.len(), 1);
    let creator_order = weiss[0].order.unwrap();
    assert!(
        title
            .credits
            .iter()
            .filter(|c| c.kind == CreditKind::Crew && c.job.as_deref() != Some("Creator"))
            .all(|c| c.order.unwrap() > creator_order)
    );
    assert_eq!(
        find(&title.credits, "Alan Taylor", None)[0].job.as_deref(),
        Some("Director")
    );
}

#[test]
fn season_document_gives_guest_stars_and_crew_per_episode() {
    let season = parse_tmdb_season(&fixture("tmdb_season_1399_1.json"));
    assert_eq!(season.season_number, 1);
    assert_eq!(season.credits.len(), 2);
    assert_eq!(season.episodes.len(), 2);

    let pilot = &season.episodes[0];
    assert_eq!(pilot.episode_number, 1);
    assert_eq!(pilot.tmdb_episode_id, Some(63056));
    let guests: Vec<&CreditEntry> = pilot.credits.iter().filter(|c| c.guest).collect();
    assert_eq!(guests.len(), 2);
    assert!(guests.iter().all(|c| c.kind == CreditKind::Cast));
    assert_eq!(guests[0].person.name, "Susan Brown");
    assert_eq!(guests[0].character.as_deref(), Some("Septa Mordane"));
    let crew: Vec<&CreditEntry> = pilot
        .credits
        .iter()
        .filter(|c| c.kind == CreditKind::Crew)
        .collect();
    assert_eq!(crew.len(), 2);
    assert!(crew.iter().all(|c| !c.guest));
    assert_eq!(crew[0].job.as_deref(), Some("Director"));
    assert_eq!(season.episodes[1].credits.len(), 1);
}

#[test]
fn person_document_gives_biography_ids_and_pictures() {
    let details = parse_tmdb_person(&fixture("tmdb_person_6384.json")).unwrap();
    assert_eq!(details.person.name, "Keanu Reeves");
    assert_eq!(details.person.tmdb_id, Some(6384));
    assert_eq!(details.person.imdb_id.as_deref(), Some("nm0000206"));
    assert_eq!(details.person.wikidata_id.as_deref(), Some("Q43416"));
    assert_eq!(
        details.person.known_for_department.as_deref(),
        Some("Acting")
    );
    assert_eq!(details.birthday.as_deref(), Some("1964-09-02"));
    assert_eq!(details.deathday, None);
    assert_eq!(details.place_of_birth.as_deref(), Some("Beirut, Lebanon"));
    assert!(
        details
            .biography
            .as_deref()
            .unwrap()
            .starts_with("Keanu Charles Reeves")
    );
    assert_eq!(details.also_known_as.len(), 2);
    // The profile picture first, then the rest by votes, without repeats.
    assert_eq!(details.image_urls.len(), 3);
    assert!(details.image_urls[0].ends_with("/4D0PpNI0kmP58hgrwGC3wCjxhnm.jpg"));
    assert!(details.image_urls[1].ends_with("/bOlYWhVuOiU6azC4Bw6zlXZ5QTC.jpg"));
}

#[test]
fn find_document_gives_the_show_id() {
    assert_eq!(
        parse_tmdb_find_show(&fixture("tmdb_find_tvdb_121361.json")),
        Some(1399)
    );
    assert_eq!(
        parse_tmdb_find_show(&serde_json::json!({ "tv_results": [] })),
        None
    );
}

#[test]
fn musicbrainz_release_gives_release_recording_and_work_credits() {
    let release = parse_musicbrainz_release(&fixture("musicbrainz_release_appetite.json"));
    assert_eq!(release.release_id, "a2a3a7ae-b4a6-4d0b-9e46-5f7f3e9a2b0f");
    assert_eq!(release.labels, vec!["Geffen Records"]);

    let jobs: Vec<(&str, &str)> = release
        .credits
        .iter()
        .map(|c| (c.person.name.as_str(), c.job.as_deref().unwrap()))
        .collect();
    assert_eq!(
        jobs,
        vec![
            ("Mike Clink", "Producer"),
            ("Steve Thompson", "Mixing Engineer"),
            ("Tom Zutaut", "Executive Producer")
        ]
    );
    assert!(release.credits.iter().all(|c| c.kind == CreditKind::Crew));
    assert_eq!(
        release.credits[0].person.musicbrainz_id.as_deref(),
        Some("6d4e7c9a-5c4b-4f9c-8c2a-8a0d8f8f0c11")
    );

    let jungle = &release.recordings["rec-jungle"];
    let guitar = jungle
        .iter()
        .find(|c| c.job.as_deref() == Some("Guitar"))
        .unwrap();
    assert_eq!(guitar.kind, CreditKind::Cast);
    assert_eq!(guitar.person.name, "Saul Hudson");
    // Credited under another name on this release.
    assert_eq!(guitar.character.as_deref(), Some("Slash"));
    assert!(
        jungle
            .iter()
            .any(|c| c.job.as_deref() == Some("Lead Vocals") && c.person.name == "Axl Rose")
    );
    // Writers come from the recording's work.
    let writers: Vec<(&str, &str)> = jungle
        .iter()
        .filter(|c| c.department.as_deref() == Some("Writing"))
        .map(|c| (c.person.name.as_str(), c.job.as_deref().unwrap()))
        .collect();
    assert_eq!(
        writers,
        vec![
            ("Axl Rose", "Composer"),
            ("Axl Rose", "Lyricist"),
            ("Saul Hudson", "Composer")
        ]
    );
    assert!(
        !release.recordings.contains_key("rec-easy"),
        "a recording with no credits has no entry"
    );
}

#[test]
fn audiobook_authors_and_narrators_become_people_known_by_name() {
    let credits = audiobook_credits(
        &["Andy Weir".into(), " andy weir ".into()],
        &["Ray Porter".into(), "".into()],
    );
    assert_eq!(credits.len(), 2);
    assert_eq!(credits[0].job.as_deref(), Some("Author"));
    assert_eq!(credits[0].kind, CreditKind::Crew);
    assert_eq!(credits[1].job.as_deref(), Some("Narrator"));
    assert_eq!(credits[1].kind, CreditKind::Cast);
    assert!(credits.iter().all(|c| !c.person.has_external_id()));
}

#[test]
fn stand_in_ids_are_negative_stable_and_distinct() {
    let a = stand_in_tmdb_id("name:ray porter", 0);
    assert!(a < 0);
    assert_eq!(a, stand_in_tmdb_id("name:ray porter", 0));
    assert_ne!(a, stand_in_tmdb_id("name:ray porter", 1));
    assert_ne!(a, stand_in_tmdb_id("name:andy weir", 0));
    for attempt in 0..1000 {
        let id = stand_in_tmdb_id("mb:x", attempt);
        assert!((i32::MIN + 1..=-1).contains(&id));
    }
    assert_eq!(name_key("  Ray  Porter! "), "ray porter");
    assert_eq!(name_key("Jean-Luc Picard"), "jean luc picard");
    assert_eq!(
        person_stand_in_key(&PersonRef {
            name: "Axl Rose".into(),
            musicbrainz_id: Some("abc".into()),
            ..Default::default()
        }),
        "mb:abc"
    );
}

#[test]
fn caps_keep_the_first_credits() {
    let cast: Vec<Value> = (0..(MAX_CAST + 20))
        .map(|i| serde_json::json!({ "id": i + 1, "name": format!("Actor {i}"), "character": "Extra", "order": i }))
        .collect();
    let title =
        parse_tmdb_movie(&serde_json::json!({ "id": 1, "credits": { "cast": cast, "crew": [] } }));
    assert_eq!(title.credits.len(), MAX_CAST);
    assert_eq!(title.credits[0].person.name, "Actor 0");
}
