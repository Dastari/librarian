# Project review — 2026-09-05

## Assessment

Librarian has a substantial implementation of its core media-library workflow, but is not yet
feature complete or release qualified. Movies, shows, music, audiobooks, torrent acquisition,
metadata matching, organization, persistent players, quality evaluation, and Google Cast all have
real application code. The main remaining work is completing recovery and unattended-operation
workflows, correcting several misleading controls, and qualifying the system with real media,
providers, receivers, and platform installations.

This review uses the current working tree, including the extensive work that was already uncommitted
when the review began. Earlier July audits are historical evidence, not current test results. No
development server was started and no user library, database, download, or receiver was modified.

The frontend actually runs as a Vite React SPA with TanStack Router; `docs/design.md` and `AGENTS.md`
still describe TanStack Start. Migrating to Start is not necessary to complete the local application;
the architecture description needs reconciliation. Likewise, the frontend rule pack still describes
the HeroUI umbrella package, while the implementation uses individual component packages and a shared
data-table adapter.

## Capability inventory

| Area | Current implementation | Remaining completion evidence or work |
| --- | --- | --- |
| Four library types | Catalog/detail pages, provider lookup, wanted flags, scan/match/link paths | End-to-end fixtures for all four types, including multi-disc albums and multi-chapter books |
| Existing-library onboarding | Scheduled scans, bounded discovery/analysis queues, durable runs/issues, manual match, unchanged-file reuse | Provider-backed onboarding, slow/network filesystems, full-pipeline scale qualification |
| Organization | Naming templates, hardlink/copy import, no-clobber moves, compensation, duplicate review | Cross-filesystem failure/restart rehearsal with representative media |
| Torrent acquisition | Stable librqbit client, public/private sources, Torznab, completion/reconciliation, wanted-target linkage | Multi-user ownership and source isolation; unattended long-running qualification |
| Auto-download | Background monitor, manual trigger, missing-item discovery and quality ranking | Ownership, pagination, query budgets, and profile-error behavior described below |
| RSS | Feed/item entities and legacy form components | No registered polling/acquisition service; no complete unattended RSS workflow |
| Quality | Video/audio profiles, analysis-derived quality status, candidate filtering, approval of encountered better files | Cutoff controls promise proactive searches that are not implemented |
| Browser playback | Persistent audio/video, direct stream/range support, HLS remux/transcode | Browser/network matrix, session concurrency, seeking, codec/HDR and resource qualification |
| Cast | Discovery, bounded transport, scoped grants, receiver state/control, compatibility decisions | Physical Chromecast/Google TV qualification, reconnect, IPv6 and multi-interface testing |
| Auth and privacy | HttpOnly cookies, origin checks, owner/admin policies, encrypted source credentials, private artwork | Extend behavioral coverage to background acquisition and full browser sessions |
| Backup/storage | Local object storage, full logical database backup, object backup, verify, empty-database restore | Object rehydration, complete object enumeration, operator restore workflow and upgrade rehearsal |
| Delivery | Linux/Windows portable build workflows, Docker, embedded asset feature, security/schema CI | Actual CI results and fresh-install/upgrade smoke tests on target systems |

Usenet/NNTP, automatic archive extraction, managed subtitles, first-party AirPlay, DLNA playback,
and Windows service/tray/MSI integration are explicitly outside the current supported scope in the
design. They are expansion work, not completed features. Keep that distinction when defining v1.

## Prioritized remaining work

### P1 — Make backup recovery complete and prove it at scale

`backend/src/services/backup.rs::restore_full_backup` restores database rows but does not rehydrate
primary object-storage bytes. It returns `Result<()>`; its comment claiming that it returns object
keys is stale. The capability object nevertheless reports `restore_available: true`.

`LibrarianBackupObjectIndex::list_objects_for_full_backup` also calls a single `fetch_all()` with no
pagination. Librarian installs the ORM's legacy pagination policy, whose default limit is 1,000.
That makes object enumeration incomplete once the cache grows past that limit. Small setting-only
round-trip tests cannot establish that a populated library can be recovered.

Acceptance: restore into a fresh temporary database and empty object root, with more than 1,000
objects; verify every object checksum and artwork lookup; preserve external encryption-key recovery
instructions; prove that a missing/corrupt object and interrupted restore fail visibly. Add an
operator-accessible restore path with clear empty-target requirements. Media files in library roots
need their own explicitly documented backup policy.

### P1 — Carry ownership through unattended acquisition

`backend/src/jobs/auto_download.rs::run_once` discovers candidates across libraries but assigns all
new torrents the value returned by `get_default_user_id`. That helper selects the earliest-created
user, rather than the candidate library's owner. The retry sweep also synthesizes an admin identity
using that default user. This is a correctness gap even though the manual trigger is admin-only:
background work must retain the intended library/torrent owner.

Candidate discovery uses repeated unpaged `fetch_all()` calls and a per-item unresolved-torrent
lookup. Large libraries can be truncated by the ORM's pagination default and incur many queries.
When profile resolution errors, acquisition falls back to unrestricted “Any Quality,” silently
weakening the user's configured constraints.

Acceptance: two-member/admin integration fixtures through discovery → grab → import; torrents and
notifications belong to the correct owner; permitted sources are explicit; candidates beyond the
first page are processed; profile lookup failures skip/retry with a visible error; periodic runs
have bounded work and cancellation. Preserve the no-overwrite and manual-match rules.

### P1 — Bound and coordinate HLS sessions

`backend/src/services/transcode/service.rs::get_or_start_session` checks, removes, spawns, and inserts
in separate steps. Concurrent first requests for the same file can create competing ffmpeg jobs;
`SessionRegistry::insert` kills the earlier session, invalidating the directory returned to its
request. Different files have no global process or disk-budget admission limit. Different playback
modes for one file share the same registry key and can replace each other.

Acceptance: serialize creation per media/mode, define coexistence of browser and Cast modes, bound
global processes/cache bytes, recover failed children, and test simultaneous requests, disconnects,
seek/resume, and restart cleanup. Browser HLS and physical receiver playback remain separate gates.

### P2 — Close visible promises: quality cutoffs and RSS

`frontend/src/components/settings/QualityProfileEditorModal.tsx` exposes “Seek upgrades until cutoff.”
The corresponding `should_seek_upgrade` helper in `backend/src/services/quality/profile.rs` is compiled
only for tests. The production monitor searches missing files, not existing files below cutoff.
Existing design Q23 also explicitly says suboptimal quality does not automatically trigger downloads.

Choose one coherent v1 contract: hide/label inactive cutoff automation, or implement opt-in searches
that propose upgrades while retaining explicit approval before replacing library files. A stored
profile flag alone is not an implementation.

RSS is similarly mentioned in the product summary, but source vocabulary and entity CRUD do not
constitute a downloader. Implement polling, bounded XML/HTTP handling, durable GUID deduplication,
ownership, retry/backoff, enabled settings, and the existing normalized torrent import path before
claiming support. If RSS is deferred, make the summary and navigation say so consistently.

### P2 — Finish operator workflows and frontend consistency

- The unmatched-file screen still has a disabled “Remove from database (coming soon)” action.
- Most forms use manual state; only `LibrarySettingsForm` uses the design's react-hook-form pattern.
- Backup settings imperatively copy Apollo results into local state, despite the hook/cache convention.
- The main production vendor chunk remains about 2.35 MB minified (about 694 KB gzip in this build).
  Route chunks exist, but the catch-all vendor grouping limits the benefit of lazy media/UI loading.
- Add browser tests for login/refresh/logout, scan issue recovery, manual matching, quality approval,
  player continuity across navigation, and narrow-screen tables. The new real table integration test
  is useful coverage but is not a substitute for those flows.

These are incremental completion tasks; they do not call for another wholesale frontend rewrite.

## Suggested delivery sequence

1. **Dependency baseline:** land the reviewed manifests, lockfiles, compatibility fixes, vendor patch,
   generated types, and CI toolchain alignment together. See the companion dependency report.
2. **Recoverability:** finish object restore and pagination; rehearse backup → upgrade → restore on
   disposable data. ORM schema fingerprints changed upstream, so old snapshot compatibility needs an
   explicit rehearsal before a production upgrade.
3. **Unattended reliability:** owner-aware acquisition, bounded discovery, fail-closed quality errors,
   and coordinated HLS sessions. Require integration tests at service boundaries.
4. **Product contract:** settle RSS and cutoff-search scope, finish actionable empty/error states,
   and remove stale promises from documentation and controls.
5. **Release qualification:** exercise real provider metadata, downloads, browser playback, Cast,
   slow mounts, Windows/Linux installs, and upgrades. Record environment and evidence for each gate.

A useful v1 exit criterion is successful onboarding → acquire → match → organize → play → recover
for each supported library type, with no wrong-owner writes or silent data loss. An exact percentage
complete would obscure these gaps; a green compile and feature-shaped settings pages are insufficient.

## Verification from this review

Verification is recorded in `dependency-upgrade-2026-09-05.md`. The frontend type-check/build and
26 tests pass. Backend checks and strict Clippy pass; 193 unit tests and 15 repository contract tests
pass using the memory-limited procedure documented there. The ordinary Cargo test compilation hit
this machine's memory limit, so a successful full default Cargo test run is not claimed. Registry
checks, full/production npm audits, Rust audit/deny, and unused-dependency checks were run against
the upgraded dependency graph. Backend schema export and deterministic frontend code generation pass.

The ignored synthetic discovery harness also passed with 100,000 empty media files (314 ms measured
traversal on this temporary filesystem). This checks bounded discovery and complete enumeration;
it does not qualify network mounts or the provider-backed matching/import pipeline at that scale.

An isolated eight-second synthetic H.264/AAC MKV also passed the current FFmpeg remux and transcode
argument paths: both produced complete HLS playlists and probeable H.264/AAC segments. This verifies
those FFmpeg commands only, not authenticated HTTP delivery, HEVC/HDR behavior, seeking, or Cast.

No application server was listening on local ports 3000/3001 during this review. Browser visual
qualification, real provider/receiver workflows, and Windows installation are not claimed.

## Authenticated frontend follow-up

The [frontend audit](frontend-audit-2026-09-05.md) records fixes and live validation for artwork, HLS movie playback, dashboard media coverage, tables, notifications, and mobile layouts. It also records remaining session-longevity, duplicate-data, torrent-path, and end-to-end verification gaps.
