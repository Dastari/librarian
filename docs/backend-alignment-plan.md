# Backend, generated GraphQL and live-data alignment plan

2026-10-24 audit; core implementation started 2026-10-25. This is not a backend rollout.
Application code, the live database and running services were not changed for
this audit. Findings refer to the current working tree, including the earlier
dependency and UI migrations.

Latest qualification, 2026-10-27: the normal recovery/migration/rollback rehearsal,
470 library tests, and 50 integration/contract tests pass after expanding the VM
to 16 GiB. One existing manual large-tree test remains ignored. The opening inventory
describes the original audit; the implementation and runtime results are recorded below.

## Decision

Keep `graphql-orm` as the entity, relationship, CRUD, policy, migration and change
event authority. Ordinary persisted reads should use generated ID/connection
resolvers and nested relations. Keep manual action mutations for orchestration,
with persisted changes going through the generated mutation path. Keep a small,
explicit set of manual reads for authentication/bootstrap, external providers,
filesystem/runtime state and genuinely computed results.

Use subscriptions to invalidate authorized query snapshots. A generated
subscription document is not proof of an emitted event, a mounted subscriber or
a correctly refreshed list. Those four parts must be verified together.

Implement the access and event fixes before adding the remaining media features.

## Audit evidence and limits

The audit is divided into reviewable inventories:

- [Every manual query, entity and relation](audits/2026-10-24/graphql-queries-relations.md).
- [Domain writes, actions and change events](audits/2026-10-24/domain-writes-events.md).
- [Frontend operations, subscriptions, polling and cache coverage](audits/2026-10-24/frontend-live-data.md).
- [Machine-readable schema and operation inventory](audits/2026-10-24/schema-inventory.json).
- [Interactive implementation sequence](audits/2026-10-24/implementation-roadmap.html).
- [Prior responsive audit and product gaps](responsive-media-audit.md).
- [Dependency upgrade and unfulfilled runtime qualification gates](glass-ux-dependency-upgrade.md).

Dependency authority is the pinned source: graphql-orm **0.57.0**, revision
`a90b5bd41f952b02416f23999c729b9ceffcbb8d`, and agql-auth **0.22.1**, revision
`86fabec09eea1eec0f675bc150e90e3809734567`.

Verified inventory:

| Surface | Current result | Interpretation |
| --- | --- | --- |
| Entity definitions | 48; 46 registered in public schema roots | StorageObject and ArtworkCache are infrastructure entities, not missing public CRUD endpoints. |
| Queries | 115 = 92 generated entity roots + 23 manual roots | Most queries are already generated. Only one manual root is a straightforward local CRUD alias; other manual reads need classification and, in some cases, correction. |
| Nested relations | 28 across 12 entities | All are generated; many foreign-key fields have no relation declaration. |
| Parent propagation | Zero explicit `propagate_change="up"` declarations | Parent notifications are not established by the present relation graph. Actual write paths and runtime brokers also need examination. |
| Mutations in committed SDL | 293 | The 230 basic entity-document fields do not include generated `upsertShow`/`upsertEpisode`; do not misclassify all 63 remaining fields as manual. |
| Subscriptions in committed SDL | 45 entity Changed fields + 5 custom fields | The registered Torrent generated subscription is shadowed by a custom type. The corrected exposure should be tested explicitly. |
| Frontend operation sources | 65 GraphQL files; 511 operation definitions | 174 queries, 287 mutations, 50 subscriptions. All validate against the committed SDL. Availability is different from runtime usage. |
| Internal data access | 51 generated repository write sites, 27 transactional hook writes, 157 typed read sites | No production application raw SQL found. Database-backed repository writes run hooks/events but bypass request ACL; review their authorization and strict generated-mutation convention separately. |
| Frontend subscription wiring | 19 call sites, 14 distinct documents | 10/45 entity documents and 4/5 custom documents have production listeners; no shared dependency consumer exists. These are source counts, not simultaneously mounted socket counts. |
| Recurring domain polling | 2 sites | Live torrent snapshots every 15s; navigation active-download count every 30s. Other identified timers have different purposes. |

This is static source, installed-library and committed-schema verification. It
does not prove runtime behavior against the newly pinned backend. No fresh Rust
schema export, integration suite, live schema apply or migration rehearsal was
run. The earlier memory-limited Rust test attempts remain unqualified; they must
pass on a suitable runner before deployment. No live access probe was used to
demonstrate the authorization findings.

Reproduce the snapshot inventory with
`node docs/audits/2026-10-24/schema-inventory.mjs` after installing the web
dependencies. This parses and validates source documents locally; it does not
start a server, introspect a running backend or establish runtime schema parity.

## Findings that set the implementation order

### Access and query purity

1. **Replace `pendingUpgrades` with generated `notifications`.** The custom root
   checks membership, then uses a raw-pool entity query without owner policy.
   Returning a Notification object does not restore root row authorization.
   Use the generated connection with action type, unresolved state, optional
   library, ordering and paging. Keep the server's owner policy; a frontend owner
   filter is not a security boundary. Evidence: `queries/quality.rs:38–63` and
   query audit Q2.
2. **Remove manufactured admin authority from collection reads.** The collection
   query passes a library ID to a service that creates an admin AuthUser, reads
   the library and writes movie metadata during the query. Preserve the actual
   caller, require a visible library, read the local overlay through generated
   Movie/MediaFile paths, and move corrections into an explicit action mutation.
   Evidence: `entities/movie.rs:414`, `metadata/providers.rs:2244`, `:2459`;
   query audit Q3.
3. **Scope runtime torrent reads, actions and streams.** Map runtime IDs/info hashes to
   authorized persisted Torrent entities. Do not expose the global librqbit
   session to any authenticated member. Keep runtime progress separate from
   slower persisted lifecycle state. Pause/resume/remove, including `deleteFiles`
   and info-hash variants, need the same check before external effects. Preserve
   owner provenance after runtime removal; reject unmapped records for members.
   Evidence: `entities/torrent.rs:391–438,648–786`,
   `subscriptions/torrents.rs:77`; query audit Q4 and write audit finding 1.
4. **Fix relationship membership.** Collection.movies currently joins only the
   provider collection ID. Include library ID in a composite generated relation,
   with no invalid inverse physical FK. Add same-owner/two-library and admin
   tests. Evidence: `entities/collection.rs:91`; query audit Q5.
5. **Audit link writes independently of relation reads.** Declaring a relation
   does not authorize assigning another owner's FK. Extend the existing
   `owned.link` policy for missing link fields and test every nullable link and
   reparenting path identified in the inventories.
6. **Authorize metadata imports and quality actions before trusted work.**
   `addTvShow` and the movie import path accept a library ID before provider
   helpers manufacture elevated authority; resolve the Library under the real
   caller first. `evaluateMediaFileQuality` must authorize its MediaFile before
   trusted reads/writes. Keep the already-admin-guarded quality approval action
   distinct from this member-access defect. Evidence: write audit findings 2–3.
7. **Stop automatic network retries of mutations.** RetryLink currently retries
   HTTP operations without checking operation kind. Make query retry explicit;
   allow command retry only after that command has qualified idempotency. Check
   the separate session-refresh replay path as well. Evidence: frontend audit F6.
8. **Authorize source-search grading targets.** `searchSources` passes no caller
   to `resolve_search_profile`, which resolves supplied Show/Movie/Album/Audiobook
   IDs through trusted repositories. Require a generated context-aware authorized
   target read before inheriting its library/profile, and authorize explicitly
   selected profiles according to their own policy. Test foreign target IDs and
   conflicting/multiple target inputs. Evidence: `entities/source.rs:521,677–718`.

These are source-level findings requiring regression tests, not demonstrated live
exploits. Wave 1 covers all related surfaces together rather than fixing only the
query that first exposed an authorization gap.

### Restore correctness is a release prerequisite

`BackupService::restore_full_backup` currently invokes `restore_snapshot` and
discards its result without restoring object bytes; its action reports success.
The comment acknowledges the missing rehydration, and the existing round-trip
test checks AppSetting rows rather than managed objects. This is a concrete
recovery gap, not a hypothetical deployment risk. Evidence:
`services/backup.rs:172–199,926–940`, `mutations/backup.rs:90–106`.

Implement a storage-backed `RestoreObjectSink` and wire the pinned backup
library's `restore_objects` path to the verified restored manifest/result. Restore
within a maintenance/staging boundary: enforce an empty approved target, verify
object keys/content hashes and paths, handle partial object failure, and report
success only after required bytes and rows are usable. Qualify the actual supported
backup scope; managed StorageObject data is distinct from arbitrary external
library media. An empty-database **and empty-object-store** round trip must compare
bytes, load artwork/managed content, and exercise corrupt/missing blobs before
calling backup/rollback qualified. Fix this in Wave 0 before using restore as a
dependency-upgrade safety gate.

### Generated query performance and completeness

The current callback-only row policy preserves authorization but disables key
batching/SQL-complete fast paths. Generated lists scan authorized candidates to
compute correct totals, subject to the configured 100,000-candidate budget.
Derived ownership checks can perform additional parent reads per candidate.
Registering a DataLoader does not establish that this app is using its batched
path. See query audit Q6.

Implement complete ownership predicates through the pinned ORM's
`RowPolicy::read_visibility` contract. Prove equivalence to the existing callback
policy for owner, derived, shared, admin and anonymous access. Retain fail-closed
callback fallback for cases where a complete predicate is not supported. Extend
the entity layer for missing typed predicate capabilities; do not introduce
application SQL or loosen policy to obtain faster queries.

Only after that work:

- Enable generated aggregate roots where a real screen needs them. The pinned
  aggregate executor rejects callback-only visibility. Available operations are
  bounded typed count/min/max/sum/grouping, not arbitrary SQL/HAVING/window queries.
- Enable generated keyset roots for suitable large changing feeds with a stable
  primary-key tie-breaker. Infinite UI alone does not turn offset pages into a
  consistent snapshot across concurrent insertions/deletions.
- Use nested connections and authorized counts rather than new list wrappers.
  Generated nested selection does not imply nested existential Where filters;
  add missing typed filter capability only for an actual requirement.
- Separate raw-pool `FindQuery` defaults from Database-backed configuration.
  Explicit pool query limits can clamp at 100 even though the application
  Database config permits 1,000. Unlimited fetches are not a safe workaround.

### Stored availability must be correct before invalidation

Album.hasFiles and Audiobook.hasFiles are persisted booleans, initialized false
by metadata import. No production maintenance path was found; browsers filter
directly on them. Emitting parent events and refetching would preserve an
incorrect value. Evidence: `entities/album.rs:123`, `entities/audiobook.rs:129`,
`metadata/providers.rs:2963,3195`, `AudiobooksBrowser.tsx:35`.

Define canonical availability from the linked playable children. Either maintain
the stored summaries transactionally through generated entity/hook operations,
or replace their filters with an equivalent authorized entity-layer projection.
Review hasFiles/count/size summaries across Show, Album, Audiobook and Library
under the same rule; do not assume they are computed. Deliver a generated-layer
backfill/reconciliation with dry-run counts and retry safety. Test first-file link,
last-file unlink/delete, rematch, parent deletion and rollback, including migrations
of existing libraries. This is explicit Wave 3 correctness work, separate from
adding notification edges.

### Events and client freshness

Rename/alias the custom `TorrentSubscriptions` type so `schema_roots!` resolves
the entity-generated type. Export and assert all 46 entity Changed roots plus
the five intentional custom roots. Existing codegen correctly skips a missing
field, so document validation alone will not catch this omission.

Add a deliberate relation and dependency graph. Examples:

| Change | Generated relations to add/use | Views that must become current |
| --- | --- | --- |
| Episode metadata, intent or file link | Episode → Show → Library | Season list, show counts/status, library browser, Wanted, home shelves. |
| Track/chapter metadata or file link | Track → Album → Artist/Library; Chapter → Audiobook → Library | Detail tables, queues, artist/book summaries, Wanted and shelves. |
| MediaFile analysis, quality or target link | MediaFile → Library and typed content targets; inverse streams/chapters | File browser, title availability/quality, player decisions and status explanations. |
| Video/audio/subtitle/chapter metadata | Child → MediaFile | Player menus, media information, relevant detail views. |
| Cast credit | Credit → Movie; generated inverse Movie.castCredits/Person.credits | Cast display and affected title details. |
| Scan run/issue | Issue → Run/Library/MediaFile; Library.scanRuns/scanIssues | Scan view, unread issues, library state and affected media. |
| PendingFileMatch / Usenet lifecycle | Typed content target and source relation, where supported | Downloading/missing/status views and affected parents. |
| PlaybackProgress / PlaybackSession | User-owned progress/session, MediaFile/typed targets | Continue Watching, row progress, active playback status; avoid whole-library invalidation on each tick. |
| Notification / settings / source | Owner feed or relevant configuration entity | Activity, badges, settings, source state and affected background work. |

This table specifies required effects, not an instruction to mark every relation
`up`. Reverse/shared dependencies and per-user progress need separate treatment.
Collection's external numeric key is not its primary key; current upward
propagation cannot simply follow that composite mapping. Prefer a stable local
Collection row ID or a reviewed entity-layer propagation extension.

The pinned propagation implementation also has boundaries:

- Current source authority is required; deleting a child can suppress propagated
  invalidations. Reparenting must reach the old and new parent, not just the final FK.
- A nullable parent must not prevent another valid sibling-parent notification.
- Propagation has 16-hop and 256-target bounds. Cycles and shared catalog/profile
  changes need bounded, deduplicated invalidation rather than broadcast fanout.
- Repository-only infrastructure sources cannot authorize a public provenance
  chain. Do not expose raw broker messages as a shortcut.

For unsupported deletion/reparenting cases, use a narrowly scoped generated
parent revision mutation in the same authorized mutation/hook transaction, with
deduplication and post-commit delivery. Use a dedicated data revision rather than
misrepresenting a child's change as a metadata refresh timestamp. If a reusable
ORM change-impact extension is preferable, implement and test it in the ORM;
do not invent another application event bus or bypass release authorization.

Existing Database-backed repository writes already run hooks and queue generated
events. Preserve that behavior during migration. The metadata helpers that write
an unchanged title/name merely to notify a parent should be retired: they can
overwrite a concurrent title edit with an old value and currently ignore payload
failure. Use qualified relation propagation or a dedicated revision instead.

Audit every generated single/bulk parent delete. Physical cascades do not execute
generated child hooks, so direct Show/Album/Audiobook/Movie deletion can skip
inverse MediaFile cleanup and child events. Define explicit transactional cleanup
and intentional retention for scan history, streams, progress and sessions.
Deleted ancestor events must clear dependent views even when child-event delivery
cannot authorize a now-missing parent.

## Target implementation boundaries

### Ordinary queries and action exceptions

Generated roots own ID/list/filter/order/page reads. Generated relations own
database relationships and nested connections. Reusable entity-layer loaders or
typed projections own computed data access when ordinary relations are insufficient.

Manual reads may remain for:

- `needsSetup` and a safe `me` response; generated User reads are admin-only, so
  removing `me` by loosening User policy would be a regression.
- Provider search/catalog data that does not exist as local entities.
- Filesystem availability/browsing and nonpersisted service capabilities.
- Current torrent/cast/transcode runtime facts, with requester-scoped entity checks.
- Explicit computed multi-entity status or runtime aggregate decisions, with
  centralized authorized fact loading, bounds and defined invalidation sources.
- Storage-backed backup capabilities/snapshots; these are not ordinary DB table rows.

Manual mutations may remain for scans, provider imports/refreshes, file operations,
matching/organization, quality approval, acquisition, cast/transport control,
backup/restore, authentication and other commands with side effects. They must
validate caller/scope first and use generated entity mutations for persistence.
Queries must not silently correct library metadata, create domain records or
perform reconnect/mount actions. Split those effects into explicit commands.

Filesystem push also needs implementation: `createDirectory` currently emits no
filesystem topic, and delete/copy/move/rename are `not_configured` stubs. Define
which operations are supported, implement real actions with compensation where
appropriate, and emit admin-scoped post-success changes. Do not present the
existing authenticated-user-wide, silent topic as functional live coverage.

Record each exception with operation name, reason generated resolvers do not
express it, read/write policy, entities touched, generated mutation entry point,
event dependencies, bounds, callers and required tests. Review new exceptions in
CI. This is a small allowlist of semantics, not a blanket exemption for a service.

### Internal writes

Preserve the repository rule: services/jobs/action workflows use generated entity
mutations through `execute_mutation` or generated resolvers in GraphQL context.
Mutation hooks use typed MutationContext operations for related writes and
transactional invariants. Infrastructure-only repository operations need a named,
documented boundary. Contextless repository methods must never stand in for
request authorization.

Do not replace correct generated internal calls merely because they are written
as GraphQL documents. Where helpful, consolidate request creation, typed generated
inputs and result/error handling into one reviewed internal executor. It must
preserve actor identity, transactions, hooks and generated event provenance.

Generated database effects and external effects need an explicit commit boundary.
Use existing deferred post-commit facilities where sufficient. Durable acquisition,
backup or worker actions need idempotent retry/recovery; frontend transport retries
must not blindly replay a non-idempotent command after an uncertain network failure.

The write inventory is a finite migration queue: classify all 51 repository sites
as ordinary domain writes to migrate or narrow infrastructure/auth exceptions.
Keep the 27 transactional hook writes in the originating transaction. Standardize
actor, owner subject, AuthUser/AuthSubject, correlation and business-payload error
handling. Eliminate the earliest-user-as-owner fallback in scanner/torrent paths;
system execution authority must not become accidental ownership.

Prioritize atomic refresh-token rotation and registration/invite consumption.
Refresh currently reads, inserts a replacement and revokes the old token in
separate transactions. Use a generated conditional mutation/CAS plus replacement
inside one transaction and assert the affected-row count. Apply the same coherent
transaction boundary to stream analysis replacement and source reordering;
several GraphQL requests are not one transaction. Preserve existing filesystem
move compensation and add durable recovery where bytes change before metadata.

### Frontend live-query lifecycle

Keep all domain operations in source `.graphql` files and codegen against the
exported schema. Hand-authored selection documents are valid; hand-maintained
copies of schema types and inline unvalidated domain operations are the drift risk.

Introduce one typed active-view invalidation registry, integrated with existing
Apollo/route lifecycles:

1. Register operation/variables, principal, semantic dataset scope, entity/parent
   dependencies and a snapshot reconciliation callback.
2. Share scoped generated metadata-only subscriptions across interested views.
   Empty lists need subscriptions too, so creation can make their first row appear.
3. Coalesce related events; mark the view dirty and perform one authorized refresh
   per affected snapshot. Do not refetch every query for every playback/file event.
4. Preserve loaded infinite windows, filters, selection scope and focus. Stage a
   refreshed complete window and replace atomically; append and replacement have
   distinct revisions. Handle filtered-out rows, changed sort positions, counts,
   deletion and old/new parent membership.
5. Register imperative snapshots explicitly. `client.query()` result arrays and
   `getObservableQueries("active")` do not automatically cover these custom hooks.
6. After reconnect, resynchronize active views to cover missed events. After
   logout/principal change, dispose subscriptions and clear scoped snapshots.
   Re-evaluate active session/role changes; reconnecting on this browser's login
   alone does not cover an administrator changing another user's access.

Subscriptions are process-local broadcasts with capacity 256 and no replay
cursor. Reconnect recovery alone does not repair silent channel lag on an open
socket. Surface overflow as an explicit resync-required signal or force a clean
reconnect/resnapshot; qualify initial query/subscription ordering with a revision
handshake or subscribe-before-snapshot protocol. Never promise exactly-once event
delivery. Require eventual correct snapshots with bounded, coalesced refreshes.

Provide a private current-session/authority-change channel or equivalent
server-enforced session revocation mechanism. A member cannot use admin-only
`userChanged` as a policy listener, and an AuthUser captured at WS admission does
not automatically reflect later demotion. Refresh the session store and route
guards on policy changes; terminate revoked streams and clear player queues,
local snapshots and old-principal caches before new queries run.

Migrate features in this order, sharing listeners where their scopes match:

| Active feature | Subscriptions/dependencies | Snapshot reconciliation |
| --- | --- | --- |
| Shell, Activity, Downloads | Notification; restored Torrent Changed; authorized runtime lifecycle/progress including pause/resume | One shell count owner instead of three CSS-hidden shell hook instances; count and loaded download window refresh on state changes. |
| Settings | Relevant settings entity Changed, including users/invites/libraries/sources/quality/casting/blocklist | Register `useSettingsConnection` callbacks; retain complete rows on refresh failure; coalesce Logs events and honor Live/pause. |
| Library browsers, scan panels | Library/content parents, ScanRun and ScanIssue | Refresh the loaded window atomically; badges observe issue resolution as well as run changes. |
| Show/audio/artist/collection details | Parent and child topics until qualified parent propagation covers them; MediaFile and typed content status dependencies | Register `useShowEpisodes`/`useAudioPages` refresh callbacks; preserve later seasons, queues, focus and complete local arrays. |
| Wanted and acquisition state | Content + PendingFileMatch/Torrent/MediaFile/quality-policy dependencies | Coalesce complete per-tab snapshots and 500-ID content-status batches; retain last good rows and inline retry. |
| Home, progress and player | Owner-scoped PlaybackProgress/Session, media/version availability | Register progress query batches; refresh membership as well as existing objects, and avoid a whole-library refresh on each checkpoint. |

Deduplicate shell and shared listeners by authenticated scope. Apollo may share a
network operation while still running three independent hook callbacks; transport
deduplication alone is not refresh coordination. The frontend audit provides the
exhaustive route/feature matrix and operation references.

Remove recurring frontend domain polling once its replacement stream is proven.
Media clock sampling, periodic progress **writes**, GraphQL WS keepalive,
exponential reconnect and PWA update checks are not substitutes for domain reads
and are not the polling targeted here. Backend reconciliation against an external
service may still be periodic; publish its changes to clients instead of making
every browser repeat that work.

## Ordered implementation waves

Each row should be delivered as several small reviewable changes, with schema
and frontend regeneration alongside any changed public contract.

| Wave | Work and principal files | Prerequisites | Acceptance gate |
| --- | --- | --- | --- |
| 0 — Reproducible baseline | Qualify pinned ORM/auth; export actual SDL; fix managed-object restore with the upstream sink API; inspect schema plan/backup; preserve authorization matrix; reconcile stale rules/docs. `schema.rs`, `backup.rs`, CI, dependency record. | None | Rust unit/API/auth/subscription/backup suites pass on an adequate runner; empty DB/object-store byte-verified restore; reviewed migration rehearsal on disposable data; no invented runtime success. |
| 1 — Access and query purity | Notifications replacement; collection caller authority/action split; import-library, quality-action and source-grading target checks; torrent read/action/stream scope; missing link policies; query-only network retries; atomic auth rotation/invite consumption. | Baseline tests usable | Two-member/admin/anonymous negative tests; concurrent refresh/invite tests; no cross-owner IDs/metadata/events; no domain write or reconnect hidden in a Query. |
| 2 — Generated query capability | Complete `read_visibility`, callback equivalence, batched ownership, Collection composite membership, targeted indexes and large-list bounds. | Wave 1 policy decisions | Mixed-owner pages/counts correct; bounded first-page work; measured 1/50/100-parent statement counts; 100k-bound behavior explicit. |
| 3 — Relations and event graph | Restore Torrent Changed root; add missing relations; generated parent propagation; revision fallback for deletion/reparenting; parent cleanup; correct and backfill stored availability/count summaries; atomic stream replacement/source reorder; implement or explicitly disable filesystem action/topic gaps. | Waves 1–2 | Create/update/delete/bulk/unlink/reparent tests; summary predicates correct on existing and new rows; rollback emits nothing; direct and parent events authorized, bounded and sufficient for stale-view recovery. |
| 4 — Frontend subscriptions | Typed invalidation registry; per-feature subscription matrix; imperative snapshot registration; remove domain polling; reconnect/role/logout handling; nested query simplification. | Wave 3 topics proven; can migrate one domain at a time | Two clients see settings, library, media, scan, wanted, activity and runtime changes without reload or recurring domain polling; infinite windows/focus retained. |
| 5 — Continuation and aggregate completeness | Source-search continuation, typed keyset/aggregate opt-ins, authoritative latest-per-title resume projection, retire fixed-cap/duplicate read workarounds. | Wave 2 predicates; Wave 3 events | More than one provider/database batch is reachable; exact/unknown/partial semantics explicit; concurrent changes and retry tested; no filter over an alleged complete partial page. |
| 6 — Personal library features | Saved predicates/manual lists, Watch Later, home shelves/library pins. | Waves 2–5 foundations | Generated owned CRUD, nested reads, migrations and cross-client invalidation; personal viewing intent never silently changes acquisition policy. |
| 7 — Versions and playback identity | Multiple file/edition model, version choice, logical progress migration and device decisions. | Access/event foundations; explicit identity design | Alternate files/cuts remain distinct; manual match/seed-source safeguards survive; progress migration and old file URLs remain compatible. |
| 8 — Listening and analysis | Audiobook bookmarks/preferences; typed intro/credit markers; bounded optional analysis jobs. | Wave 7 identity for version-specific data | User bookmark isolation; marker provenance/manual overrides; chapter/edition boundaries correct; no frontend analysis polling. |
| 9 — Offline media | Device-scoped offline manifests/queues, quota/expiry, transfer workers and progress reconciliation. | Wave 7 playback identity; event/recovery foundation | Actual offline playback, resume conflict resolution, cancellation/storage failure/revocation and reconnect tested; distinguish acquisition downloads from device copies. |
| 10 — Existing roadmap gaps | Explicit RSS/Usenet service work and unsupported-platform capability decisions described below. | Acquisition/event foundation; separate provider/platform test fixtures | A registered worker and end-to-end acceptance tests exist before describing a schema-only capability as supported. |
| 11 — Enforcement and release | Resolver exception inventory, schema/root/operation drift checks, polling/SQL ratchets, event and performance budgets, migration/deployment rehearsal. | All selected waves; ratchets introduced earlier | Full final checklist passes; stale wrappers/compatibility paths removed after callers migrate. |

Waves 6–10 are product implementation work, not permission to defer Waves 1–4.
Different feature branches can proceed after their prerequisites, but access,
identity and event contracts must land before dependent UI promises.

## Backend feature gaps and proposed entity contracts

All new persistent models should use generated string-ID CRUD, declared policies,
relations, schema registration and codegen. Prefer a stable ID plus reviewed unique
constraints for frontend-owned models; the pinned composite-key repository API
does not automatically provide equivalent public GraphQL mutation fields.

| Gap | Proposed data and generated reads | Manual actions / events / acceptance |
| --- | --- | --- |
| Source continuation | Per-provider opaque cursor, supported/unknown/exhausted state, stable source+GUID/hash identity, query/profile binding and per-source failures. If sessions/results are persisted, use owned SearchSession/SearchResult entities and generated nested result connections with TTL/caps. | Start/continue/cancel search are allowed actions. Verify each adapter's actual paging support; YTS/HTML/default Torznab batches are not exhaustion proof. Push session changes; resume/cancel/retry without duplicate grabs. Never expose credentials in results/cursors. |
| Smart collections and manual lists | Separate owned SavedView/rule and ordered membership models from the existing TMDB Collection. Use a versioned, typed allowlist of generated Where/order fields. Evaluate a saved rule through generated queries over its authorized library scope. | Rule validation/optional materialization may be actions. Do not add arbitrary SQL or assume generated nested existential filtering exists. Invalidate active results on rule, membership or relevant media changes; count the complete matching dataset. |
| Personal Watch Later | Owned viewing-intent relation to a typed title/edition, independent from wanted/ignored/autoDownload. | Generated personal CRUD and events; adding a watch-later item must not request a torrent or alter shared acquisition policy. |
| Shelves and favourite libraries | Owned HomeLayout/HomeShelf and library preference records, stable shelf IDs, order/visibility and optional SavedView/library relations. Do not store per-user layout in admin AppSetting. | Generated preference events refresh the user's Home/navigation. Avoid treating a shelf reorder as an auth/session policy change. Defaults remain usable before preference rows exist. |
| Authoritative resume/next-up | Add progress → MediaFile → typed content relations. Define a complete latest-per-logical-title query/projection; the current small progress window plus client dedupe is not an exhaustive resume index. | Prefer supported entity-layer grouping/projection; otherwise a narrowly justified computed read with owner scope and stable continuation. Progress/session events update that user's shelf, with no library-wide tick fanout. |
| Versions/editions | First expose multiple linked files through generated relations; then define Edition/ContentVersion or equivalent stable identity, primary/default selection and file membership. Preserve the current single-file reference during migration. | Import/link/select-playback actions retain pipeline rules. Track progress by intended logical edition, not accidentally by whichever encode was selected. Different cuts must not inherit an end position blindly. Device suitability remains a scoped computed decision. |
| Audiobook listening | Owned ListeningBookmark with book/chapter/version/position/label/note, and typed preferences as needed. | Generated bookmarks/events. A local sleep countdown needs no new backend entity; add server/cast scheduling only if cross-device active timers are required. Test chapter-boundary stop and per-user isolation. |
| Intro/credits markers | MediaMarker with file/version, kind, start/end, provenance, algorithm version and manual override; generated ordered marker relation. Persist analysis job state through the entity system. | Optional analyze/cancel/correct actions; use existing service lifecycle/scheduler where suitable. Bounded CPU/I/O, retry/cancellation, file-change invalidation and preserved manual markers. Skip UI consumes real markers. |
| Offline media | Owned OfflineDevice, manifest/bundle/transfer records with content identity, state, quota, expiry and device progress checkpoints. Generated queries/events present the queue. | Prepare/download/cancel/reconcile actions. Media byte/range routes remain valid HTTP transport. Define out-of-order progress conflict policy and offline authorization/storage rules; prove playback without the network, not just shell caching. |
| Better acquisition explanations | Compose authoritative content status with authorized scan issues, pending matches, notifications, quality/profile/blocklist and playback decision facts. Add typed reason fields/projections where existing fields are insufficient. | Explicit resolution/retry/approval actions, with generated persistence and relevant parent invalidation. Keep one status precedence definition. A read must not repair metadata as a side effect. |

Remove or explicitly qualify remaining bounded-discovery contracts too. The TV
guide still has a 100-episode and 1,000-schedule ceiling; browseDirectory lacks a
large-folder continuation contract; public showcase artwork uses an unordered
candidate window. Define a deliberate bounded product window where appropriate,
otherwise add authorized continuation. Do not label a capped picker, guide or
aggregate exhaustive just because its current contents fit on screen.

### Existing backend capability gaps beyond the Plex comparison

RSS feed/item and Usenet server/download entities exist, but entity CRUD is not a
feed-ingestion or NNTP/download service. The current registered service list has
Sources, Torrent, LibraryScan, AutoDownload, ScheduleSync and Transcode; it has no
dedicated RSS/Usenet worker. `docs/design.md` explicitly places real Newznab/Usenet
ingestion in follow-on work. Treat these as planned capabilities until a worker,
supported protocol, state transitions and grab → import → status/event tests exist.

For RSS, define feed scheduling/deduplication/backoff and generated Feed/Item
updates; new items must pass the same acquisition decision path. For Usenet,
define adapter/client ownership, queue lifecycle, completion/import, failure,
repair/unpack policy and PendingFileMatch integration before exposing controls.
Keep new side-effect commands manual and all persisted queue reads generated.

Do not infer first-party AirPlay, DLNA playback or Windows service/tray support
from discovery/platform types. They are explicitly unsupported in the current
design/platform implementation. Preserve clear capability reporting; implement
each as a separate protocol/platform milestone with state/events and device tests.
Their presence in a broader product roadmap does not block core GraphQL alignment.

## Verification and completion criteria

### Backend contract and policy

- Registry-to-SDL assertions for every public generated query/mutation/Changed
  field, including opt-in generated upserts and the restored Torrent root.
- Two owners, administrator, unauthenticated caller, inactive/revoked user and
  shared/admin-only entity tests for roots, nested relations, mutations and events.
- Composite Collection membership, nullable links, duplicate IDs, old/new parent
  reparenting, unlink, child deletion, parent cascade and bulk mutation tests.
- No event before commit; no event from rollback; post-commit external actions
  recover safely and are not blindly duplicated by transport retry.
- Generated list/count/aggregate visibility matches callback decisions. Statement
  counts remain bounded for increasing parent counts and unrelated owner rows.
- Stored availability/count summaries match linked content through migration,
  link/unlink/rematch/delete and rollback; events do not merely refresh stale facts.
- Full restore into an empty DB/object store verifies managed bytes and hashes,
  missing/corrupt-object failures and actual content reads before reporting success.

### Frontend and synchronization

- Two browser clients verify updates without navigation/reload: create the first
  row in an empty view; edit a value from settings; link/unlink a file; finish a
  scan/download; update progress; acknowledge activity; change permissions.
- Correct membership/count/order for filtered infinite lists and complete
  snapshots, preserving focus and already-loaded later records on refresh/failure.
- Socket reconnect after dropped events resynchronizes active views once;
  auth/principal changes dispose stale streams and snapshots.
- No recurring frontend domain-read polling outside a reviewed temporary
  exception. Progress writes, animation/media clocks, PWA checks and keepalive
  timers remain distinguishable in the lint/audit rules.
- Codegen validates all source operations against the freshly exported SDL;
  schema and operation completeness cannot silently skip a missing registered root.

### CI and documentation

The existing `.github/workflows/ci.yml` already exports schema, regenerates entity
documents and rejects schema/document drift. Extend it; do not replace it with
another hand-maintained schema. Add expected registered-root coverage, exception
inventory review, inline-operation/custom-type checks, direct-SQL and polling
ratchets, and representative event/statement-count regressions.

Extend entity document discovery to support intentionally read-only roots and
produce an explicit registry-to-SDL-to-document coverage manifest. Keep private
fields out of output/filter/order schemas and generated fragments; add regression
assertions rather than trusting broad leaf selection. Make codegen watch rerun
entity generation after SDL changes. Existing HTTP media/artwork byte routes and
tooling introspection are valid exceptions to the domain GraphQL transport rule.

Refresh `.cursor/rules/entity-single-source-of-truth.mdc`: its raw-SQL examples and
broad historical exceptions conflict with the stricter repository rule. Retire
`docs/graphql-refactor-plan.md` as a historical plan: it references removed paths
and proposes building a parallel ORM whose functionality is already supplied by
the installed dependency. Reconcile stale ownership/batching comments and the
design document's old “no Collection entity” statement with the actual source.

All future tests/builds must use an aggregate 80% CPU cap including child compiler
and linker processes; `-j 1` alone is not the cap. Run runtime/export/migration and
backup rehearsal on a runner with sufficient memory. Deployment is complete only
after the real schema plan is reviewed, data/objects are backed up, the new binary
and frontend are qualified together, and rollback is rehearsed on disposable data.

## Implementation checklist

- [ ] Runtime/schema baseline qualified on the pinned dependencies.
- [ ] Managed object bytes restore correctly, not just database metadata.
- [ ] Proven access/query-purity issues closed and negative tests passing.
- [x] Generated Torrent Changed root restored and codegen completeness asserted.
- [x] Ordinary reads use generated roots/relations; remaining manual reads registered.
- [ ] Owner visibility is SQL-complete where proven, with safe bounded fallback.
- [ ] Missing relations and parent/dependency event graph implemented and tested.
- [ ] Stored availability/count summaries maintained and existing rows reconciled.
- [ ] Direct/generated/internal/action writes have one reviewed persistence path.
- [ ] Active frontend views subscribe and recover without recurring domain polling.
- [ ] Infinite/aggregate/source-search completeness contracts implemented honestly.
- [ ] Feature model migrations and their dependent frontend flows delivered by wave.
- [ ] CI ratchets, docs, performance budgets and deployment/rollback gates satisfied.

## Audit delivery verification

Three GPT-6.1-sol research partitions covered queries/relations, writes/events and
frontend live data. Independent review identified the stored-availability,
source-grading authorization and managed-object restore omissions; all three are
now explicit implementation work above. A second independent review found no
remaining material issue within that reviewed scope.

The reproducible static inventory validates all 511 operation definitions against
the committed SDL with zero errors. Plan links and whitespace checks pass. The
standalone roadmap's scope switches/expanders work at 375px and 728px without
horizontal overflow or browser exceptions. T3's headless preview exited with
code 1; the artifact was checked with local Playwright instead. These checks
validate audit artifacts, not the application's backend behavior. Runtime schema,
authorization, event, restore and migration acceptance gates remain future work.

## Core implementation progress — 2026-10-25

The working tree now contains the core implementation. The checklist above is
still a qualification checklist: source changes and tests written do not establish
that the new backend is ready to run. The existing backend process and live data
have not been replaced or migrated.

Implemented areas include caller-scoped torrent actions and streams, explicit
system actors, transaction-bound authentication changes, pre-dispatch socket
session validation, SQL-complete typed ownership predicates, pure collection
overlay reads, generated notification queries, generated nested relations and
parent invalidations, transactional media availability summaries, and managed
database-plus-object recovery into new offline targets. The frontend has a shared
subscription dependency registry with reconnect/lag reconciliation and retained
complete windows. Source search has per-source continuation and honest unknown
or unsupported states; Resume uses existing nested entities in a bounded,
snapshot-consistent computed read.

Independent implementation reviews also exposed trust boundaries beyond the
original resolver inventory. Torrent identity, approved library filesystem roots,
notification command envelopes, naming destination containment, and validated
mutable torrent library links are part of this work, not deferred product waves.

Validation is serialized in an aggregate 80% CPU cgroup. Old project build
artifacts and stale Vitest transform caches were selectively removed; dependency
caches and running executables were preserved. Normal backend checks exceeded
this VM's practical memory/time budget. A separate, opt-in `schema-export-only`
macro feature preserves declarations and registrations while stubbing generated
runtime bodies. All 17 feature-off macro tests, including both subscription lag
branches, and five export-body AST/expansion tests pass. It is tooling only: runtime,
test and binary guards prevent using it as a server or passing off its export as
normal backend qualification. Normal-versus-tooling SDL parity and the ordinary
backend test suite remain required on a sufficiently provisioned runner.

The tooling exporter now successfully constructs the actual merged Rust schema:
122 query fields, 296 mutation fields and 54 subscription fields, including all
46 registered entity change roots. Its first execution exposed a runtime/generated
Torrent subscription type-name collision that a Rust import alias alone did not
resolve; the runtime GraphQL type now has its own name. Frontend codegen passes
against this exported SDL and asserts all 46 entity documents before replacing
output. Required generated topics replace the interim optional-export fallbacks;
the 15/30-second domain query polls have been removed.

Generated nesting now includes 54 additional relations and 19 stable upward
edges. Transaction hooks cover surviving old/new parents, Artist summaries,
Torrent/Pending/RSS dependencies and composite Movie/Collection membership.
Pending torrent provenance is `(infoHash, owner)`, conditioned on source type,
not a presumed UUID. Artist metadata-only events perform timestamp invalidation
without rescanning all tracks. The inactive custom helper/filter/pagination
modules are retired; the old refactor blueprint is marked historical.

Fresh static security reviews found no further material issue in the final
acquisition/import/quality/naming scope or the parent-summary graph. This does
not replace the pending normal-runtime authorization, event, rollback, recovery
and performance acceptance tests.

Final frontend verification passes: 60 Vitest files, 477 tests passed and one
unchanged existing skip; full and production TypeScript checks; entity coverage
and operation codegen; production Vite/PWA build. The build reports its existing
large-chunk advisory. A deterministic test exposed settings refreshes overwriting
dirty fields; incoming snapshots now preserve edits and discard uses the latest
defaults. The checked-in low-memory export script also builds and exports
successfully, and that final SDL matches the snapshot used for frontend checks.

Backend regressions are written for ownership/actions, session dispatch,
transaction rollback, complete paging, parent events, summary reconciliation,
source continuations, Resume, destination containment and object restoration.
The generated-subscription lag test uses a tiny real event channel and committed
mutations rather than fabricated witnesses. Existing HTTP ownership fixtures
now use administrator-approved library roots and still test member metadata
editing and foreign-owner denial. These backend tests have **not** run: neither
the tooling export nor the frontend passes qualify generated runtime bodies.

Before rollout, use a sufficiently provisioned runner under the same aggregate
CPU rule to run normal `cargo check --locked --all-targets`, Clippy and the full
normal `cargo test --locked -j 1` suite; compare normal SDL with the tooling SDL;
rehearse byte-verified same-schema offline recovery and the reviewed migration
on disposable targets; measure ownership/count/relation statement budgets; then
exercise cross-client reconnect/revocation and loaded-window updates. Keep the
old matched backend/frontend and recovery build available. No live migration,
summary repair or backend restart was performed during implementation.

Physical schema changes must be reviewed before application: nullable general
`MediaFile.updatedAt`, RefreshToken session/user indexes, and unique persisted
Torrent info hashes. Inspect existing duplicate torrent hashes and resolve their
ownership deliberately; do not auto-delete or pick an arbitrary surviving owner.
The new nested relations explicitly avoid adding physical foreign keys. Summary
reconciliation is an explicit administrator action with dry-run default, not a
startup repair. Offline restore must complete both database and byte verification
before either target is published.

## Personal library implementation — 2026-10-26

Wave 6 now has five generated entities: PersonalList, ListEntry, SavedView, HomeShelf and
LibraryPreference. Their caller-only scope includes administrators; links and invariants
are checked inside the generated transaction. Canonical Watch Later is independent of
Wanted. Typed version-one smart rules compile to generated filters/order, and library
deletion removes scoped rules instead of broadening them. Deletion cleanup uses typed
transaction mutations and surviving-list notifications. Registration, batch loaders,
authorization inventory and frontend subscriptions now cover 51 public entities.

The frontend provides `/lists` and `/lists/:listId`, title/track save actions, ordered track
playlists, rule editing, Home pin/hide/reorder, and favourite libraries. Complete manual
lists filter before incremental rendering; smart results use server filtering/infinite
loading and retain loaded windows during library refresh. Pending actions retain their
focus targets. Owner-only actions are not offered for administrator-visible foreign
libraries or titles.

Frontend verification passes 525 tests (71 files; one existing skip), full TypeScript,
production build, schema/codegen and layout checks. Browser fixtures pass 120 phone/tablet
checks. Backend privacy, rollback and event test execution is tracked separately below.
Early normal and staged-only recovery builds exceeded the original VM's memory ceiling,
including a service-stopped retry with a 7 GiB build limit. After expansion to 16 GiB,
the normal-runtime rehearsal passed using a 14 GiB build limit and the same 80% CPU cap.
It verifies summary repair/idempotence, managed-object byte restoration, partial failure
and restart refusal, reviewed synthetic destructive migration, incompatible-schema
recovery rejection, and fresh-target rollback. Both normal library variants built with
generated public dispatch present; this offline typed rehearsal does not exercise that
dispatch or qualify the whole application. Evidence is retained in
`/tmp/librarian-recovery-20261026-f/result.json`. The current library was not migrated or
reset, and backend/frontend services remain stopped.

## Backend runtime qualification — 2026-10-27

The memory blocker is cleared on the expanded 16 GiB VM. All Rust compilation and
test execution remained serial under `cpu.max=320000 100000` and a 14 GiB memory cap.
`schema-export-only` and the staged public-dispatch omission were disabled.

- All library tests: **470 passed, zero failed, one existing ignored**. The ignored
  test creates a synthetic 10k/100k-file tree for separate manual qualification.
- Public HTTP/cookie/authorization integration: **8 passed** (`api_auth`).
- Generated CRUD/filter/order/page and WebSocket change delivery: **5 passed**
  (`api_entities`), including create/update/delete events.
- Remaining integration/contract targets: **37 passed** — acquisition pipeline (10),
  acquisition API (5), migration/source contracts (15), guide API (3), settings (3),
  and actual loopback torrent transfer with hardlink-preserving payload deletion (1).
  Together, all eight integration targets pass **50 checks** with no failures.
- The normal source/candidate recovery rehearsal passed every phase as recorded above.
  This remains a synthetic schema transition, not a historical-release upgrade test.
- Formatting and whitespace checks pass. No application-domain SQL was added.

Runtime execution exposed and corrected test fixtures missing real policy providers
or ORM authentication data, plus a subscription test that stopped polling while a
single pooled connection was held by its pending authorization read. Tests now retain
their ownership, privacy, exact-target and transaction assertions without permissive
policies, skips or pool-size workarounds.

Application corrections from this qualification pass:

- Narrowed session role claims remain narrowed; current-account authority does not
  promote them. Expiry, active account and persisted session checks still apply.
- Logout revokes the session family, including rotated successors. Family selection,
  complete paging and revocation share one `StateMachine` transaction, serialized
  against refresh rotation. Independent sessions remain active.
- Password-hash changes revoke sessions in the user update's transaction, as role
  changes and disablement already do. Foreign users' sessions remain untouched.
- Mutable Torrent library links validate fresh Torrent and destination ownership
  through the originating write transaction. Field policy no longer re-enters a pool
  whose connection is already held. Link/relink/unlink, admin override, hard rejection,
  uncommitted ownership and rollback behavior pass their regressions.

Logs: `/tmp/librarian-expanded-ram-lib-test-build-4.log`,
`/tmp/librarian-expanded-ram-lib-tests-2.log`, and
`/tmp/librarian-expanded-ram-api-tests.log`, and
`/tmp/librarian-expanded-ram-remaining-integration-tests{,-2}.log`. Two static contract
false positives were corrected: SQL detection ignores whole-line comments, and the
retired local `macros` dependency check matches its dependency key rather than the
optional `graphql-orm-macros` tooling package. The security review found no remaining
material issue in the final session-family and mutable-link scope. The live database
was not migrated/reset and application services remain stopped for manual restart.

These checks do not establish physical iOS/Safari/TV behavior, real playback/decoding,
large-library performance or compatibility with an actual historical backup release.
