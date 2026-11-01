//! Content rules for generated writes, applied by the write input transform to
//! every create and update regardless of the path (generated mutation, internal
//! service write, seed). They hold for the stored values, so internal writers
//! that already produce valid rows are unaffected.
//!
//! Errors surface to the caller as the mutation's `error`; nothing here may
//! rely on a deferred hook, whose failures are only logged. Cast has its own
//! module (`services::cast::write_rules`) for the same reason.

use async_graphql::{Error, ErrorExtensions, Result};

use crate::graphql::entities::{
    CreateUserInput, LibraryScanIssue, MediaFile, UpdateLibraryScanIssueInput,
    UpdateMediaFileInput, UpdateUserInput,
};
use crate::services::graphql::auth::{AuthUser, Role};

/// Longest resolution note kept on a scan issue.
pub const MAX_RESOLUTION_CHARS: usize = 1_000;

/// `match_type` of a link made by an explicit user action (design.md Q9b).
/// Automatic matching never overwrites it.
pub const MANUAL_MATCH_TYPE: &str = "manual";

fn invalid(message: impl Into<String>) -> Error {
    Error::new(message.into()).extend_with(|_, extensions| extensions.set("code", "BAD_USER_INPUT"))
}

fn bounded(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

/// Resolving a scan issue.
///
/// * A write that sets `resolvedAt` is a resolve request. The server stamps the
///   time, whatever the caller sent.
/// * A resolve request needs a non-blank note, trimmed and bounded.
/// * Resolving an issue that is already resolved changes nothing: the original
///   time and note stay. (A bulk update is checked against its first row, so a
///   bulk resolve should filter on `resolvedAt: { isNull: true }`.)
///
/// A write that only edits the note is trimmed and bounded the same way.
pub fn prepare_scan_issue_update(
    existing: Option<&LibraryScanIssue>,
    input: &mut UpdateLibraryScanIssueInput,
) -> Result<()> {
    let resolving = matches!(input.resolved_at, Some(Some(_)));
    if resolving && existing.is_some_and(|row| row.resolved_at.is_some()) {
        input.resolved_at = None;
        input.resolution = None;
        return Ok(());
    }
    if let Some(Some(note)) = input.resolution.as_ref() {
        let note = bounded(note.trim(), MAX_RESOLUTION_CHARS);
        if resolving && note.is_empty() {
            return Err(invalid("A resolution note is required"));
        }
        input.resolution = Some((!note.is_empty()).then_some(note));
    }
    if resolving {
        if !matches!(input.resolution, Some(Some(_))) {
            return Err(invalid("A resolution note is required"));
        }
        input.resolved_at = Some(Some(chrono::Utc::now().to_rfc3339()));
    }
    Ok(())
}

/// A user's role must be one `Role::parse` accepts, stored in its canonical
/// lower-case form (`admin` or `member`): authorization parses it on every
/// request and a few checks compare it as a string.
fn normalized_role(role: &str) -> Result<String> {
    Role::parse(role)
        .map(|role| role.as_str().to_string())
        .ok_or_else(|| invalid("Unsupported role; expected admin or member"))
}

pub fn prepare_user_create(input: &mut CreateUserInput) -> Result<()> {
    input.role = normalized_role(&input.role)?;
    Ok(())
}

pub fn prepare_user_update(input: &mut UpdateUserInput) -> Result<()> {
    if let Some(role) = input.role.as_deref() {
        input.role = Some(normalized_role(role)?);
    }
    Ok(())
}

fn sets_link(input: &UpdateMediaFileInput) -> bool {
    [
        &input.movie_id,
        &input.episode_id,
        &input.track_id,
        &input.chapter_id,
    ]
    .into_iter()
    .any(|link| matches!(link, Some(Some(_))))
}

/// The file's link once `input` is applied, if it still has one.
fn links_after<'a>(existing: &'a MediaFile, input: &'a UpdateMediaFileInput) -> Option<&'a str> {
    fn pick<'a>(set: &'a Option<Option<String>>, current: &'a Option<String>) -> Option<&'a str> {
        match set {
            Some(value) => value.as_deref(),
            None => current.as_deref(),
        }
    }
    pick(&input.movie_id, &existing.movie_id)
        .or(pick(&input.episode_id, &existing.episode_id))
        .or(pick(&input.track_id, &existing.track_id))
        .or(pick(&input.chapter_id, &existing.chapter_id))
}

fn touches_link(input: &UpdateMediaFileInput) -> bool {
    [
        &input.movie_id,
        &input.episode_id,
        &input.track_id,
        &input.chapter_id,
    ]
    .into_iter()
    .any(Option::is_some)
}

/// The manual-match guard reads `matchType`, `matchedByUserId` and
/// `matchConfirmedAt` (see `LibraryScanService::should_block_automatic_match`),
/// which `matchMediaFile` and the importers set explicitly. A link written
/// through the generated `updateMediaFile` by a signed-in caller is an explicit
/// user decision too, so it is recorded as one unless the caller said otherwise;
/// without this a hand-made link carries no `matchType` and the next automatic
/// match is free to replace it. Clearing every link clears the record with it,
/// as `unmatchMediaFile` does.
///
/// Only writes with a signed-in caller are touched: the importers and the
/// inverse-link hook set their own match fields.
pub fn prepare_media_file_update(
    actor: Option<&AuthUser>,
    existing: Option<&MediaFile>,
    input: &mut UpdateMediaFileInput,
    now: &str,
) {
    let Some(actor) = actor else {
        return;
    };
    let match_fields_supplied = input.match_type.is_some()
        || input.matched_by_user_id.is_some()
        || input.match_confirmed_at.is_some();
    if match_fields_supplied {
        return;
    }
    if sets_link(input) {
        input.match_type = Some(Some(MANUAL_MATCH_TYPE.to_string()));
        input.matched_by_user_id = Some(Some(actor.user_id.clone()));
        input.match_confirmed_at = Some(Some(now.to_string()));
    } else if touches_link(input)
        && let Some(existing) = existing
        && existing.match_type.is_some()
        && links_after(existing, input).is_none()
    {
        input.match_type = Some(None);
        input.matched_by_user_id = Some(None);
        input.match_confirmed_at = Some(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(resolved_at: Option<&str>) -> LibraryScanIssue {
        serde_json::from_value(serde_json::json!({
            "Id": "issue", "UserId": "u", "ScanRunId": "r", "LibraryId": "l", "MediaFileId": null,
            "Stage": "ANALYSIS", "IssueCode": "X", "Severity": "WARN", "Message": "m",
            "Remediation": null, "DetailsJson": null, "OccurrenceCount": 1, "ReadAt": null,
            "ResolvedAt": resolved_at, "Resolution": resolved_at.map(|_| "original"),
            "CreatedAt": "2026-01-01T00:00:00Z", "UpdatedAt": "2026-01-01T00:00:00Z",
        }))
        .expect("issue fixture")
    }

    fn resolve(note: Option<&str>) -> UpdateLibraryScanIssueInput {
        UpdateLibraryScanIssueInput {
            resolved_at: Some(Some("client-supplied".to_string())),
            resolution: note.map(|note| Some(note.to_string())),
            ..Default::default()
        }
    }

    #[test]
    fn resolving_trims_bounds_and_stamps_the_note() {
        let mut input = resolve(Some(&format!("  {}  ", "x".repeat(2_000))));
        prepare_scan_issue_update(Some(&issue(None)), &mut input).unwrap();
        let note = input.resolution.unwrap().unwrap();
        assert_eq!(note.chars().count(), MAX_RESOLUTION_CHARS);
        let stamped = input.resolved_at.unwrap().unwrap();
        assert_ne!(stamped, "client-supplied");
        chrono::DateTime::parse_from_rfc3339(&stamped).expect("server timestamp");
    }

    #[test]
    fn resolving_needs_a_note() {
        for note in [None, Some("   ")] {
            let mut input = resolve(note);
            assert!(prepare_scan_issue_update(Some(&issue(None)), &mut input).is_err());
        }
    }

    #[test]
    fn resolving_a_resolved_issue_changes_nothing() {
        let mut input = resolve(Some("second note"));
        prepare_scan_issue_update(Some(&issue(Some("2026-01-02T00:00:00Z"))), &mut input).unwrap();
        assert!(input.resolved_at.is_none() && input.resolution.is_none());
    }

    #[test]
    fn other_scan_issue_updates_are_untouched() {
        let mut input = UpdateLibraryScanIssueInput {
            read_at: Some(Some("2026-01-03T00:00:00Z".to_string())),
            ..Default::default()
        };
        prepare_scan_issue_update(Some(&issue(None)), &mut input).unwrap();
        assert!(input.resolved_at.is_none() && input.resolution.is_none());
    }

    #[test]
    fn roles_are_checked_and_canonical() {
        let mut input = UpdateUserInput {
            role: Some(" Admin ".to_string()),
            ..Default::default()
        };
        prepare_user_update(&mut input).unwrap();
        assert_eq!(input.role.as_deref(), Some("admin"));
        let mut input = UpdateUserInput {
            role: Some("superuser".to_string()),
            ..Default::default()
        };
        assert!(prepare_user_update(&mut input).is_err());
        let mut input = UpdateUserInput::default();
        prepare_user_update(&mut input).unwrap();
        assert!(input.role.is_none());
    }

    fn actor() -> AuthUser {
        AuthUser {
            user_id: "admin-1".to_string(),
            email: None,
            role: Some("admin".to_string()),
        }
    }

    fn file(match_type: Option<&str>) -> MediaFile {
        serde_json::from_value(serde_json::json!({
            "Id": "f", "Path": "/p", "Size": 1, "IsHdr": false, "AddedAt": "2026-01-01T00:00:00Z",
            "MatchType": match_type,
        }))
        .expect("media file fixture")
    }

    #[test]
    fn a_signed_in_link_write_is_recorded_as_manual() {
        let mut input = UpdateMediaFileInput {
            movie_id: Some(Some("m".to_string())),
            ..Default::default()
        };
        prepare_media_file_update(Some(&actor()), Some(&file(None)), &mut input, "now");
        assert_eq!(input.match_type, Some(Some("manual".to_string())));
        assert_eq!(input.matched_by_user_id, Some(Some("admin-1".to_string())));
        assert_eq!(input.match_confirmed_at, Some(Some("now".to_string())));
    }

    #[test]
    fn explicit_match_fields_and_unsigned_writes_are_left_alone() {
        let mut input = UpdateMediaFileInput {
            movie_id: Some(Some("m".to_string())),
            match_type: Some(Some("auto".to_string())),
            ..Default::default()
        };
        prepare_media_file_update(Some(&actor()), None, &mut input, "now");
        assert_eq!(input.match_type, Some(Some("auto".to_string())));
        assert!(input.matched_by_user_id.is_none());

        let mut input = UpdateMediaFileInput {
            movie_id: Some(Some("m".to_string())),
            ..Default::default()
        };
        prepare_media_file_update(None, None, &mut input, "now");
        assert!(input.match_type.is_none());
    }

    #[test]
    fn clearing_the_links_clears_the_match_record() {
        let mut input = UpdateMediaFileInput {
            movie_id: Some(None),
            ..Default::default()
        };
        prepare_media_file_update(
            Some(&actor()),
            Some(&file(Some("manual"))),
            &mut input,
            "now",
        );
        assert_eq!(input.match_type, Some(None));
        assert_eq!(input.matched_by_user_id, Some(None));
        assert_eq!(input.match_confirmed_at, Some(None));

        let mut unrelated = UpdateMediaFileInput {
            size: Some(2),
            ..Default::default()
        };
        prepare_media_file_update(
            Some(&actor()),
            Some(&file(Some("manual"))),
            &mut unrelated,
            "now",
        );
        assert!(unrelated.match_type.is_none());
    }
}
