//! Statistics over play history (design.md Q135): pure aggregation of already-authorized rows.
//!
//! The rows are read through the generated entity layer and summed here; nothing is cached.
//! Days are the viewer's local days (`utc_offset_minutes`), weeks start on Monday, and a play
//! counts on the day it started. Imported plays count as plays with no time.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use async_graphql::SimpleObject;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};

use crate::graphql::entities::PlayHistory;

use super::parse_time;

/// The most days a period lists one by one; longer periods still have weeks.
pub const MAX_DAYS: i64 = 400;

#[derive(Clone, Debug, Default, PartialEq, SimpleObject)]
#[graphql(name = "PlayStatisticsTotals", rename_fields = "camelCase")]
pub struct Totals {
    pub seconds_watched: f64,
    pub seconds_listened: f64,
    pub plays: i32,
    pub completed_plays: i32,
    pub imported_plays: i32,
    /// Different things played (an episode, a track, a film, a book).
    pub items: i32,
    pub active_days: i32,
}

#[derive(Clone, Debug, Default, PartialEq, SimpleObject)]
#[graphql(name = "PlayStatisticsStreak", rename_fields = "camelCase")]
pub struct Streak {
    /// Days in a row up to today (or yesterday, when nothing was played yet today).
    pub current: i32,
    pub longest: i32,
    /// The last day of the longest run.
    pub longest_ended_on: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, SimpleObject)]
#[graphql(name = "PlayStatisticsBucket", rename_fields = "camelCase")]
pub struct Bucket {
    /// The day, or the Monday that starts the week (YYYY-MM-DD).
    pub date: String,
    pub seconds_watched: f64,
    pub seconds_listened: f64,
    pub plays: i32,
}

#[derive(Clone, Debug, Default, PartialEq, SimpleObject)]
#[graphql(name = "PlayStatisticsTopItem", rename_fields = "camelCase")]
pub struct TopItem {
    /// Stable key: the entity id, or the name when there is none (authors, genres).
    pub key: String,
    /// `show`, `movie`, `artist`, `album`, `audiobook`, `author` or `genre`.
    pub kind: String,
    /// The entity, when the play named one; it may since have been removed.
    pub entity_id: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub seconds: f64,
    pub plays: i32,
}

#[derive(Clone, Debug, Default, PartialEq, SimpleObject)]
#[graphql(name = "PlayStatisticsDevice", rename_fields = "camelCase")]
pub struct DeviceUsage {
    pub device_id: Option<String>,
    pub name: String,
    pub seconds: f64,
    pub plays: i32,
    pub last_played_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, SimpleObject)]
#[graphql(name = "PlayStatisticsUser", rename_fields = "camelCase")]
pub struct UserUsage {
    pub user_id: String,
    pub name: String,
    pub seconds: f64,
    pub plays: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Aggregate {
    pub totals: Totals,
    pub streak: Streak,
    pub days: Vec<Bucket>,
    pub weeks: Vec<Bucket>,
    /// Seconds by local hour of the day, 0–23.
    pub hours: Vec<f64>,
    pub top_shows: Vec<TopItem>,
    pub top_movies: Vec<TopItem>,
    pub top_artists: Vec<TopItem>,
    pub top_albums: Vec<TopItem>,
    pub top_audiobooks: Vec<TopItem>,
    pub top_authors: Vec<TopItem>,
    pub top_genres: Vec<TopItem>,
    pub devices: Vec<DeviceUsage>,
    pub users: Vec<UserUsage>,
}

/// The local period being summed.
#[derive(Clone, Copy, Debug)]
pub struct Period {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub utc_offset_minutes: i32,
    /// "Today" for the current streak.
    pub now: DateTime<Utc>,
}

impl Period {
    pub fn local_date(&self, at: DateTime<Utc>) -> NaiveDate {
        (at + Duration::minutes(self.utc_offset_minutes as i64)).date_naive()
    }
    fn local_hour(&self, at: DateTime<Utc>) -> usize {
        (at + Duration::minutes(self.utc_offset_minutes as i64)).hour() as usize
    }
}

pub fn is_listening(content_type: &str) -> bool {
    matches!(content_type, "track" | "audiobook")
}

fn monday(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

#[derive(Default)]
struct Tally {
    kind: &'static str,
    entity_id: Option<String>,
    title: String,
    subtitle: Option<String>,
    seconds: f64,
    plays: i32,
}

fn add(
    map: &mut HashMap<String, Tally>,
    kind: &'static str,
    entity_id: Option<&str>,
    title: Option<&str>,
    subtitle: Option<&str>,
    row: &PlayHistory,
) {
    let title = title.map(str::trim).filter(|title| !title.is_empty());
    let Some(key) = entity_id
        .map(str::to_owned)
        .or_else(|| title.map(str::to_lowercase))
    else {
        return;
    };
    let tally = map.entry(key).or_insert_with(|| Tally {
        kind,
        entity_id: entity_id.map(str::to_owned),
        ..Default::default()
    });
    // Rows come oldest first, so the newest name wins.
    if let Some(title) = title {
        tally.title = title.to_owned();
    }
    if subtitle.is_some() {
        tally.subtitle = subtitle.map(str::to_owned);
    }
    tally.seconds += row.seconds_played;
    tally.plays += 1;
}

fn ranked(map: HashMap<String, Tally>, top: usize) -> Vec<TopItem> {
    let mut items: Vec<TopItem> = map
        .into_iter()
        .filter(|(_, tally)| !tally.title.is_empty())
        .map(|(key, tally)| TopItem {
            key,
            kind: tally.kind.to_owned(),
            entity_id: tally.entity_id,
            title: tally.title,
            subtitle: tally.subtitle,
            seconds: tally.seconds,
            plays: tally.plays,
        })
        .collect();
    items.sort_by(|a, b| {
        b.seconds
            .total_cmp(&a.seconds)
            .then(b.plays.cmp(&a.plays))
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    items.truncate(top);
    items
}

/// Longest and current runs of consecutive days in `days`.
pub fn streak(days: &BTreeSet<NaiveDate>, today: NaiveDate) -> Streak {
    let mut longest = 0;
    let mut longest_end = None;
    let mut run = 0;
    let mut previous: Option<NaiveDate> = None;
    for &day in days {
        run = match previous {
            Some(previous) if day - previous == Duration::days(1) => run + 1,
            _ => 1,
        };
        if run > longest {
            longest = run;
            longest_end = Some(day);
        }
        previous = Some(day);
    }
    let mut current = 0;
    let mut day = if days.contains(&today) {
        today
    } else {
        today - Duration::days(1)
    };
    while days.contains(&day) {
        current += 1;
        day -= Duration::days(1);
    }
    Streak {
        current,
        longest,
        longest_ended_on: longest_end.map(|day| day.to_string()),
    }
}

/// Sum `rows` (any order) over `period`. `genres` maps a movie, show or album id to its genres;
/// `names` a user id to the name shown for them.
pub fn aggregate(
    rows: &[PlayHistory],
    period: Period,
    genres: &HashMap<String, Vec<String>>,
    names: &HashMap<String, String>,
    top: usize,
) -> Aggregate {
    let mut rows: Vec<(&PlayHistory, DateTime<Utc>)> = rows
        .iter()
        .filter_map(|row| Some((row, parse_time(&row.started_at)?)))
        .filter(|(_, at)| *at >= period.from && *at <= period.to)
        .collect();
    rows.sort_by_key(|(_, at)| *at);

    let first_day = period.local_date(period.from);
    let last_day = period.local_date(period.to);
    let mut days: BTreeMap<NaiveDate, Bucket> = BTreeMap::new();
    if (last_day - first_day).num_days() < MAX_DAYS {
        let mut day = first_day;
        while day <= last_day {
            days.insert(
                day,
                Bucket {
                    date: day.to_string(),
                    ..Default::default()
                },
            );
            day += Duration::days(1);
        }
    }
    let mut weeks: BTreeMap<NaiveDate, Bucket> = BTreeMap::new();
    let mut week = monday(first_day);
    while week <= last_day {
        weeks.insert(
            week,
            Bucket {
                date: week.to_string(),
                ..Default::default()
            },
        );
        week += Duration::days(7);
    }

    let mut totals = Totals::default();
    let mut hours = vec![0.0; 24];
    let mut items = BTreeSet::new();
    let mut active = BTreeSet::new();
    let (mut shows, mut movies, mut artists, mut albums, mut books, mut authors, mut genre_map) = (
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    );
    let mut devices: HashMap<String, DeviceUsage> = HashMap::new();
    let mut users: HashMap<String, UserUsage> = HashMap::new();

    for (row, at) in &rows {
        let listening = is_listening(&row.content_type);
        let seconds = row.seconds_played.max(0.0);
        if listening {
            totals.seconds_listened += seconds;
        } else {
            totals.seconds_watched += seconds;
        }
        totals.plays += 1;
        totals.completed_plays += i32::from(row.completed);
        totals.imported_plays += i32::from(row.imported);
        let item = row
            .episode_id
            .as_ref()
            .or(row.movie_id.as_ref())
            .or(row.track_id.as_ref())
            .or(row.chapter_id.as_ref())
            .or(row.audiobook_id.as_ref())
            .cloned()
            .unwrap_or_else(|| format!("{}:{}", row.content_type, row.title));
        items.insert(item);
        let day = period.local_date(*at);
        active.insert(day);
        hours[period.local_hour(*at)] += seconds;
        for bucket in [days.get_mut(&day), weeks.get_mut(&monday(day))]
            .into_iter()
            .flatten()
        {
            if listening {
                bucket.seconds_listened += seconds;
            } else {
                bucket.seconds_watched += seconds;
            }
            bucket.plays += 1;
        }

        match row.content_type.as_str() {
            "episode" => add(
                &mut shows,
                "show",
                row.show_id.as_deref(),
                row.parent_title.as_deref(),
                None,
                row,
            ),
            "movie" => add(
                &mut movies,
                "movie",
                row.movie_id.as_deref(),
                Some(row.title.as_str()),
                None,
                row,
            ),
            "track" => {
                add(
                    &mut artists,
                    "artist",
                    row.artist_id.as_deref(),
                    row.parent_title.as_deref(),
                    None,
                    row,
                );
                add(
                    &mut albums,
                    "album",
                    row.album_id.as_deref(),
                    row.subtitle.as_deref(),
                    row.parent_title.as_deref(),
                    row,
                );
            }
            "audiobook" => {
                add(
                    &mut books,
                    "audiobook",
                    row.audiobook_id.as_deref(),
                    Some(row.title.as_str()),
                    row.parent_title.as_deref(),
                    row,
                );
                add(
                    &mut authors,
                    "author",
                    None,
                    row.parent_title.as_deref(),
                    None,
                    row,
                );
            }
            _ => {}
        }
        let mut seen = BTreeSet::new();
        for id in [&row.movie_id, &row.show_id, &row.album_id]
            .into_iter()
            .flatten()
        {
            for genre in genres.get(id).into_iter().flatten() {
                if seen.insert(genre.to_lowercase()) {
                    add(&mut genre_map, "genre", None, Some(genre.as_str()), None, row);
                }
            }
        }

        let device_key = row
            .device_id
            .clone()
            .or_else(|| row.device_name.clone())
            .unwrap_or_else(|| {
                if row.imported {
                    "imported".into()
                } else {
                    "unknown".into()
                }
            });
        let device = devices.entry(device_key).or_insert_with(|| DeviceUsage {
            device_id: row.device_id.clone(),
            ..Default::default()
        });
        device.name = row.device_name.clone().unwrap_or_else(|| {
            if row.imported {
                "Imported from watch progress".into()
            } else {
                "Unknown device".into()
            }
        });
        device.seconds += seconds;
        device.plays += 1;
        device.last_played_at = row.ended_at.clone();

        let user = users
            .entry(row.user_id.clone())
            .or_insert_with(|| UserUsage {
                user_id: row.user_id.clone(),
                name: names
                    .get(&row.user_id)
                    .cloned()
                    .unwrap_or_else(|| "Removed user".into()),
                ..Default::default()
            });
        user.seconds += seconds;
        user.plays += 1;
    }
    totals.items = items.len() as i32;
    totals.active_days = active.len() as i32;

    let mut devices: Vec<DeviceUsage> = devices.into_values().collect();
    devices.sort_by(|a, b| b.seconds.total_cmp(&a.seconds).then(b.plays.cmp(&a.plays)));
    let mut users: Vec<UserUsage> = users.into_values().collect();
    users.sort_by(|a, b| b.seconds.total_cmp(&a.seconds).then(b.plays.cmp(&a.plays)));

    Aggregate {
        totals,
        streak: streak(&active, period.local_date(period.now)),
        days: days.into_values().collect(),
        weeks: weeks.into_values().collect(),
        hours,
        top_shows: ranked(shows, top),
        top_movies: ranked(movies, top),
        top_artists: ranked(artists, top),
        top_albums: ranked(albums, top),
        top_audiobooks: ranked(books, top),
        top_authors: ranked(authors, top),
        top_genres: ranked(genre_map, top),
        devices,
        users,
    }
}
