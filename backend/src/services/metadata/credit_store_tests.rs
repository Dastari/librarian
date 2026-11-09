//! Saving credits, details and people through the generated mutations, on a throwaway database.

use super::*;
use crate::services::database::summary_tests::{library, movie, user};
use crate::services::metadata::credits::{CreditKind, parse_tmdb_movie, parse_tmdb_person};
use crate::services::{AuthConfig, DatabaseServiceConfig, GraphqlServiceConfig, ServicesManager};

struct Fixture {
    _temp: tempfile::TempDir,
    _services: std::sync::Arc<ServicesManager>,
    db: Database,
    schema: LibrarianSchema,
}

async fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let services = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: format!("sqlite://{}", temp.path().join("credits.db").display()),
            connect_timeout: std::time::Duration::from_secs(5),
        })
        .add_service(AuthConfig::for_tests())
        .add_service(GraphqlServiceConfig { server_port: 0 })
        .start()
        .await
        .unwrap();
    let db = services.get_database().await.unwrap().pool().clone();
    let schema = services
        .get_graphql()
        .await
        .unwrap()
        .schema()
        .await
        .unwrap();
    Fixture {
        _temp: temp,
        _services: services,
        db,
        schema,
    }
}

fn recorded(name: &str) -> Value {
    let path = format!(
        "{}/tests/fixtures/credits/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

async fn credits_of(db: &Database, movie_id: &str) -> Vec<Credit> {
    Credit::query(db.pool())
        .filter(CreditWhereInput {
            movie_id: Some(string_eq(movie_id)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .unwrap()
}

#[tokio::test]
async fn a_refresh_replaces_a_titles_credits_instead_of_adding_to_them() {
    let f = fixture().await;
    let owner = user(&f.db).await;
    let library = library(&f.db, &owner).await;
    let film = movie(&f.db, &library).await;
    let target = TitleTarget::new(TargetKind::Movie, &film.id, &film.user_id);
    let title = parse_tmdb_movie(&recorded("tmdb_movie_603.json"));
    let expected = title.credits.len();
    assert!(expected > 8);

    let mut store = CreditStore::new(f.schema.clone(), f.db.clone());
    let first = store
        .replace_credits(&target, "tmdb", &title.credits)
        .await
        .unwrap();
    assert_eq!(first.created, expected);
    let saved = credits_of(&f.db, &film.id).await;
    assert_eq!(saved.len(), expected);
    assert!(
        saved
            .iter()
            .all(|c| c.user_id == film.user_id && c.target_kind == "movie")
    );
    let neo = saved
        .iter()
        .find(|c| c.character_name.as_deref() == Some("Thomas A. Anderson / Neo"))
        .unwrap();
    assert_eq!(neo.kind, "cast");
    assert_eq!(neo.credit_order, Some(0));

    // The same document again, through a fresh store (nothing remembered): nothing changes.
    let mut store = CreditStore::new(f.schema.clone(), f.db.clone());
    let again = store
        .replace_credits(&target, "tmdb", &title.credits)
        .await
        .unwrap();
    assert_eq!(
        again,
        ReplaceSummary {
            unchanged: expected,
            ..Default::default()
        }
    );
    let after = credits_of(&f.db, &film.id).await;
    assert_eq!(after.len(), expected);
    let ids: HashSet<&str> = saved.iter().map(|c| c.id.as_str()).collect();
    assert!(
        after.iter().all(|c| ids.contains(c.id.as_str())),
        "rows are kept, not recreated"
    );

    // The provider dropped one credit, renamed a character and added a person.
    let mut changed = title.credits.clone();
    let dropped = changed.remove(
        changed
            .iter()
            .position(|c| c.person.name == "Hugo Weaving")
            .unwrap(),
    );
    let trinity = changed
        .iter_mut()
        .find(|c| c.character.as_deref() == Some("Trinity"))
        .unwrap();
    trinity.order = Some(7);
    let mut newcomer = changed[0].clone();
    newcomer.person = PersonRef {
        name: "Joe Pantoliano".into(),
        tmdb_id: Some(532),
        ..Default::default()
    };
    newcomer.character = Some("Cypher".into());
    changed.push(newcomer);
    let third = store
        .replace_credits(&target, "tmdb", &changed)
        .await
        .unwrap();
    assert_eq!((third.created, third.updated, third.deleted), (1, 1, 1));
    let finally = credits_of(&f.db, &film.id).await;
    assert_eq!(finally.len(), expected);
    assert!(
        !finally
            .iter()
            .any(|c| c.character_name == dropped.character)
    );
    assert_eq!(
        finally
            .iter()
            .find(|c| c.character_name.as_deref() == Some("Trinity"))
            .unwrap()
            .credit_order,
        Some(7)
    );

    // Credits from another source are their own set.
    let mut other = CreditStore::new(f.schema.clone(), f.db.clone());
    other
        .replace_credits(&target, "audiobook", &[])
        .await
        .unwrap();
    assert_eq!(credits_of(&f.db, &film.id).await.len(), expected);

    // Details: one row per title, rewritten in place; a locked field keeps what it had.
    store
        .save_title_detail(&target, "tmdb", &title.facts, &[], "2026-11-09T00:00:00Z")
        .await
        .unwrap();
    let mut facts = title.facts.clone();
    facts.keywords = vec!["changed".into()];
    facts.studios = vec!["Another Studio".into()];
    store
        .save_title_detail(
            &target,
            "tmdb",
            &facts,
            &["keywords".to_string()],
            "2026-11-10T00:00:00Z",
        )
        .await
        .unwrap();
    let details = TitleDetail::query(f.db.pool())
        .filter(target.detail_filter())
        .fetch_all()
        .await
        .unwrap();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].keywords, title.facts.keywords);
    assert_eq!(details[0].studios, vec!["Another Studio"]);
    assert_eq!(details[0].fetched_at, "2026-11-10T00:00:00Z");
    let videos: Vec<Value> = serde_json::from_str(details[0].videos.as_deref().unwrap()).unwrap();
    assert_eq!(videos[0]["key"], "vKQi3bBA1y8");
    assert!(
        details[0]
            .external_ids
            .as_deref()
            .unwrap()
            .contains("tt0133093")
    );
}

#[tokio::test]
async fn people_are_matched_across_sources_by_their_ids() {
    let f = fixture().await;
    let mut store = CreditStore::new(f.schema.clone(), f.db.clone());

    // TMDB first, then its own record adds the IMDb and Wikidata ids.
    let keanu = store
        .resolve_person(&PersonRef {
            name: "Keanu Reeves".into(),
            tmdb_id: Some(6384),
            ..Default::default()
        })
        .await
        .unwrap();
    let row = Person::get(f.db.pool(), &keanu).await.unwrap().unwrap();
    store
        .save_person_details(
            &row,
            &parse_tmdb_person(&recorded("tmdb_person_6384.json")).unwrap(),
            "2026-11-09T00:00:00Z",
        )
        .await
        .unwrap();
    let row = Person::get(f.db.pool(), &keanu).await.unwrap().unwrap();
    assert_eq!(row.imdb_id.as_deref(), Some("nm0000206"));
    assert_eq!(row.birthday.as_deref(), Some("1964-09-02"));
    assert_eq!(row.image_urls.as_ref().map(Vec::len), Some(3));

    // MusicBrainz names him with his IMDb id: the same person, who gains the MusicBrainz id.
    let mut store = CreditStore::new(f.schema.clone(), f.db.clone());
    let from_musicbrainz = store
        .resolve_person(&PersonRef {
            name: "Keanu Reeves".into(),
            imdb_id: Some("nm0000206".into()),
            musicbrainz_id: Some("mb-keanu".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(from_musicbrainz, keanu);
    let mut store = CreditStore::new(f.schema.clone(), f.db.clone());
    let by_musicbrainz_alone = store
        .resolve_person(&PersonRef {
            name: "Keanu".into(),
            musicbrainz_id: Some("mb-keanu".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_musicbrainz_alone, keanu);
    let row = Person::get(f.db.pool(), &keanu).await.unwrap().unwrap();
    assert_eq!(row.musicbrainz_artist_id.as_deref(), Some("mb-keanu"));
    assert_eq!(row.tmdb_person_id, 6384);
    assert_eq!(
        row.name, "Keanu Reeves",
        "another source never renames a TMDB person"
    );

    // A name alone never matches a person who has ids.
    let named = store
        .resolve_person(&PersonRef::named("Keanu Reeves"))
        .await
        .unwrap();
    assert_ne!(named, keanu);
    let named_row = Person::get(f.db.pool(), &named).await.unwrap().unwrap();
    assert!(named_row.tmdb_person_id < 0, "a stand-in, not a TMDB id");
    // …but two books' narrator with the same name is one person.
    let mut store = CreditStore::new(f.schema.clone(), f.db.clone());
    assert_eq!(
        store
            .resolve_person(&PersonRef::named(" keanu  reeves "))
            .await
            .unwrap(),
        named
    );

    // Known first from MusicBrainz (with a Wikidata id), later from TMDB: one person, who then
    // holds the real TMDB id in place of the stand-in.
    let slash = store
        .resolve_person(&PersonRef {
            name: "Saul Hudson".into(),
            musicbrainz_id: Some("mb-slash".into()),
            wikidata_id: Some("Q180453".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    let before = Person::get(f.db.pool(), &slash).await.unwrap().unwrap();
    assert!(before.tmdb_person_id < 0);
    let mut store = CreditStore::new(f.schema.clone(), f.db.clone());
    let from_tmdb = store
        .resolve_person(&PersonRef {
            name: "Slash".into(),
            tmdb_id: Some(19998),
            wikidata_id: Some("Q180453".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(from_tmdb, slash);
    let after = Person::get(f.db.pool(), &slash).await.unwrap().unwrap();
    assert_eq!(after.tmdb_person_id, 19998);
    assert_eq!(after.name, "Slash", "TMDB's name wins once it is known");

    let everyone = Person::query(f.db.pool()).fetch_all().await.unwrap();
    assert_eq!(everyone.len(), 3);
}

#[tokio::test]
async fn credits_of_one_kind_keep_their_kind() {
    let f = fixture().await;
    let owner = user(&f.db).await;
    let library = library(&f.db, &owner).await;
    let film = movie(&f.db, &library).await;
    let target = TitleTarget::new(TargetKind::Movie, &film.id, &film.user_id);
    let mut store = CreditStore::new(f.schema.clone(), f.db.clone());
    let title = parse_tmdb_movie(&recorded("tmdb_movie_603.json"));
    store
        .replace_credits(&target, "tmdb", &title.credits)
        .await
        .unwrap();
    let saved = credits_of(&f.db, &film.id).await;
    let crew = saved.iter().filter(|c| c.kind == "crew").count();
    let cast = saved.iter().filter(|c| c.kind == "cast").count();
    assert_eq!(
        crew,
        title
            .credits
            .iter()
            .filter(|c| c.kind == CreditKind::Crew)
            .count()
    );
    assert_eq!(cast, 4);
    assert!(
        saved
            .iter()
            .filter(|c| c.kind == "crew")
            .all(|c| c.character_name.is_none())
    );
}
