# Personal and Glass browser fixture checks

This standalone script exercises the existing managed Vite UI with Playwright.
It does not import the regular Playwright configuration, start the sandbox or any
server, build the application, use live credentials, or access the live database.
The user authorized Playwright fallback after the T3 browser failed with the
Chrome root sandbox prerequisite `serverSandbox0missing`.

Run source and schema checks without a browser:

```sh
cd web
node --check e2e/personal-browser.mjs
node e2e/personal-browser.mjs --check
```

When the browser lane is free and `web/live.sh` is already serving port 3000:

```sh
cd web
node e2e/personal-browser.mjs
```

The default matrix is 375, 390, 768 and 834 CSS pixels, each in light and dark
mode with Solid material and reduced motion. It uses one Chromium context at a
time. A bounded diagnostic run can select one matrix:

```sh
PERSONAL_UI_WIDTHS=390 PERSONAL_UI_MODES=dark node e2e/personal-browser.mjs
```

The bounded final-controls follow-up uses only 375/834 dark Solid and adds a
populated audio dock, visible volume, Now Playing and queue check per width:

```sh
node e2e/personal-browser.mjs --post-smoke
```

This runs 32 checks (30 existing plus two combined audio-control checks), writes
latest results to `results.json`, and prefixes screenshots with `post-smoke-`
to retain the original screenshots. Before this follow-up, the successful full
matrix JSON was copied unchanged to `results-full-120.json`. Fixture tracks now
both have file identities so the queue can contain two items, while stream bodies
still contain no playable audio. Volume assertions inspect control callbacks and
the media element's volume/muted state; they make no decoding claim.

`PERSONAL_UI_URL` may select a different already-running localhost origin;
`PLAYWRIGHT_CHROMIUM_EXECUTABLE` may select an installed Chromium binary.
Screenshots and structured `results.json` go under the ignored directory
`web/tmp/screenshots/personal-browser/`. A failed assertion exits nonzero.

All GraphQL HTTP operations are intercepted at the browser network boundary,
validated against the current `web/schema.graphql`, then executed against an
in-memory fixture resolver. Actual incoming generated documents are used.
Personal mutations change only the fixture state. Catalogue fixtures include
movies, libraries, tracks, an album and their parent relations. Unknown roots,
filters or API routes are reported as fixture failures. GraphQL websocket
connections receive a local acknowledgement with no server connection and no
change events. Service workers are blocked. Other origin requests and non-asset
writes are blocked; artwork is a labelled fixture SVG.

The checks cover list creation/removal/deletion, saving a movie to a manual list
and Watch Later, smart collection creation/editing/results, playlist reordering, Home pinning and shelf
settings, library favourites, search, menu keyboard/focus behavior, modal focus
containment/return, video menu controls, and mocked sign-in errors/success.

**Qualification: UI fixture checks only.** This does not qualify the running
backend schema, authorization, database constraints, real subscription delivery,
playback/decoding, physical devices, TV/remote behavior, or visual correctness.
The media stream fixture contains no playable media. Catalogue scalar defaults
exist only to supply nonessential metadata selected by generated documents.
Screenshots are evidence for review, with automated checks limited to asserted
behavior and document overflow. Console errors are retained in the artifact;
uncaught page errors and GraphQL errors fail the boundary check.

## Recorded fixture run

On 2026-10-26, 22:18:36–22:24:01 UTC, the complete matrix passed **120 checks,
0 failures**: 15 checks for each of 375/390/768/834 × light/dark, all Solid.
The preserved ignored full-matrix artifact manifest is
`web/tmp/screenshots/personal-browser/results-full-120.json`, with 120 referenced PNGs
beside it. It records 2,858 intercepted HTTP GraphQL operations, no GraphQL
validation/execution errors and no uncaught page errors. The only console error
class was `ERR_BLOCKED_BY_CLIENT` from intentionally blocked external imagery.
Chromium exited at completion; the parent browser/build lane was released.

An initial attempt encountered an absent managed Vite process. The script now
checks the existing origin before launching Chromium and exits with code 2 if it
is unavailable. Subsequent diagnostic fixture fixes added the content-status
root, preserved Vite HMR while mocking only the GraphQL websocket, waited for
responsive action layout, corrected EmptyState text matching, and used Glass's
actual `dialog` role for confirmation. Subtitle/audio selection is not asserted
because those menus require real native/HLS tracks; playback speed/menu focus is
asserted. Diagnostic failure screenshots were removed after the passing run.

No production UI defect was identified by these assertions. Shelf writes are
verified in fixture state and pinning is verified through a fresh Home route;
cross-mounted snapshot reconciliation remains outside this silent-subscription
fixture test. No live DB writes, backend restarts, builds, compilers, or additional
agents were used.

The final-controls post-smoke subsequently passed **32 checks, 0 failures** on
375 and 834 dark Solid. Latest results are in `results.json`; the prior 120-pass
manifest remains in `results-full-120.json`. Both widths passed the added
populated audio dock/Now Playing/queue check. At 834 the visible dock Slider
changed the audio element volume to 0.98 and mute toggled its muted property;
both widths changed Now Playing volume to 0.02, selected the second queue item,
returned focus to the opener and stopped/removed the dock. Screenshots include
`post-smoke-{375,834}-dark-solid-audio-now-playing-open.png` and
`post-smoke-{375,834}-dark-solid-audio-dock-populated.png`. No GraphQL fixture
execution/validation errors or uncaught page errors occurred. Chromium exited
and the browser lane was released after the run.
