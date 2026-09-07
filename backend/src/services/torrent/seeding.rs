//! Seeding policy: pure decision functions for when a completed torrent should
//! stop seeding, and whether its downloaded payload may then be deleted.
//!
//! Implements the design rule "source-defined file handling — a source decides
//! when deletion is allowed, e.g. after seeding" (docs/design.md, "Acquisition
//! and Sources"). Everything here is pure so it can be unit tested without a
//! session, a database, or a clock; the enforcement loop in
//! [`super::service`] supplies the facts and performs the side effects.
//!
//! Settings that drive this (seeded in `services/bootstrap_defaults.rs`):
//! - `torrent.seed_ratio_limit` (Float, default `1.0`; `0` = no ratio rule)
//! - `torrent.seed_time_minutes` (Int, default `0` = no time rule)
//! - `torrent.remove_after_import` (Bool, default `false`)

use chrono::{DateTime, Utc};

/// Global seeding rules read from `app_settings`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeedingRules {
    /// `torrent.seed_ratio_limit`: stop seeding once uploaded/total reaches
    /// this ratio. `0` (or negative) disables the ratio rule.
    pub ratio_limit: f64,
    /// `torrent.seed_time_minutes`: stop seeding once the torrent has been
    /// complete for this many minutes. `0` disables the time rule.
    pub time_minutes: i64,
    /// `torrent.remove_after_import`: when true, a torrent whose
    /// `post_process_status` is `completed` and whose seeding rules are
    /// satisfied is removed from the session and its payload deleted. When
    /// false the torrent only stops seeding and the files are kept.
    pub remove_after_import: bool,
}

impl Default for SeedingRules {
    fn default() -> Self {
        Self {
            ratio_limit: 1.0,
            time_minutes: 0,
            remove_after_import: false,
        }
    }
}

impl SeedingRules {
    /// True when neither rule is configured, i.e. seed indefinitely.
    pub fn seeds_indefinitely(&self) -> bool {
        self.ratio_limit <= 0.0 && self.time_minutes <= 0
    }
}

/// Per-torrent facts the rules are evaluated against.
///
/// `minimum_ratio` / `minimum_seed_time_minutes` are the private-tracker floor.
/// The `Torrent` entity does not store them yet, so the enforcement loop passes
/// `None` today; once those columns exist they can be threaded straight in
/// without changing any of the logic below.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SeedingFacts {
    pub uploaded_bytes: i64,
    pub total_bytes: i64,
    /// When the torrent finished downloading (`Torrent.completed_at`). Seed
    /// time is measured from here.
    pub completed_at: Option<DateTime<Utc>>,
    /// `Torrent.post_process_status`: `completed`, `partial`, `unmatched`,
    /// `failed`, or `None` when post-processing has not run yet.
    pub post_process_status: Option<String>,
    /// Private-tracker minimum share ratio, if recorded for this torrent.
    pub minimum_ratio: Option<f64>,
    /// Private-tracker minimum seed time in minutes, if recorded.
    pub minimum_seed_time_minutes: Option<i64>,
}

/// Share ratio (uploaded / total). Zero when the total size is unknown, so a
/// ratio rule is never satisfied by accident on a metadata-less torrent.
pub fn share_ratio(facts: &SeedingFacts) -> f64 {
    if facts.total_bytes <= 0 {
        return 0.0;
    }
    facts.uploaded_bytes.max(0) as f64 / facts.total_bytes as f64
}

/// Minutes elapsed since the torrent completed, or `None` if it never recorded
/// a completion timestamp (so a time rule cannot be evaluated yet).
pub fn seeded_minutes(facts: &SeedingFacts, now: DateTime<Utc>) -> Option<i64> {
    let completed_at = facts.completed_at?;
    Some((now - completed_at).num_minutes().max(0))
}

/// Whether the private tracker's minimums (if any) have been met. Vacuously
/// true when the torrent has no recorded tracker requirements.
pub fn tracker_minimums_met(facts: &SeedingFacts, now: DateTime<Utc>) -> bool {
    let ratio_ok = facts
        .minimum_ratio
        .is_none_or(|min| min <= 0.0 || share_ratio(facts) >= min);
    let time_ok = facts.minimum_seed_time_minutes.is_none_or(|min| {
        min <= 0 || seeded_minutes(facts, now).is_some_and(|seeded| seeded >= min)
    });
    ratio_ok && time_ok
}

/// Whether the user-configured rules are met. With both rules configured the
/// first one reached wins (the usual client behaviour); with neither
/// configured the torrent seeds indefinitely.
pub fn user_rules_met(rules: &SeedingRules, facts: &SeedingFacts, now: DateTime<Utc>) -> bool {
    if rules.seeds_indefinitely() {
        return false;
    }
    let ratio_met = rules.ratio_limit > 0.0 && share_ratio(facts) >= rules.ratio_limit;
    let time_met = rules.time_minutes > 0
        && seeded_minutes(facts, now).is_some_and(|seeded| seeded >= rules.time_minutes);
    ratio_met || time_met
}

/// A torrent may stop seeding once the user's rules are met *and* the private
/// tracker's minimums are met. The tracker floor can only ever delay stopping,
/// never trigger it.
pub fn seeding_satisfied(rules: &SeedingRules, facts: &SeedingFacts, now: DateTime<Utc>) -> bool {
    user_rules_met(rules, facts, now) && tracker_minimums_met(facts, now)
}

/// What the enforcement loop should do with one completed torrent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedingAction {
    /// Leave it alone (rules unmet, or already stopped and files are kept).
    Continue,
    /// Pause the torrent so it stops uploading; keep the payload on disk.
    StopSeeding,
    /// Remove the torrent from the session and delete its downloaded payload.
    /// Library files are hardlinks into the media tree, so deleting the
    /// payload never removes the imported copy.
    RemoveAndDeleteFiles,
}

/// Decide what to do with a torrent that has finished downloading.
///
/// `is_paused` reflects the live session state so an already-stopped torrent
/// produces no repeated pause calls; the decision is otherwise idempotent and
/// safe to re-evaluate on every tick.
pub fn decide_seeding_action(
    rules: &SeedingRules,
    facts: &SeedingFacts,
    now: DateTime<Utc>,
    is_paused: bool,
) -> SeedingAction {
    if !seeding_satisfied(rules, facts, now) {
        return SeedingAction::Continue;
    }
    if rules.remove_after_import && facts.post_process_status.as_deref() == Some("completed") {
        return SeedingAction::RemoveAndDeleteFiles;
    }
    if is_paused {
        return SeedingAction::Continue;
    }
    SeedingAction::StopSeeding
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn minutes_ago(m: i64) -> Option<DateTime<Utc>> {
        Some(now() - chrono::Duration::minutes(m))
    }

    fn facts(uploaded: i64, total: i64, completed_minutes_ago: i64) -> SeedingFacts {
        SeedingFacts {
            uploaded_bytes: uploaded,
            total_bytes: total,
            completed_at: minutes_ago(completed_minutes_ago),
            ..Default::default()
        }
    }

    #[test]
    fn default_rules_stop_at_ratio_one() {
        let rules = SeedingRules::default();
        assert_eq!(rules.ratio_limit, 1.0);
        assert_eq!(rules.time_minutes, 0);
        assert!(!rules.remove_after_import);
        assert!(!seeding_satisfied(&rules, &facts(500, 1000, 5), now()));
        assert!(seeding_satisfied(&rules, &facts(1000, 1000, 5), now()));
    }

    #[test]
    fn zero_ratio_and_zero_time_seed_indefinitely() {
        let rules = SeedingRules {
            ratio_limit: 0.0,
            time_minutes: 0,
            remove_after_import: false,
        };
        assert!(rules.seeds_indefinitely());
        // Even a huge ratio and a long seed time never satisfy "no rules".
        assert!(!seeding_satisfied(
            &rules,
            &facts(100_000, 1000, 100_000),
            now()
        ));
    }

    #[test]
    fn time_rule_alone_satisfies_after_the_window() {
        let rules = SeedingRules {
            ratio_limit: 0.0,
            time_minutes: 60,
            remove_after_import: false,
        };
        assert!(!seeding_satisfied(&rules, &facts(0, 1000, 59), now()));
        assert!(seeding_satisfied(&rules, &facts(0, 1000, 60), now()));
    }

    #[test]
    fn either_rule_reached_satisfies_when_both_configured() {
        let rules = SeedingRules {
            ratio_limit: 2.0,
            time_minutes: 60,
            remove_after_import: false,
        };
        // Ratio reached first.
        assert!(seeding_satisfied(&rules, &facts(2000, 1000, 1), now()));
        // Time reached first.
        assert!(seeding_satisfied(&rules, &facts(0, 1000, 90), now()));
        // Neither reached.
        assert!(!seeding_satisfied(&rules, &facts(500, 1000, 10), now()));
    }

    #[test]
    fn missing_completed_at_never_satisfies_a_time_rule() {
        let rules = SeedingRules {
            ratio_limit: 0.0,
            time_minutes: 1,
            remove_after_import: false,
        };
        let facts = SeedingFacts {
            uploaded_bytes: 0,
            total_bytes: 1000,
            completed_at: None,
            ..Default::default()
        };
        assert_eq!(seeded_minutes(&facts, now()), None);
        assert!(!seeding_satisfied(&rules, &facts, now()));
    }

    #[test]
    fn unknown_total_size_never_satisfies_a_ratio_rule() {
        let rules = SeedingRules::default();
        let facts = SeedingFacts {
            uploaded_bytes: 10_000,
            total_bytes: 0,
            completed_at: minutes_ago(5),
            ..Default::default()
        };
        assert_eq!(share_ratio(&facts), 0.0);
        assert!(!seeding_satisfied(&rules, &facts, now()));
    }

    #[test]
    fn private_tracker_minimum_ratio_delays_stopping() {
        let rules = SeedingRules {
            ratio_limit: 0.0,
            time_minutes: 10,
            remove_after_import: false,
        };
        let mut facts = facts(500, 1000, 30);
        facts.minimum_ratio = Some(1.0);
        // User time rule is met, but the tracker still wants ratio 1.0.
        assert!(user_rules_met(&rules, &facts, now()));
        assert!(!tracker_minimums_met(&facts, now()));
        assert!(!seeding_satisfied(&rules, &facts, now()));

        facts.uploaded_bytes = 1000;
        assert!(seeding_satisfied(&rules, &facts, now()));
    }

    #[test]
    fn private_tracker_minimum_seed_time_delays_stopping() {
        let rules = SeedingRules::default();
        let mut facts = facts(2000, 1000, 30);
        facts.minimum_seed_time_minutes = Some(120);
        assert!(user_rules_met(&rules, &facts, now()));
        assert!(!seeding_satisfied(&rules, &facts, now()));

        facts.completed_at = minutes_ago(120);
        assert!(seeding_satisfied(&rules, &facts, now()));
    }

    #[test]
    fn tracker_minimums_alone_never_trigger_stopping() {
        // No user rules configured: tracker minimums being met is not a reason
        // to stop seeding.
        let rules = SeedingRules {
            ratio_limit: 0.0,
            time_minutes: 0,
            remove_after_import: false,
        };
        let mut facts = facts(10_000, 1000, 10_000);
        facts.minimum_ratio = Some(1.0);
        facts.minimum_seed_time_minutes = Some(10);
        assert!(tracker_minimums_met(&facts, now()));
        assert!(!seeding_satisfied(&rules, &facts, now()));
    }

    #[test]
    fn action_is_continue_while_rules_are_unmet() {
        let rules = SeedingRules::default();
        assert_eq!(
            decide_seeding_action(&rules, &facts(100, 1000, 1), now(), false),
            SeedingAction::Continue
        );
    }

    #[test]
    fn action_stops_seeding_once_and_then_goes_quiet() {
        let rules = SeedingRules::default();
        let facts = facts(1000, 1000, 1);
        assert_eq!(
            decide_seeding_action(&rules, &facts, now(), false),
            SeedingAction::StopSeeding
        );
        // Already paused: nothing further to do, so the loop stays idempotent.
        assert_eq!(
            decide_seeding_action(&rules, &facts, now(), true),
            SeedingAction::Continue
        );
    }

    #[test]
    fn remove_after_import_requires_a_completed_import() {
        let rules = SeedingRules {
            ratio_limit: 1.0,
            time_minutes: 0,
            remove_after_import: true,
        };
        let mut facts = facts(1000, 1000, 1);

        // Import not run yet: stop seeding but keep the payload.
        assert_eq!(
            decide_seeding_action(&rules, &facts, now(), false),
            SeedingAction::StopSeeding
        );

        // Import ran but matched nothing: still keep the payload.
        facts.post_process_status = Some("unmatched".to_string());
        assert_eq!(
            decide_seeding_action(&rules, &facts, now(), false),
            SeedingAction::StopSeeding
        );

        // Import failed: keep the payload so it can be retried.
        facts.post_process_status = Some("failed".to_string());
        assert_eq!(
            decide_seeding_action(&rules, &facts, now(), false),
            SeedingAction::StopSeeding
        );

        // Import completed: safe to remove and delete.
        facts.post_process_status = Some("completed".to_string());
        assert_eq!(
            decide_seeding_action(&rules, &facts, now(), false),
            SeedingAction::RemoveAndDeleteFiles
        );
        // Still removable even if it was already paused.
        assert_eq!(
            decide_seeding_action(&rules, &facts, now(), true),
            SeedingAction::RemoveAndDeleteFiles
        );
    }

    #[test]
    fn remove_after_import_still_waits_for_the_seeding_rules() {
        let rules = SeedingRules {
            ratio_limit: 2.0,
            time_minutes: 0,
            remove_after_import: true,
        };
        let mut facts = facts(500, 1000, 1);
        facts.post_process_status = Some("completed".to_string());
        assert_eq!(
            decide_seeding_action(&rules, &facts, now(), false),
            SeedingAction::Continue
        );
    }
}
