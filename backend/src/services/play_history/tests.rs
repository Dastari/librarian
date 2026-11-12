//! Play history: coalescing, completion, statistics, sharing permissions and devices.

use std::collections::{BTreeSet, HashMap};

use async_graphql::{EmptySubscription, MergedObject, Request, Schema, Variables};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use graphql_orm::graphql::auth::AuthSubject;
use serde_json::{Value, json};

use super::devices::{RequestClientInfo, default_name, describe_user_agent};
use super::stats::{Period, aggregate, streak};
use super::*;
use crate::graphql::AuthUser;
use crate::services::database::DatabaseService;
use crate::services::graphql::entities::torrent::authorization_tests::{account, fixture};
use crate::services::graphql::mutations::DeviceActionMutations;
use crate::services::graphql::queries::StatisticsQueries;

fn at(seconds: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 2, 20, 0, 0).unwrap() + Duration::seconds(seconds)
}

fn cp(position: f64, playing: bool, seconds: i64) -> Checkpoint {
    Checkpoint {
        position,
        playing,
        at: at(seconds),
    }
}

// ---- Pure rules ------------------------------------------------------------------------

#[test]
fn steady_playing_is_credited_by_position_moved() {
    assert_eq!(credited_seconds(cp(0.0, true, 0), cp(10.0, true, 10)), 10.0);
    // Reports are a little uneven; never more than the wall time between them.
    assert_eq!(credited_seconds(cp(0.0, true, 0), cp(12.0, true, 10)), 10.0);
    assert_eq!(credited_seconds(cp(0.0, true, 0), cp(8.0, true, 10)), 8.0);
}

#[test]
fn faster_speech_counts_real_minutes_not_book_minutes() {
    // 2x speed: 30 seconds of the book in 15 seconds of listening.
    assert_eq!(
        credited_seconds(cp(100.0, true, 0), cp(130.0, true, 15)),
        15.0
    );
}

#[test]
fn paused_time_is_not_played() {
    assert_eq!(
        credited_seconds(cp(50.0, false, 0), cp(50.0, false, 600)),
        0.0
    );
    // Stalled while "playing" (buffering): the position did not move.
    assert_eq!(credited_seconds(cp(50.0, true, 0), cp(50.0, true, 15)), 0.0);
}

#[test]
fn resuming_from_pause_credits_what_moved() {
    assert_eq!(
        credited_seconds(cp(50.0, false, 0), cp(55.0, true, 15)),
        5.0
    );
}

#[test]
fn a_seek_counts_wall_time_never_the_distance() {
    // Forward an hour in 15 seconds of wall time.
    assert_eq!(
        credited_seconds(cp(60.0, true, 0), cp(3660.0, true, 15)),
        15.0
    );
    // Back to the start.
    assert_eq!(
        credited_seconds(cp(3000.0, true, 0), cp(10.0, true, 15)),
        15.0
    );
    // A long silence and a jump: capped at a minute.
    assert_eq!(
        credited_seconds(cp(60.0, true, 0), cp(5000.0, true, 900)),
        60.0
    );
    // Seeking while paused plays nothing.
    assert_eq!(
        credited_seconds(cp(60.0, false, 0), cp(900.0, true, 10)),
        0.0
    );
}

#[test]
fn reports_out_of_order_credit_nothing() {
    assert_eq!(credited_seconds(cp(0.0, true, 10), cp(10.0, true, 5)), 0.0);
    assert_eq!(
        credited_seconds(cp(0.0, true, 0), cp(f64::NAN, true, 10)),
        0.0
    );
}

#[test]
fn completion_uses_the_watched_rule() {
    assert!(!is_complete(900.0, Some(1000.0), None));
    assert!(is_complete(920.0, Some(1000.0), None));
    assert!(is_complete(
        10.0,
        Some(1000.0),
        Some("2026-11-02T20:00:00.000Z")
    ));
    assert!(!is_complete(10.0, None, None));
    assert!(!is_complete(10.0, Some(0.0), None));
}

#[test]
fn a_play_continues_within_the_resume_window() {
    assert!(continues(at(0), at(RESUME_WINDOW_SECS)));
    assert!(!continues(at(0), at(RESUME_WINDOW_SECS + 1)));
    assert!(!continues(at(10), at(0)));
}

#[test]
fn timestamps_match_the_web_clients_text() {
    assert_eq!(timestamp(at(0)), "2026-11-02T20:00:00.000Z");
    assert_eq!(parse_time("2026-11-02T20:00:00+00:00"), Some(at(0)));
    assert_eq!(parse_time("2026-11-03T06:00:00+10:00"), Some(at(0)));
}

#[test]
fn retention_setting_reads_days() {
    assert_eq!(retention_days(None), 0);
    assert_eq!(retention_days(Some("")), 0);
    assert_eq!(retention_days(Some("365")), 365);
    assert_eq!(retention_days(Some("\"90\"")), 90);
    assert_eq!(retention_days(Some("-5")), 0);
    assert_eq!(retention_days(Some("never")), 0);
}

#[test]
fn user_agents_identify_devices() {
    let ipad = describe_user_agent(
        "Mozilla/5.0 (iPad; CPU OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
    );
    assert_eq!(ipad.product.as_deref(), Some("Safari"));
    assert_eq!(ipad.platform.as_deref(), Some("iPadOS"));
    assert_eq!(ipad.form_factor, "tablet");
    assert_eq!(
        default_name(
            ipad.product.as_deref(),
            ipad.platform.as_deref(),
            ipad.form_factor
        ),
        "Safari on iPad"
    );

    let iphone = describe_user_agent(
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/124.0 Mobile/15E148 Safari/604.1",
    );
    assert_eq!(iphone.product.as_deref(), Some("Chrome"));
    assert_eq!(iphone.form_factor, "phone");

    let pixel = describe_user_agent(
        "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36",
    );
    assert_eq!(pixel.platform.as_deref(), Some("Android"));
    assert_eq!(pixel.form_factor, "phone");

    let tab = describe_user_agent(
        "Mozilla/5.0 (Linux; Android 13; SM-X700) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
    );
    assert_eq!(tab.form_factor, "tablet");

    let lg = describe_user_agent(
        "Mozilla/5.0 (Web0S; Linux/SmartTV) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/94.0 Safari/537.36 WebAppManager",
    );
    assert_eq!(lg.platform.as_deref(), Some("webOS"));
    assert_eq!(lg.form_factor, "tv");
    assert_eq!(default_name(None, None, "tv"), "TV");
    assert_eq!(
        default_name(
            lg.product.as_deref(),
            lg.platform.as_deref(),
            lg.form_factor
        ),
        "webOS TV"
    );

    let samsung = describe_user_agent(
        "Mozilla/5.0 (SMART-TV; LINUX; Tizen 6.5) AppleWebKit/537.36 (KHTML, like Gecko) 85.0 TV Safari/537.36",
    );
    assert_eq!(samsung.platform.as_deref(), Some("Tizen"));
    assert_eq!(samsung.form_factor, "tv");

    let edge = describe_user_agent(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36 Edg/124.0",
    );
    assert_eq!(edge.product.as_deref(), Some("Edge"));
    assert_eq!(edge.platform.as_deref(), Some("Windows"));
    assert_eq!(edge.form_factor, "desktop");
    assert_eq!(
        default_name(edge.product.as_deref(), edge.platform.as_deref(), "desktop"),
        "Edge on Windows"
    );

    let firefox = describe_user_agent(
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.4; rv:125.0) Gecko/20100101 Firefox/125.0",
    );
    assert_eq!(firefox.product.as_deref(), Some("Firefox"));
    assert_eq!(
        default_name(
            firefox.product.as_deref(),
            firefox.platform.as_deref(),
            "desktop"
        ),
        "Firefox on Mac"
    );

    let nothing = describe_user_agent("");
    assert_eq!(nothing.form_factor, "unknown");
    assert_eq!(default_name(None, None, "unknown"), "Unknown device");
}

// ---- Statistics ------------------------------------------------------------------------

fn play(kind: &str, started: DateTime<Utc>, seconds: f64) -> PlayHistory {
    PlayHistory {
        id: uuid::Uuid::new_v4().to_string(),
        user_id: "u1".into(),
        device_id: Some("d1".into()),
        device_name: Some("Living room TV".into()),
        playback_session_id: None,
        content_type: kind.into(),
        media_file_id: None,
        movie_id: None,
        episode_id: None,
        show_id: None,
        track_id: None,
        album_id: None,
        artist_id: None,
        audiobook_id: None,
        chapter_id: None,
        title: "Untitled".into(),
        parent_title: None,
        subtitle: None,
        started_at: timestamp(started),
        ended_at: timestamp(started + Duration::seconds(seconds as i64)),
        seconds_played: seconds,
        start_position: 0.0,
        end_position: seconds,
        duration: None,
        completed: false,
        play_method: Some("direct".into()),
        imported: false,
        source_progress_id: None,
        origin: None,
        origin_name: None,
        created_at: timestamp(started),
        updated_at: timestamp(started),
    }
}

fn episode(show: &str, name: &str, id: &str, started: DateTime<Utc>, seconds: f64) -> PlayHistory {
    PlayHistory {
        episode_id: Some(id.into()),
        show_id: Some(show.into()),
        parent_title: Some(name.into()),
        title: format!("Episode {id}"),
        ..play("episode", started, seconds)
    }
}

fn track(artist: &str, album: &str, id: &str, started: DateTime<Utc>, seconds: f64) -> PlayHistory {
    PlayHistory {
        track_id: Some(id.into()),
        album_id: Some(format!("album-{album}")),
        artist_id: Some(format!("artist-{artist}")),
        parent_title: Some(artist.into()),
        subtitle: Some(album.into()),
        title: format!("Track {id}"),
        device_id: Some("d2".into()),
        device_name: Some("Chrome on Mac".into()),
        ..play("track", started, seconds)
    }
}

fn day(d: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, d, hour, 0, 0).unwrap()
}

fn period(from: DateTime<Utc>, to: DateTime<Utc>, offset: i32) -> Period {
    Period {
        from,
        to,
        utc_offset_minutes: offset,
        now: to,
    }
}

#[test]
fn statistics_sum_days_weeks_and_kinds() {
    let rows = vec![
        episode("s1", "Severance", "e1", day(2, 20), 3000.0),
        episode("s1", "Severance", "e2", day(2, 21), 3000.0),
        episode("s2", "Slow Horses", "e3", day(4, 20), 2400.0),
        track("Radiohead", "OK Computer", "t1", day(4, 9), 240.0),
        track("Radiohead", "OK Computer", "t2", day(4, 9), 300.0),
        track("Björk", "Homogenic", "t3", day(10, 9), 200.0),
        PlayHistory {
            movie_id: Some("m1".into()),
            title: "Arrival".into(),
            imported: true,
            completed: true,
            device_id: None,
            device_name: None,
            ..play("movie", day(9, 12), 0.0)
        },
    ];
    let mut genres = HashMap::new();
    genres.insert(
        "s1".to_string(),
        vec!["Drama".to_string(), "Thriller".to_string()],
    );
    genres.insert("s2".to_string(), vec!["Thriller".to_string()]);
    genres.insert(
        "m1".to_string(),
        vec!["Science Fiction".to_string(), "Drama".to_string()],
    );
    let names = HashMap::from([("u1".to_string(), "Toby".to_string())]);
    let stats = aggregate(
        &rows,
        period(day(1, 0), day(14, 23), 0),
        &genres,
        &names,
        10,
    );

    assert_eq!(stats.totals.seconds_watched, 8400.0);
    assert_eq!(stats.totals.seconds_listened, 740.0);
    assert_eq!(stats.totals.plays, 7);
    assert_eq!(stats.totals.imported_plays, 1);
    assert_eq!(stats.totals.completed_plays, 1);
    assert_eq!(stats.totals.items, 7);
    assert_eq!(stats.totals.active_days, 4);

    assert_eq!(stats.days.len(), 14);
    let nov4 = stats.days.iter().find(|d| d.date == "2026-11-04").unwrap();
    assert_eq!(nov4.seconds_watched, 2400.0);
    assert_eq!(nov4.seconds_listened, 540.0);
    assert_eq!(nov4.plays, 3);
    // 2026-11-02 is a Monday: the first week holds the 2nd and 4th, the second the 9th and 10th.
    assert_eq!(stats.weeks[0].date, "2026-10-26");
    assert_eq!(stats.weeks[1].date, "2026-11-02");
    assert_eq!(stats.weeks[1].plays, 5);
    assert_eq!(stats.weeks[2].date, "2026-11-09");
    assert_eq!(stats.weeks[2].plays, 2);

    assert_eq!(stats.top_shows[0].title, "Severance");
    assert_eq!(stats.top_shows[0].seconds, 6000.0);
    assert_eq!(stats.top_shows[0].plays, 2);
    assert_eq!(stats.top_shows[0].entity_id.as_deref(), Some("s1"));
    assert_eq!(stats.top_artists[0].title, "Radiohead");
    assert_eq!(stats.top_albums[0].title, "OK Computer");
    assert_eq!(stats.top_albums[0].subtitle.as_deref(), Some("Radiohead"));
    // Imported plays count, with no time.
    assert_eq!(stats.top_movies[0].title, "Arrival");
    assert_eq!(stats.top_movies[0].seconds, 0.0);
    assert_eq!(stats.top_movies[0].plays, 1);
    // Severance's two episodes are Drama and Thriller; Slow Horses is Thriller.
    assert_eq!(stats.top_genres[0].title, "Thriller");
    assert_eq!(stats.top_genres[0].seconds, 8400.0);
    assert_eq!(stats.top_genres[1].title, "Drama");
    assert_eq!(stats.top_genres[1].plays, 3);

    assert_eq!(stats.devices[0].name, "Living room TV");
    assert!(
        stats
            .devices
            .iter()
            .any(|d| d.name == "Imported from watch progress")
    );
    assert_eq!(stats.users[0].name, "Toby");
    assert_eq!(stats.hours[20], 5400.0);
}

#[test]
fn local_days_follow_the_viewers_offset() {
    // 23:30 UTC on the 2nd is the 3rd in Brisbane (UTC+10).
    let late = Utc.with_ymd_and_hms(2026, 11, 2, 23, 30, 0).unwrap();
    let rows = vec![play("movie", late, 100.0)];
    let utc = aggregate(
        &rows,
        period(day(1, 0), day(5, 0), 0),
        &HashMap::new(),
        &HashMap::new(),
        5,
    );
    let bne = aggregate(
        &rows,
        period(day(1, 0), day(5, 0), 600),
        &HashMap::new(),
        &HashMap::new(),
        5,
    );
    let date_of = |stats: &stats::Aggregate| {
        stats
            .days
            .iter()
            .find(|d| d.plays > 0)
            .map(|d| d.date.clone())
    };
    assert_eq!(date_of(&utc).as_deref(), Some("2026-11-02"));
    assert_eq!(date_of(&bne).as_deref(), Some("2026-11-03"));
    assert_eq!(bne.hours[9], 100.0);
}

#[test]
fn plays_outside_the_period_are_left_out() {
    let rows = vec![
        play("movie", day(1, 10), 100.0),
        play("movie", day(20, 10), 50.0),
    ];
    let stats = aggregate(
        &rows,
        period(day(15, 0), day(25, 0), 0),
        &HashMap::new(),
        &HashMap::new(),
        5,
    );
    assert_eq!(stats.totals.plays, 1);
    assert_eq!(stats.totals.seconds_watched, 50.0);
}

#[test]
fn streaks_count_consecutive_days() {
    let d = |n: u32| NaiveDate::from_ymd_opt(2026, 11, n).unwrap();
    let days: BTreeSet<_> = [1, 2, 3, 5, 6, 8, 9].into_iter().map(d).collect();
    let s = streak(&days, d(9));
    assert_eq!(s.current, 2);
    assert_eq!(s.longest, 3);
    assert_eq!(s.longest_ended_on.as_deref(), Some("2026-11-03"));
    // Nothing yet today: yesterday's run is still current.
    assert_eq!(streak(&days, d(10)).current, 2);
    // A missed day breaks it.
    assert_eq!(streak(&days, d(11)).current, 0);
    assert_eq!(streak(&BTreeSet::new(), d(11)).longest, 0);
}

#[test]
fn ties_rank_by_plays_then_name() {
    let rows = vec![
        episode("b", "Beta", "1", day(2, 1), 100.0),
        episode("a", "Alpha", "2", day(2, 2), 100.0),
        episode("c", "Gamma", "3", day(2, 3), 50.0),
        episode("c", "Gamma", "4", day(2, 4), 50.0),
    ];
    let stats = aggregate(
        &rows,
        period(day(1, 0), day(3, 0), 0),
        &HashMap::new(),
        &HashMap::new(),
        2,
    );
    let titles: Vec<_> = stats.top_shows.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(titles, ["Gamma", "Alpha"]);
}

#[test]
fn shares_grant_their_viewer_or_everyone() {
    let share = |audience: &str, viewer: Option<&str>| StatsShare {
        id: "s".into(),
        user_id: "owner".into(),
        audience: audience.into(),
        viewer_user_id: viewer.map(str::to_owned),
        created_at: String::new(),
        updated_at: String::new(),
    };
    assert!(access::share_grants(&share("everyone", None), "anyone"));
    assert!(access::share_grants(
        &share("user", Some("friend")),
        "friend"
    ));
    assert!(!access::share_grants(
        &share("user", Some("friend")),
        "stranger"
    ));
    assert!(!access::share_grants(&share("user", None), "stranger"));
    assert!(!access::share_grants(&share("everyone", None), "owner"));
    assert!(!access::share_grants(
        &share("nonsense", Some("friend")),
        "friend"
    ));
}

// ---- Through the database and the generated schema -------------------------------------

#[derive(Default, MergedObject)]
struct Queries(
    PlayHistoryQueries,
    StatsShareQueries,
    DeviceQueries,
    PlaybackSessionQueries,
    StatisticsQueries,
);
#[derive(Default, MergedObject)]
struct Mutations(
    PlayHistoryMutations,
    StatsShareMutations,
    DeviceMutations,
    PlaybackSessionMutations,
    LibraryMutations,
    MovieMutations,
    UserMutations,
    DeviceActionMutations,
);
type TestSchema = Schema<Queries, Mutations, EmptySubscription>;

struct Fixture {
    db: Database,
    schema: TestSchema,
    user: AuthUser,
    other: AuthUser,
    third: AuthUser,
    admin: AuthUser,
}

impl Fixture {
    async fn new() -> Self {
        let db = DatabaseService::new(fixture().await).pool().clone();
        let user = account(&db, "member").await;
        let other = account(&db, "member").await;
        let third = account(&db, "member").await;
        let admin = account(&db, "admin").await;
        let schema = Schema::build(Queries::default(), Mutations::default(), EmptySubscription)
            .data(db.clone())
            .finish();
        Self {
            db,
            schema,
            user,
            other,
            third,
            admin,
        }
    }

    async fn execute(&self, document: &str, variables: Value, user: &AuthUser) -> Value {
        let response = self.execute_raw(document, variables, user).await;
        assert!(response.errors.is_empty(), "{response:?}");
        response.data.into_json().unwrap()
    }

    async fn execute_raw(
        &self,
        document: &str,
        variables: Value,
        user: &AuthUser,
    ) -> async_graphql::Response {
        self.schema
            .execute(
                Request::new(document)
                    .variables(Variables::from_json(variables))
                    .data(user.clone())
                    .data(user.user_id.clone())
                    .data(RequestClientInfo {
                        ip: Some("192.0.2.7".into()),
                        user_agent: Some("Mozilla/5.0 (iPad; CPU OS 17_4 like Mac OS X) AppleWebKit/605.1.15 Version/17.4 Mobile/15E148 Safari/604.1".into()),
                    })
                    .data(AuthSubject::from_parts(
                        user.user_id.clone(),
                        vec![user.role.clone().unwrap()],
                        vec![],
                        None,
                    )),
            )
            .await
    }

    async fn create(&self, entity: &str, input: Value, user: &AuthUser) -> String {
        let node = format!("{}{}", entity[..1].to_ascii_lowercase(), &entity[1..]);
        let field = format!("create{entity}");
        let data = self
            .execute(
                &format!("mutation($input:Create{entity}Input!){{{field}(input:$input){{success error {node}{{id}}}}}}"),
                json!({"input": input}),
                user,
            )
            .await;
        assert_eq!(data[&field]["success"], true, "{data}");
        data[&field][&node]["id"].as_str().unwrap().into()
    }

    async fn movie(&self, owner: &AuthUser, title: &str) -> String {
        let library = self.create("Library", json!({"userId":owner.user_id,"name":format!("{title} library"),"path":format!("/history-fixture/{}", uuid::Uuid::new_v4()),"libraryType":"movies","autoScan":false,"autoOrganize":false,"namingPattern":"","scanIntervalMinutes":60,"watchForChanges":false,"scanning":false}), &self.admin).await;
        self.create("Movie", json!({"userId":owner.user_id,"libraryId":library,"title":title,"genres":["Drama"],"castNames":[],"productionCountries":[],"spokenLanguages":[],"monitored":false,"wanted":false,"hasFile":false}), &self.admin).await
    }

    async fn session(
        &self,
        owner: &AuthUser,
        movie: &str,
        device: Option<&str>,
        start: i64,
    ) -> String {
        PlaybackSession::insert(
            &self.db,
            CreatePlaybackSessionInput {
                user_id: owner.user_id.clone(),
                content_type: Some("movie".into()),
                media_file_id: None,
                episode_id: None,
                movie_id: Some(movie.into()),
                track_id: None,
                audiobook_id: None,
                tv_show_id: None,
                album_id: None,
                chapter_id: None,
                current_position: 0.0,
                duration: Some(1000.0),
                volume: 1.0,
                is_muted: false,
                is_playing: true,
                started_at: timestamp(at(start)),
                last_updated_at: timestamp(at(start)),
                completed_at: None,
                device_id: device.map(str::to_owned),
                state: Some("playing".into()),
                play_method: Some("direct".into()),
                quality: Some("1080p".into()),
                bitrate: Some(8_000_000),
                ended_at: None,
                stop_requested_at: None,
            },
        )
        .await
        .unwrap()
        .id
    }

    /// A report from the player, as the web client sends it every few seconds.
    async fn report(&self, session: &str, position: f64, playing: bool, seconds: i64) {
        PlaybackSession::update_by_id(
            &self.db,
            &session.to_string(),
            UpdatePlaybackSessionInput {
                current_position: Some(position),
                is_playing: Some(playing),
                last_updated_at: Some(timestamp(at(seconds))),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    }

    async fn plays(&self, user: &AuthUser) -> Vec<PlayHistory> {
        let mut rows = PlayHistory::query(self.db.pool())
            .filter(PlayHistoryWhereInput {
                user_id: Some(StringFilter {
                    eq: Some(user.user_id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap();
        rows.sort_by(|a, b| a.started_at.cmp(&b.started_at));
        rows
    }
}

#[tokio::test]
async fn a_play_with_pauses_and_seeks_is_one_row_of_real_time() {
    let f = Fixture::new().await;
    let movie = f.movie(&f.user, "Arrival").await;
    let session = f.session(&f.user, &movie, None, 0).await;
    assert!(
        f.plays(&f.user).await.is_empty(),
        "nothing is recorded before anything plays"
    );

    f.report(&session, 15.0, true, 15).await;
    f.report(&session, 30.0, true, 30).await;
    // Paused for ten minutes, the player still reporting.
    f.report(&session, 30.0, false, 45).await;
    f.report(&session, 30.0, false, 645).await;
    // Resumed, then a seek forward 500 seconds.
    f.report(&session, 40.0, true, 655).await;
    f.report(&session, 545.0, true, 665).await;
    f.report(&session, 560.0, true, 680).await;

    let plays = f.plays(&f.user).await;
    assert_eq!(plays.len(), 1, "{plays:#?}");
    let p = &plays[0];
    assert_eq!(p.seconds_played, 15.0 + 15.0 + 10.0 + 10.0 + 15.0);
    assert_eq!(p.title, "Arrival");
    assert_eq!(p.content_type, "movie");
    assert_eq!(p.movie_id.as_deref(), Some(movie.as_str()));
    assert_eq!(p.play_method.as_deref(), Some("direct"));
    assert_eq!(p.started_at, timestamp(at(0)));
    assert_eq!(p.ended_at, timestamp(at(680)));
    assert_eq!(p.end_position, 560.0);
    assert!(!p.completed);
    assert!(!p.imported);
}

#[tokio::test]
async fn reaching_the_watched_point_completes_the_play() {
    let f = Fixture::new().await;
    let movie = f.movie(&f.user, "Heat").await;
    let session = f.session(&f.user, &movie, None, 0).await;
    f.report(&session, 900.0, true, 10).await; // a seek near the end: 10 seconds credited
    assert!(!f.plays(&f.user).await[0].completed);
    f.report(&session, 915.0, true, 25).await;
    f.report(&session, 925.0, true, 35).await;
    let plays = f.plays(&f.user).await;
    assert_eq!(plays.len(), 1);
    assert!(plays[0].completed);
    assert_eq!(plays[0].seconds_played, 35.0);
}

#[tokio::test]
async fn reopening_soon_continues_the_play_and_later_starts_another() {
    let f = Fixture::new().await;
    let movie = f.movie(&f.user, "Dune").await;
    let first = f.session(&f.user, &movie, Some("tv"), 0).await;
    f.report(&first, 20.0, true, 20).await;
    f.report(&first, 20.0, false, 25).await;

    // The player was closed and the same film reopened ten minutes later: one play.
    let second = f.session(&f.user, &movie, Some("tv"), 600).await;
    f.report(&second, 30.0, true, 610).await;
    let plays = f.plays(&f.user).await;
    assert_eq!(plays.len(), 1, "{plays:#?}");
    assert_eq!(plays[0].seconds_played, 30.0);
    assert_eq!(
        plays[0].playback_session_id.as_deref(),
        Some(second.as_str())
    );

    // On another device it is a separate play.
    let phone = f.session(&f.user, &movie, Some("phone"), 700).await;
    f.report(&phone, 40.0, true, 710).await;
    assert_eq!(f.plays(&f.user).await.len(), 2);

    // Resumed on the TV after two hours: a new play, even in the same session.
    f.report(&second, 30.0, false, 620).await;
    f.report(&second, 40.0, true, 620 + 2 * 3600).await;
    f.report(&second, 50.0, true, 630 + 2 * 3600).await;
    let plays = f.plays(&f.user).await;
    assert_eq!(plays.len(), 3, "{plays:#?}");
    assert!(
        plays
            .iter()
            .any(|p| p.started_at == timestamp(at(620)) && p.seconds_played == 20.0),
        "{plays:#?}"
    );
}

#[tokio::test]
async fn a_finished_play_is_not_continued_by_a_rewatch() {
    let f = Fixture::new().await;
    let movie = f.movie(&f.user, "Alien").await;
    let first = f.session(&f.user, &movie, None, 0).await;
    f.report(&first, 950.0, true, 10).await;
    let rewatch = f.session(&f.user, &movie, None, 60).await;
    f.report(&rewatch, 10.0, true, 70).await;
    let plays = f.plays(&f.user).await;
    assert_eq!(plays.len(), 2);
    assert!(plays[0].completed && !plays[1].completed);
}

#[tokio::test]
async fn internal_session_writes_are_not_reports() {
    let f = Fixture::new().await;
    let movie = f.movie(&f.user, "Ran").await;
    let session = f.session(&f.user, &movie, None, 0).await;
    PlaybackSession::update_by_id(
        &f.db,
        &session,
        UpdatePlaybackSessionInput {
            is_playing: Some(false),
            stop_requested_at: Some(Some(timestamp(at(900)))),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(f.plays(&f.user).await.is_empty());
}

#[tokio::test]
async fn history_is_read_and_deleted_by_its_owner_and_never_written_by_clients() {
    let f = Fixture::new().await;
    let mine = f.movie(&f.user, "Mine").await;
    let theirs = f.movie(&f.other, "Theirs").await;
    let s1 = f.session(&f.user, &mine, None, 0).await;
    f.report(&s1, 10.0, true, 10).await;
    let s2 = f.session(&f.other, &theirs, None, 0).await;
    f.report(&s2, 10.0, true, 10).await;

    let list = "{playHistories{edges{node{id userId title}} pageInfo{totalCount}}}";
    let data = f.execute(list, json!({}), &f.user).await;
    assert_eq!(data["playHistories"]["pageInfo"]["totalCount"], 1);
    assert_eq!(data["playHistories"]["edges"][0]["node"]["title"], "Mine");
    let data = f.execute(list, json!({}), &f.admin).await;
    assert_eq!(data["playHistories"]["pageInfo"]["totalCount"], 2);

    let create = f
        .execute_raw(
            "mutation($input:CreatePlayHistoryInput!){createPlayHistory(input:$input){success error}}",
            json!({"input":{"userId":f.user.user_id,"contentType":"movie","title":"Fake","startedAt":"2026-01-01T00:00:00.000Z","endedAt":"2026-01-01T09:00:00.000Z","secondsPlayed":32400.0,"startPosition":0.0,"endPosition":0.0,"completed":true,"imported":false}}),
            &f.user,
        )
        .await;
    assert!(
        !create.errors.is_empty()
            || create.data.into_json().unwrap()["createPlayHistory"]["success"] == false
    );
    let id = f.plays(&f.user).await[0].id.clone();
    let update = f
        .execute_raw(
            "mutation($id:String!){updatePlayHistory(id:$id,input:{secondsPlayed:99999.0}){success error}}",
            json!({"id": id}),
            &f.user,
        )
        .await;
    assert!(
        !update.errors.is_empty()
            || update.data.into_json().unwrap()["updatePlayHistory"]["success"] == false
    );
    assert_eq!(f.plays(&f.user).await[0].seconds_played, 10.0);

    // A filter naming someone else's plays deletes none of theirs: the generated bulk delete
    // refuses a set that includes rows the requester may not write.
    let _ = f
        .execute_raw(
            "mutation($where:PlayHistoryWhereInput!){deletePlayHistories(where:$where){success error deletedCount}}",
            json!({"where":{"userId":{"inList":[f.user.user_id, f.other.user_id]}}}),
            &f.user,
        )
        .await;
    assert_eq!(f.plays(&f.other).await.len(), 1);

    // "Clear my history", as the History page sends it, removes all of yours.
    let cleared = f
        .execute(
            "mutation($where:PlayHistoryWhereInput!){deletePlayHistories(where:$where){success error deletedCount}}",
            json!({"where":{"userId":{"eq":f.user.user_id}}}),
            &f.user,
        )
        .await;
    assert_eq!(cleared["deletePlayHistories"]["success"], true, "{cleared}");
    assert!(f.plays(&f.user).await.is_empty());
    assert_eq!(f.plays(&f.other).await.len(), 1);
}

const STATS: &str = "query($userId:String,$from:String,$to:String){playStatistics(userId:$userId,from:$from,to:$to,utcOffsetMinutes:0){userId userName totals{secondsWatched plays} topMovies{title seconds} recent{title}}}";

async fn stats_for(f: &Fixture, viewer: &AuthUser, target: &AuthUser) -> async_graphql::Response {
    f.execute_raw(
        STATS,
        json!({"userId": target.user_id, "from": timestamp(at(-3600)), "to": timestamp(at(3600))}),
        viewer,
    )
    .await
}

fn denied(response: &async_graphql::Response) -> bool {
    response
        .errors
        .iter()
        .any(|error| error.message.contains("not shared with you"))
}

#[tokio::test]
async fn statistics_are_private_until_shared_and_read_only_after() {
    let f = Fixture::new().await;
    let movie = f.movie(&f.user, "Paris, Texas").await;
    let session = f.session(&f.user, &movie, None, 0).await;
    f.report(&session, 60.0, true, 60).await;

    // Your own, and an administrator.
    for viewer in [&f.user, &f.admin] {
        let response = stats_for(&f, viewer, &f.user).await;
        assert!(response.errors.is_empty(), "{response:?}");
        let data = response.data.into_json().unwrap();
        assert_eq!(data["playStatistics"]["totals"]["secondsWatched"], 60.0);
        assert_eq!(
            data["playStatistics"]["topMovies"][0]["title"],
            "Paris, Texas"
        );
    }
    // Private by default: the same answer as a user who does not exist.
    assert!(denied(&stats_for(&f, &f.other, &f.user).await));
    let missing = AuthUser {
        user_id: "nobody".into(),
        ..f.other.clone()
    };
    assert!(denied(&stats_for(&f, &f.other, &missing).await));
    // The people picker shows only yourself.
    let people = f
        .execute(
            "{statisticsPeople{userId isSelf shared}}",
            json!({}),
            &f.other,
        )
        .await;
    assert_eq!(people["statisticsPeople"].as_array().unwrap().len(), 1);

    // Shared with one person: they can read, someone else cannot.
    let share = f
        .create(
            "StatsShare",
            json!({"userId": f.user.user_id, "audience": "user", "viewerUserId": f.other.user_id}),
            &f.user,
        )
        .await;
    assert!(stats_for(&f, &f.other, &f.user).await.errors.is_empty());
    assert!(denied(&stats_for(&f, &f.third, &f.user).await));
    let people = f
        .execute(
            "{statisticsPeople{userId isSelf shared}}",
            json!({}),
            &f.other,
        )
        .await;
    assert!(
        people["statisticsPeople"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["userId"] == f.user.user_id && p["shared"] == true)
    );

    // The viewer cannot see or touch the share itself, nor the raw history.
    let shares = f
        .execute("{statsShares{pageInfo{totalCount}}}", json!({}), &f.other)
        .await;
    assert_eq!(shares["statsShares"]["pageInfo"]["totalCount"], 0);
    let history = f
        .execute("{playHistories{pageInfo{totalCount}}}", json!({}), &f.other)
        .await;
    assert_eq!(history["playHistories"]["pageInfo"]["totalCount"], 0);
    let removed = f
        .execute_raw(
            "mutation($id:String!){deleteStatsShare(id:$id){success}}",
            json!({"id": share}),
            &f.other,
        )
        .await;
    assert!(
        !removed.errors.is_empty()
            || removed.data.into_json().unwrap()["deleteStatsShare"]["success"] == false
    );
    // Someone else cannot share your statistics on your behalf.
    // (Refused, or made a share of the caller's own: either way not the user's.)
    f.execute_raw(
        "mutation($input:CreateStatsShareInput!){createStatsShare(input:$input){success}}",
        json!({"input": {"userId": f.user.user_id, "audience": "everyone"}}),
        &f.third,
    )
    .await;
    assert!(denied(&stats_for(&f, &f.third, &f.user).await));

    // With everyone.
    f.create(
        "StatsShare",
        json!({"userId": f.user.user_id, "audience": "everyone"}),
        &f.user,
    )
    .await;
    assert!(stats_for(&f, &f.third, &f.user).await.errors.is_empty());

    // Taking both back makes them private again.
    f.execute(
        "mutation($where:StatsShareWhereInput!){deleteStatsShares(where:$where){success}}",
        json!({"where": {"userId": {"eq": f.user.user_id}}}),
        &f.user,
    )
    .await;
    assert!(denied(&stats_for(&f, &f.other, &f.user).await));
    assert!(denied(&stats_for(&f, &f.third, &f.user).await));
}

#[tokio::test]
async fn shares_are_validated() {
    let f = Fixture::new().await;
    let make = |input: Value| {
        let f = &f;
        async move {
            f.execute_raw(
                "mutation($input:CreateStatsShareInput!){createStatsShare(input:$input){success error}}",
                json!({"input": input}),
                &f.user,
            )
            .await
        }
    };
    let failed = |response: async_graphql::Response| {
        !response.errors.is_empty()
            || response.data.into_json().unwrap()["createStatsShare"]["success"] == false
    };
    assert!(failed(
        make(json!({"userId": f.user.user_id, "audience": "user", "viewerUserId": f.user.user_id}))
            .await
    ));
    assert!(failed(
        make(json!({"userId": f.user.user_id, "audience": "user", "viewerUserId": "nobody"})).await
    ));
    assert!(failed(
        make(json!({"userId": f.user.user_id, "audience": "user"})).await
    ));
    assert!(failed(
        make(json!({"userId": f.user.user_id, "audience": "friends"})).await
    ));
    assert!(!failed(
        make(
            json!({"userId": f.user.user_id, "audience": "user", "viewerUserId": f.other.user_id})
        )
        .await
    ));
    assert!(failed(make(json!({"userId": f.user.user_id, "audience": "user", "viewerUserId": f.other.user_id})).await), "twice");
    let update = f
        .execute_raw(
            "mutation($where:StatsShareWhereInput!){updateStatsShares(where:$where,input:{viewerUserId:\"x\"}){success}}",
            json!({"where": {"userId": {"eq": f.user.user_id}}}),
            &f.user,
        )
        .await;
    assert!(
        !update.errors.is_empty()
            || update.data.into_json().unwrap()["updateStatsShares"]["success"] == false
    );
}

#[tokio::test]
async fn everyones_statistics_need_an_administrator() {
    let f = Fixture::new().await;
    let query = "{playStatistics(allUsers:true){userId users{name plays}}}";
    assert!(
        !f.execute_raw(query, json!({}), &f.user)
            .await
            .errors
            .is_empty()
    );
    let data = f.execute(query, json!({}), &f.admin).await;
    assert!(data["playStatistics"]["userId"].is_null());
}

#[tokio::test]
async fn devices_check_in_rename_and_sign_out() {
    let f = Fixture::new().await;
    let check_in = "mutation($input:CheckInDeviceInput!){checkInDevice(input:$input){success device{id name nameIsCustom formFactor platform product lastIp userAgent}}}";
    let data = f
        .execute(
            check_in,
            json!({"input": {"clientId": "client-abc", "appVersion": "0.6.0"}}),
            &f.user,
        )
        .await;
    let device = &data["checkInDevice"]["device"];
    let id = device["id"].as_str().unwrap().to_owned();
    assert_eq!(device["name"], "Safari on iPad");
    assert_eq!(device["formFactor"], "tablet");
    assert_eq!(device["lastIp"], "192.0.2.7");

    // Checking in again is the same device; a client name is a suggestion until renamed.
    let data = f
        .execute(
            check_in,
            json!({"input": {"clientId": "client-abc", "name": "TV", "formFactor": "tv"}}),
            &f.user,
        )
        .await;
    assert_eq!(data["checkInDevice"]["device"]["id"], id);
    assert_eq!(data["checkInDevice"]["device"]["name"], "TV");
    let renamed = f
        .execute("mutation($id:String!){updateDevice(id:$id,input:{name:\"  Lounge  \"}){success device{name nameIsCustom}}}", json!({"id": id}), &f.user)
        .await;
    assert_eq!(renamed["updateDevice"]["device"]["name"], "Lounge");
    assert_eq!(renamed["updateDevice"]["device"]["nameIsCustom"], true);
    let data = f
        .execute(
            check_in,
            json!({"input": {"clientId": "client-abc", "name": "TV"}}),
            &f.user,
        )
        .await;
    assert_eq!(data["checkInDevice"]["device"]["name"], "Lounge");
    // Only the name can be edited.
    let spoof = f
        .execute_raw(
            "mutation($id:String!){updateDevice(id:$id,input:{lastIp:\"10.0.0.1\"}){success}}",
            json!({"id": id}),
            &f.user,
        )
        .await;
    assert!(
        !spoof.errors.is_empty()
            || spoof.data.into_json().unwrap()["updateDevice"]["success"] == false
    );
    // Clients cannot make devices up.
    let made_up = f
        .execute_raw("mutation($input:CreateDeviceInput!){createDevice(input:$input){success}}", json!({"input": {"userId": f.user.user_id, "clientId": "x", "name": "x", "nameIsCustom": false, "formFactor": "tv", "lastSeenAt": "2026-01-01T00:00:00.000Z"}}), &f.user)
        .await;
    assert!(
        !made_up.errors.is_empty()
            || made_up.data.into_json().unwrap()["createDevice"]["success"] == false
    );

    // Another member neither sees nor signs it out; an administrator sees it.
    let list = "{devices{pageInfo{totalCount}}}";
    assert_eq!(
        f.execute(list, json!({}), &f.other).await["devices"]["pageInfo"]["totalCount"],
        0
    );
    assert_eq!(
        f.execute(list, json!({}), &f.admin).await["devices"]["pageInfo"]["totalCount"],
        1
    );
    let sign_out = "mutation($id:String!){signOutDevice(id:$id){success revokedSessions}}";
    assert!(
        !f.execute_raw(sign_out, json!({"id": id}), &f.other)
            .await
            .errors
            .is_empty()
    );

    // A session cannot be bound to someone else's device: the link is dropped.
    let movie = f.movie(&f.other, "Theirs").await;
    let foreign = f
        .execute_raw(
            "mutation($input:CreatePlaybackSessionInput!){createPlaybackSession(input:$input){success}}",
            json!({"input": {"userId": f.other.user_id, "movieId": movie, "contentType": "movie", "deviceId": id, "currentPosition": 0.0, "volume": 1.0, "isMuted": false, "isPlaying": true, "startedAt": "2026-01-01T00:00:00.000Z", "lastUpdatedAt": "2026-01-01T00:00:00.000Z"}}),
            &f.other,
        )
        .await;
    let created = foreign.errors.is_empty()
        && foreign.data.into_json().unwrap()["createPlaybackSession"]["success"] == true;
    let sessions = PlaybackSession::query(f.db.pool())
        .fetch_all()
        .await
        .unwrap();
    assert!(
        sessions
            .iter()
            .all(|session| session.device_id.as_deref() != Some(id.as_str()))
    );
    let _ = created;

    // Signing out revokes the login session the device last checked in with.
    let device = Device::get(f.db.pool(), &id).await.unwrap().unwrap();
    let mut tx = MutationContext::begin(&f.db, TransactionMode::Default)
        .await
        .unwrap();
    tx.update_by_id::<Device>(
        &device.id,
        UpdateDeviceInput {
            session_id: Some(Some("session-9".into())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tx.commit_and_emit().await.unwrap();
    for (token, session) in [
        ("t1", "session-9"),
        ("t2", "session-9"),
        ("t3", "session-other"),
    ] {
        RefreshToken::insert(
            &f.db,
            CreateRefreshTokenInput {
                id: token.into(),
                user_id: f.user.user_id.clone(),
                token_hash: format!("hash-{token}"),
                session_id: session.into(),
                session_family_id: format!("family-{session}"),
                scopes: vec![],
                session: "{}".into(),
                grant_source: None,
                roles: None,
                authorization_roles: None,
                ip_address: None,
                user_agent: None,
                expires_at: "2099-01-01T00:00:00Z".into(),
                last_used_at: None,
                revoked_at: None,
                replaced_by_token_id: None,
                revocation_reason: None,
            },
        )
        .await
        .unwrap();
    }
    let data = f.execute(sign_out, json!({"id": id}), &f.user).await;
    assert_eq!(data["signOutDevice"]["revokedSessions"], 2);
    for (token, revoked) in [("t1", true), ("t2", true), ("t3", false)] {
        let row = RefreshToken::get(f.db.pool(), &token.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.revoked_at.is_some(), revoked, "{token}");
    }
    assert!(
        Device::get(f.db.pool(), &id)
            .await
            .unwrap()
            .unwrap()
            .signed_out_at
            .is_some()
    );
}

#[tokio::test]
async fn a_new_session_on_a_device_ends_its_old_one() {
    let f = Fixture::new().await;
    let device = super::devices::check_in(
        &f.db,
        &f.user,
        None,
        &RequestClientInfo::default(),
        super::devices::CheckIn {
            client_id: "tv-1".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let movie = f.movie(&f.user, "Stalker").await;
    let old = f.session(&f.user, &movie, Some(&device.id), 0).await;
    let new = f.session(&f.user, &movie, Some(&device.id), 30).await;
    let old = PlaybackSession::get(f.db.pool(), &old)
        .await
        .unwrap()
        .unwrap();
    let new = PlaybackSession::get(f.db.pool(), &new)
        .await
        .unwrap()
        .unwrap();
    assert!(old.ended_at.is_some() && !old.is_playing);
    assert!(new.ended_at.is_none());

    // Forgetting the device lets go of its sessions.
    f.execute(
        "mutation($id:String!){deleteDevice(id:$id){success}}",
        json!({"id": device.id}),
        &f.user,
    )
    .await;
    let new = PlaybackSession::get(f.db.pool(), &new.id)
        .await
        .unwrap()
        .unwrap();
    assert!(new.device_id.is_none() && new.ended_at.is_some());
}

#[tokio::test]
async fn watch_progress_imports_once_without_claiming_time() {
    let f = Fixture::new().await;
    let movie = f.movie(&f.user, "Tokyo Story").await;
    let movie_row = Movie::get(f.db.pool(), &movie).await.unwrap().unwrap();
    let file = MediaFile::insert(
        &f.db,
        CreateMediaFileInput {
            library_id: Some(movie_row.library_id.clone()),
            episode_id: None,
            movie_id: Some(movie.clone()),
            track_id: None,
            chapter_id: None,
            path: "/history-fixture/tokyo-story.mkv".into(),
            relative_path: None,
            original_name: None,
            size: 1024,
            container: Some("mkv".into()),
            video_codec: None,
            audio_codec: None,
            width: None,
            height: None,
            duration: None,
            bitrate: None,
            resolution: None,
            is_hdr: false,
            hdr_type: None,
            audio_channels: None,
            metadata: None,
            content_type: Some("movie".into()),
            added_at: now(),
            closed_captions: None,
            picture_crop: None,
            previews_checked_at: None,
            storyboard: None,
            analyzed_at: None,
            file_modified_at: None,
            match_type: None,
            matched_by_user_id: None,
            match_confirmed_at: None,
            quality_status: None,
        },
    )
    .await
    .unwrap();
    for (watched, position) in [(true, 7000.0), (false, 100.0)] {
        PlaybackProgress::insert(
            &f.db,
            CreatePlaybackProgressInput {
                user_id: f.user.user_id.clone(),
                media_file_id: Some(file.id.clone()),
                current_position: position,
                duration: Some(7200.0),
                progress_percent: position / 72.0,
                is_watched: watched,
                watched_at: watched.then(|| "2025-03-04T05:06:07Z".to_string()),
            },
        )
        .await
        .unwrap();
    }
    let outcome = import_from_progress(&f.db, None).await.unwrap();
    assert_eq!(outcome.imported, 1);
    let plays = f.plays(&f.user).await;
    assert_eq!(plays.len(), 1);
    assert!(plays[0].imported && plays[0].completed);
    assert_eq!(plays[0].seconds_played, 0.0);
    assert_eq!(plays[0].started_at, "2025-03-04T05:06:07.000Z");
    assert_eq!(plays[0].title, "Tokyo Story");
    assert_eq!(
        import_from_progress(&f.db, None).await.unwrap().imported,
        0,
        "once"
    );

    // An imported play is never continued by a new one.
    let session = f.session(&f.user, &movie, None, 0).await;
    f.report(&session, 10.0, true, 10).await;
    assert_eq!(f.plays(&f.user).await.len(), 2);

    // Retention keeps the new play and removes the year-old import.
    assert_eq!(prune(&f.db, 30, at(60)).await.unwrap(), 1);
    assert_eq!(prune(&f.db, 0, at(60)).await.unwrap(), 0);
    let plays = f.plays(&f.user).await;
    assert_eq!(plays.len(), 1);
    assert!(!plays[0].imported);
}

#[tokio::test]
async fn deleting_a_user_removes_their_history_devices_and_shares() {
    let f = Fixture::new().await;
    let movie = f.movie(&f.other, "Gone").await;
    let session = f.session(&f.other, &movie, None, 0).await;
    f.report(&session, 10.0, true, 10).await;
    f.create(
        "StatsShare",
        json!({"userId": f.user.user_id, "audience": "user", "viewerUserId": f.other.user_id}),
        &f.user,
    )
    .await;
    super::devices::check_in(
        &f.db,
        &f.other,
        None,
        &RequestClientInfo::default(),
        super::devices::CheckIn {
            client_id: "c".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    f.execute(
        "mutation($id:String!){deleteUser(id:$id){success}}",
        json!({"id": f.other.user_id}),
        &f.admin,
    )
    .await;
    assert!(f.plays(&f.other).await.is_empty());
    assert!(
        StatsShare::query(f.db.pool())
            .fetch_all()
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        Device::query(f.db.pool())
            .fetch_all()
            .await
            .unwrap()
            .is_empty()
    );
}
