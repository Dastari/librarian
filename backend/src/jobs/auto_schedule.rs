//! The scheduler's decisions, kept pure so they can be tested with a fixed clock.
//!
//! - **When** a target may be searched: an aired episode waits out a grace period for releases
//!   to appear, is then retried on a quick schedule for a few days, and afterwards on the
//!   ordinary backoff ([`classify_release`], [`backoff_for`], [`backoff_elapsed`]).
//! - **How many** automatic grabs may run at once ([`grab_budget`]).
//! - **What** a release must be to be taken for a target ([`Expectation`]): the right episode,
//!   the right season, the right movie year. A search can return neighbours of what was asked
//!   for, and a feed returns everything, so a title that merely resembles the target is not enough.

use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};

use super::auto_settings::AutoDownloadSettings;
use crate::services::quality::scoring::ParsedRelease;

/// `target key -> last searched at (RFC3339)`.
pub type SearchState = HashMap<String, String>;

/// Where a dated item stands relative to its release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timing {
    /// Not yet searchable: it has not aired, or its grace period has not passed.
    NotYet { searchable_at: DateTime<Utc> },
    /// Released within the quick-retry window.
    Fresh,
    /// Released earlier (or undated back-catalogue): the ordinary, slower schedule.
    Back,
}

/// The moment a date (and optional time stamp) refers to. An RFC 3339 stamp is exact; a bare
/// `YYYY-MM-DD` date is taken as the start of that day (UTC), because it carries no time or zone.
pub fn release_instant(date: Option<&str>, stamp: Option<&str>) -> Option<DateTime<Utc>> {
    for value in [stamp, date].into_iter().flatten() {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
            return Some(parsed.with_timezone(&Utc));
        }
        if let Ok(parsed) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
            return parsed.and_hms_opt(0, 0, 0).map(|naive| naive.and_utc());
        }
    }
    None
}

/// Classify an item released at `released_at` (undated items are back-catalogue and always
/// searchable; they are often entries with no air date at all).
pub fn classify_release(
    released_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    grace: Duration,
    window: Duration,
) -> Timing {
    let Some(released_at) = released_at else {
        return Timing::Back;
    };
    let searchable_at = released_at + grace;
    if now < searchable_at {
        return Timing::NotYet { searchable_at };
    }
    if now - released_at <= window {
        Timing::Fresh
    } else {
        Timing::Back
    }
}

/// [`classify_release`] for an episode, using the configured grace period and window.
pub fn classify_episode(
    air_date: Option<&str>,
    air_stamp: Option<&str>,
    now: DateTime<Utc>,
    settings: &AutoDownloadSettings,
) -> Timing {
    classify_release(
        release_instant(air_date, air_stamp),
        now,
        Duration::minutes(settings.air_grace_minutes),
        Duration::days(settings.air_window_days),
    )
}

/// How long a searched target waits before it is searched again.
pub fn backoff_for(timing: Timing, settings: &AutoDownloadSettings) -> Duration {
    match timing {
        Timing::Fresh => Duration::minutes(settings.air_retry_minutes),
        _ => Duration::minutes(settings.search_backoff_minutes),
    }
}

/// Whether a target may be searched again: never searched, or the backoff has elapsed.
pub fn backoff_elapsed(
    state: &SearchState,
    key: &str,
    now: DateTime<Utc>,
    backoff: Duration,
) -> bool {
    next_search_at(state, key, backoff).is_none_or(|next| now >= next)
}

/// When a target may next be searched, if it has been searched before.
pub fn next_search_at(state: &SearchState, key: &str, backoff: Duration) -> Option<DateTime<Utc>> {
    let last = DateTime::parse_from_rfc3339(state.get(key)?).ok()?;
    Some(last.with_timezone(&Utc) + backoff)
}

/// How many more automatic grabs may start, given how many are still downloading.
pub fn grab_budget(in_flight: usize, cap: usize) -> usize {
    cap.saturating_sub(in_flight)
}

/// What a release has to be to count as the wanted item. Built from the target, checked against
/// the parse of the release title.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Expectation {
    /// No identity to verify (music, audiobooks: matched by the title ranking alone).
    #[default]
    Unchecked,
    Episode {
        season: i32,
        episode: i32,
    },
    SeasonPack {
        season: i32,
    },
    Movie {
        year: Option<i32>,
    },
}

impl Expectation {
    /// Why `parsed` is not the wanted item, or `None` when it is.
    ///
    /// `strict` is for feeds, which carry every release of every show: a title that does not say
    /// which episode or year it is for cannot be accepted. A search that was already narrowed by
    /// season and episode may keep a title that does not say (`strict = false`), but never one
    /// that names a different episode.
    pub fn mismatch(&self, parsed: &ParsedRelease, strict: bool) -> Option<String> {
        match self {
            Self::Unchecked => None,
            Self::Episode { season, episode } => {
                if parsed.is_season_pack {
                    return Some(format!(
                        "a season pack, wanted S{season:02}E{episode:02} on its own"
                    ));
                }
                match (parsed.season, parsed.episodes.is_empty()) {
                    (Some(found), false) => {
                        if found == *season && parsed.episodes.contains(episode) {
                            None
                        } else {
                            Some(format!(
                                "for S{found:02}E{:02}, wanted S{season:02}E{episode:02}",
                                parsed.episodes.first().copied().unwrap_or_default()
                            ))
                        }
                    }
                    _ if strict => Some(format!(
                        "does not say which episode it is, wanted S{season:02}E{episode:02}"
                    )),
                    _ => None,
                }
            }
            Self::SeasonPack { season } => {
                if !parsed.episodes.is_empty() && !parsed.is_season_pack {
                    return Some(format!("a single episode, wanted the S{season:02} pack"));
                }
                match parsed.season {
                    Some(first) => {
                        let last = parsed.season_end.unwrap_or(first);
                        if (first..=last).contains(season) {
                            None
                        } else {
                            Some(format!("for season {first}, wanted season {season}"))
                        }
                    }
                    None if strict => Some(format!(
                        "does not say which season it is, wanted season {season}"
                    )),
                    None => None,
                }
            }
            Self::Movie { year } => {
                if parsed.season.is_some() || !parsed.episodes.is_empty() {
                    return Some("an episode of a show, wanted a movie".to_string());
                }
                match (year, parsed.year) {
                    (Some(wanted), Some(found)) if (wanted - found).abs() > 1 => {
                        Some(format!("from {found}, wanted {wanted}"))
                    }
                    (Some(wanted), None) if strict => {
                        Some(format!("does not say its year, wanted {wanted}"))
                    }
                    _ => None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::quality::scoring::parse_release;

    fn at(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn settings() -> AutoDownloadSettings {
        AutoDownloadSettings::default()
    }

    #[test]
    fn an_episode_waits_for_its_air_time_plus_the_grace_period() {
        let stamp = "2026-11-08T02:00:00+00:00";
        // Airing in an hour: not yet.
        let before = at("2026-11-08T01:00:00Z");
        assert!(matches!(
            classify_episode(Some("2026-11-08"), Some(stamp), before, &settings()),
            Timing::NotYet { .. }
        ));
        // Aired, but inside the 60 minute grace period: still not searchable, and says when.
        let during = at("2026-11-08T02:30:00Z");
        assert_eq!(
            classify_episode(Some("2026-11-08"), Some(stamp), during, &settings()),
            Timing::NotYet {
                searchable_at: at("2026-11-08T03:00:00Z")
            }
        );
        // Grace passed: searchable, on the quick schedule.
        let after = at("2026-11-08T03:00:00Z");
        assert_eq!(
            classify_episode(Some("2026-11-08"), Some(stamp), after, &settings()),
            Timing::Fresh
        );
    }

    #[test]
    fn a_week_after_airing_an_episode_moves_to_the_ordinary_schedule() {
        let stamp = "2026-11-01T02:00:00+00:00";
        let inside = at("2026-11-08T01:59:00Z");
        assert_eq!(
            classify_episode(None, Some(stamp), inside, &settings()),
            Timing::Fresh
        );
        let outside = at("2026-11-08T02:01:00Z");
        assert_eq!(
            classify_episode(None, Some(stamp), outside, &settings()),
            Timing::Back
        );
    }

    #[test]
    fn a_date_without_a_time_means_the_start_of_that_day() {
        let now = at("2026-11-08T00:30:00Z");
        // 00:00 + 60 minutes grace has not passed at 00:30.
        assert!(matches!(
            classify_episode(Some("2026-11-08"), None, now, &settings()),
            Timing::NotYet { .. }
        ));
        let later = at("2026-11-08T01:00:00Z");
        assert_eq!(
            classify_episode(Some("2026-11-08"), None, later, &settings()),
            Timing::Fresh
        );
    }

    #[test]
    fn an_undated_item_is_back_catalogue_and_always_searchable() {
        assert_eq!(
            classify_episode(None, None, at("2026-11-08T00:00:00Z"), &settings()),
            Timing::Back
        );
        assert_eq!(
            classify_episode(
                Some("  "),
                Some(""),
                at("2026-11-08T00:00:00Z"),
                &settings()
            ),
            Timing::Back
        );
    }

    #[test]
    fn fresh_episodes_retry_quickly_and_old_ones_slowly() {
        let settings = settings();
        assert_eq!(backoff_for(Timing::Fresh, &settings), Duration::minutes(60));
        assert_eq!(backoff_for(Timing::Back, &settings), Duration::minutes(360));
    }

    #[test]
    fn the_backoff_blocks_a_repeat_search_until_it_has_elapsed() {
        let now = at("2026-11-08T12:00:00Z");
        let mut state = SearchState::new();
        let six_hours = Duration::hours(6);
        assert!(backoff_elapsed(&state, "movie:1", now, six_hours));

        state.insert("movie:1".into(), (now - Duration::hours(1)).to_rfc3339());
        assert!(!backoff_elapsed(&state, "movie:1", now, six_hours));
        assert_eq!(
            next_search_at(&state, "movie:1", six_hours),
            Some(now + Duration::hours(5))
        );

        state.insert("movie:1".into(), (now - Duration::hours(6)).to_rfc3339());
        assert!(backoff_elapsed(&state, "movie:1", now, six_hours));

        // A fresh episode's shorter backoff lets it through sooner than the same age would.
        state.insert("ep:1".into(), (now - Duration::minutes(90)).to_rfc3339());
        assert!(backoff_elapsed(&state, "ep:1", now, Duration::minutes(60)));
        assert!(!backoff_elapsed(&state, "ep:1", now, six_hours));

        // Garbage in the stored state never blocks a search.
        state.insert("movie:2".into(), "not-a-timestamp".into());
        assert!(backoff_elapsed(&state, "movie:2", now, six_hours));
    }

    #[test]
    fn the_cap_leaves_room_only_for_what_is_not_already_downloading() {
        assert_eq!(grab_budget(0, 5), 5);
        assert_eq!(grab_budget(3, 5), 2);
        assert_eq!(grab_budget(5, 5), 0);
        assert_eq!(grab_budget(9, 5), 0);
    }

    #[test]
    fn an_episode_must_be_the_wanted_one() {
        let wanted = Expectation::Episode {
            season: 2,
            episode: 5,
        };
        assert_eq!(
            wanted.mismatch(&parse_release("Show.Name.S02E05.1080p.WEB.h264-GRP"), true),
            None
        );
        assert!(
            wanted
                .mismatch(&parse_release("Show.Name.S02E06.1080p.WEB.h264-GRP"), false)
                .is_some(),
            "the next episode is not this one"
        );
        assert!(
            wanted
                .mismatch(&parse_release("Show.Name.S03E05.1080p.WEB.h264-GRP"), false)
                .is_some()
        );
        assert!(
            wanted
                .mismatch(&parse_release("Show.Name.S02.1080p.WEB.h264-GRP"), false)
                .is_some(),
            "a pack is not a single episode"
        );
        // A double episode still covers it.
        assert_eq!(
            wanted.mismatch(
                &parse_release("Show.Name.S02E04E05.720p.HDTV.x264-GRP"),
                true
            ),
            None
        );
    }

    #[test]
    fn an_unlabelled_title_passes_a_narrowed_search_but_never_a_feed() {
        let wanted = Expectation::Episode {
            season: 1,
            episode: 1,
        };
        let unlabelled = parse_release("Show Name 1080p WEB-DL");
        assert_eq!(wanted.mismatch(&unlabelled, false), None);
        assert!(wanted.mismatch(&unlabelled, true).is_some());
    }

    #[test]
    fn a_season_pack_must_cover_the_wanted_season() {
        let wanted = Expectation::SeasonPack { season: 2 };
        assert_eq!(
            wanted.mismatch(
                &parse_release("Show.Name.S02.COMPLETE.1080p.WEB-DL-GRP"),
                true
            ),
            None
        );
        assert_eq!(
            wanted.mismatch(&parse_release("Show.Name.S01-S03.1080p.BluRay-GRP"), true),
            None
        );
        assert!(
            wanted
                .mismatch(&parse_release("Show.Name.S01.1080p.WEB-DL-GRP"), true)
                .is_some()
        );
        assert!(
            wanted
                .mismatch(&parse_release("Show.Name.S02E03.1080p.WEB-DL-GRP"), true)
                .is_some(),
            "one episode is not the pack"
        );
    }

    #[test]
    fn a_movie_must_be_the_right_year_and_not_an_episode() {
        let wanted = Expectation::Movie { year: Some(2019) };
        assert_eq!(
            wanted.mismatch(
                &parse_release("Some.Movie.2019.1080p.WEB-DL.x264-GRP"),
                true
            ),
            None
        );
        assert!(
            wanted
                .mismatch(
                    &parse_release("Some.Movie.1994.1080p.BluRay.x264-GRP"),
                    false
                )
                .is_some(),
            "a remake from another year is another movie"
        );
        assert!(
            wanted
                .mismatch(&parse_release("Some.Movie.S01E01.1080p.WEB-DL-GRP"), false)
                .is_some()
        );
        // A year off by one is a festival-versus-release difference, not a different film.
        assert_eq!(
            wanted.mismatch(
                &parse_release("Some.Movie.2020.1080p.WEB-DL.x264-GRP"),
                true
            ),
            None
        );
        let undated = parse_release("Some.Movie.1080p.WEB-DL.x264-GRP");
        assert_eq!(wanted.mismatch(&undated, false), None);
        assert!(wanted.mismatch(&undated, true).is_some());
    }

    #[test]
    fn unchecked_targets_accept_anything() {
        assert_eq!(
            Expectation::Unchecked.mismatch(&parse_release("Anything at all"), true),
            None
        );
    }
}
