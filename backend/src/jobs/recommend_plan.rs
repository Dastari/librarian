//! Coverage plans: the smallest set of releases that covers everything missing of a show (Q101).
//!
//! Pure code. The recommender gathers releases (feed catalogue, a complete-series search, season
//! searches), says what each one covers ([`claim_of`], or its inspected files), and
//! [`plan_cover`] picks the releases:
//!
//! - **Quality first.** Releases are grouped by the profile's resolution tier. The best tier is
//!   planned first; a lower tier only fills what the better ones leave uncovered. A plan never
//!   trades quality for fewer downloads.
//! - **Fewer releases.** Within a tier, the release that covers the most of what is still
//!   uncovered is taken next (a complete-series pack, then season packs, then episodes), so packs
//!   and single episodes mix naturally: a pack for each season that has one, episodes for the gaps.
//! - **Healthier, then smaller.** Ties go to the better-seeded release (by bands, so 99 and 100
//!   seeders are the same), then one that bundles subtitles (Q114), then the smaller one, then the
//!   profile's own order.
//! - A release made redundant by later picks is dropped, and each release in a plan is credited
//!   only with what it adds, so the counts add up.

use std::collections::BTreeSet;

use crate::services::quality::scoring::ParsedRelease;

/// One missing episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingEpisode {
    pub id: String,
    pub season: i32,
    pub episode: i32,
}

/// What a release's name says it covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claim {
    /// Single episodes (`S01E05`, `S01E05E06`).
    Episodes(i32, Vec<i32>),
    /// One season or a range of seasons (`S02`, `S01-S03`).
    Seasons(i32, i32),
    /// A complete series that names no season.
    Everything,
    /// A pack that says only that it is complete (`Show Complete DVD Rip`): counts only for what
    /// its inspected files show.
    Vague,
}

impl Claim {
    /// The scope a recommendation row gets for a release of this claim.
    pub fn scope(&self) -> &'static str {
        match self {
            Self::Episodes(..) => "episode",
            Self::Seasons(first, last) if first == last => "season_pack",
            _ => "series_pack",
        }
    }

    pub fn is_pack(&self) -> bool {
        !matches!(self, Self::Episodes(..))
    }
}

/// Words that make a release without numbering a pack of the whole show.
fn says_complete(title: &str) -> bool {
    crate::services::quality::title_match::tokens(title)
        .iter()
        .any(|word| {
            matches!(
                word.as_str(),
                "complete" | "collection" | "boxset" | "anthology"
            )
        })
}

/// What a release's name claims to cover. `None` when it names no episode or season and does not
/// say it is complete: nothing can be planned on it.
pub fn claim_of(parsed: &ParsedRelease, title: &str) -> Option<Claim> {
    if let (Some(season), false) = (parsed.season, parsed.episodes.is_empty()) {
        return Some(Claim::Episodes(season, parsed.episodes.clone()));
    }
    if let Some(first) = parsed.season {
        let last = parsed.season_end.unwrap_or(first).max(first);
        return Some(Claim::Seasons(first, last));
    }
    if parsed.is_season_pack {
        return Some(Claim::Everything);
    }
    says_complete(title).then_some(Claim::Vague)
}

/// The indexes of `missing` a claim covers by name.
pub fn claimed_cover(claim: &Claim, missing: &[MissingEpisode]) -> Vec<usize> {
    missing
        .iter()
        .enumerate()
        .filter(|(_, item)| match claim {
            Claim::Episodes(season, episodes) => {
                item.season == *season && episodes.contains(&item.episode)
            }
            Claim::Seasons(first, last) => (*first..=*last).contains(&item.season),
            Claim::Everything => true,
            Claim::Vague => false,
        })
        .map(|(index, _)| index)
        .collect()
}

/// A release that could be part of a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanOption {
    /// Indexes of the missing items it covers.
    pub covers: Vec<usize>,
    /// The profile's resolution tier: lower is better.
    pub tier: u32,
    pub seeders: Option<i32>,
    pub size: Option<i64>,
    /// Position in the profile's ranking: the last tiebreak.
    pub rank: usize,
    /// Its files bundle subtitles for what it covers (Q114): a small tiebreak after health.
    pub subtitles: bool,
}

/// A planned release: which option, and what it adds to the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub option: usize,
    pub adds: Vec<usize>,
}

/// The releases to take, in the order they were chosen, and what stays uncovered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoverPlan {
    pub releases: Vec<Planned>,
    pub uncovered: Vec<usize>,
}

impl CoverPlan {
    pub fn covered(&self) -> usize {
        self.releases.iter().map(|planned| planned.adds.len()).sum()
    }
}

/// Seeders as health bands: unknown sorts with the middle band.
fn health_band(seeders: Option<i32>) -> u8 {
    match seeders {
        Some(0) => 3,
        Some(1..=4) => 2,
        None | Some(5..=19) => 1,
        Some(_) => 0,
    }
}

/// The smallest set of `options` covering `missing` items, quality tier first (see the module
/// documentation).
pub fn plan_cover(missing: usize, options: &[PlanOption]) -> CoverPlan {
    let mut uncovered: BTreeSet<usize> = (0..missing).collect();
    let mut chosen: Vec<usize> = Vec::new();
    let mut tiers: Vec<u32> = options.iter().map(|option| option.tier).collect();
    tiers.sort_unstable();
    tiers.dedup();
    for tier in tiers {
        loop {
            let best = options
                .iter()
                .enumerate()
                .filter(|(index, option)| option.tier == tier && !chosen.contains(index))
                .map(|(index, option)| {
                    let gain = option
                        .covers
                        .iter()
                        .filter(|item| uncovered.contains(item))
                        .count();
                    (index, option, gain)
                })
                .filter(|(_, _, gain)| *gain > 0)
                .min_by_key(|(_, option, gain)| {
                    (
                        std::cmp::Reverse(*gain),
                        health_band(option.seeders),
                        !option.subtitles,
                        option.size.unwrap_or(i64::MAX),
                        option.rank,
                    )
                });
            let Some((index, option, _)) = best else {
                break;
            };
            for item in &option.covers {
                uncovered.remove(item);
            }
            chosen.push(index);
            if uncovered.is_empty() {
                break;
            }
        }
        if uncovered.is_empty() {
            break;
        }
    }

    // Drop a release whose every item the others of the same or a better tier also cover, latest
    // picks first. A better release is never dropped for a worse one that happens to cover it.
    let mut position = chosen.len();
    while position > 0 {
        position -= 1;
        let tier = options[chosen[position]].tier;
        let others: BTreeSet<usize> = chosen
            .iter()
            .enumerate()
            .filter(|(at, index)| *at != position && options[**index].tier <= tier)
            .flat_map(|(_, index)| options[*index].covers.iter().copied())
            .collect();
        let redundant = options[chosen[position]]
            .covers
            .iter()
            .filter(|item| **item < missing)
            .all(|item| others.contains(item));
        if redundant {
            chosen.remove(position);
        }
    }

    // Credit each release with what it adds, in order.
    let mut seen: BTreeSet<usize> = BTreeSet::new();
    let releases = chosen
        .into_iter()
        .map(|option| {
            let adds: Vec<usize> = options[option]
                .covers
                .iter()
                .copied()
                .filter(|item| *item < missing && seen.insert(*item))
                .collect();
            Planned { option, adds }
        })
        .filter(|planned| !planned.adds.is_empty())
        .collect();
    CoverPlan {
        releases,
        uncovered: uncovered.into_iter().collect(),
    }
}

/// What is still missing after a plan, as a person would say it: `S01E19–E22, all of S03`.
pub fn describe_items(missing: &[MissingEpisode], items: &[usize]) -> String {
    let mut by_season: std::collections::BTreeMap<i32, Vec<i32>> = Default::default();
    for index in items {
        if let Some(item) = missing.get(*index) {
            by_season.entry(item.season).or_default().push(item.episode);
        }
    }
    let mut parts: Vec<String> = Vec::new();
    for (season, mut episodes) in by_season {
        episodes.sort_unstable();
        let whole = missing.iter().filter(|item| item.season == season).count();
        if episodes.len() == whole && whole > 2 {
            parts.push(format!("all {whole} of season {season}"));
            continue;
        }
        let mut iter = episodes.into_iter().peekable();
        while let Some(first) = iter.next() {
            let mut last = first;
            while iter.peek() == Some(&(last + 1)) {
                last = iter.next().unwrap_or(last);
            }
            parts.push(if last == first {
                format!("S{season:02}E{first:02}")
            } else {
                format!("S{season:02}E{first:02}–E{last:02}")
            });
        }
    }
    if parts.len() > 6 {
        let more = parts.len() - 5;
        parts.truncate(5);
        parts.push(format!("and {more} more"));
    }
    parts.join(", ")
}

/// The seasons a set of missing items falls in, ascending.
pub fn seasons_of(missing: &[MissingEpisode], items: &[usize]) -> Vec<i32> {
    let seasons: BTreeSet<i32> = items
        .iter()
        .filter_map(|index| missing.get(*index).map(|item| item.season))
        .collect();
    seasons.into_iter().collect()
}

#[cfg(test)]
#[path = "recommend_plan_tests.rs"]
mod tests;
