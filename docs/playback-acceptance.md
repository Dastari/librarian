# Isolated real-media playback acceptance

`scripts/playback-acceptance.sh` compiles the real acceptance fixture, starts its
disposable backend, builds the copied production UI under the CPU quota, serves
it with Vite preview on `127.0.0.1:3003`, runs the Chromium
acceptance script, then shuts down its own processes. `--serve` holds the fixture
and UI for manual inspection instead of running the browser checks. Only the
explicit combination `--serve --lan-ip <assigned-RFC1918-IPv4>` exposes this
disposable fixture on the physical LAN and registers the real Cast service.
Default invocations remain loopback-only, with no Cast service or discovery.

Run only after the parent releases the compilation/browser lane. Physical
devices require their own checks regardless of a Chromium run's result.

## Prerequisites and exclusive lane

- Linux with cgroup v2, Python 3.9+, Bash, Cargo, Node, FFmpeg and FFprobe.
  FFmpeg needs the fixture's `libx264`, AAC, MPEG-4 and `libmp3lame` encoders.
- Existing locked/offline Rust dependency caches and installed `web/node_modules`
  with Vite. Automated runs also need Playwright and a compatible Chromium browser
  already provisioned; manual `--serve` runs do not require a browser.
  The launcher never installs dependencies or downloads a browser. It preserves
  `PLAYWRIGHT_BROWSERS_PATH` for an existing nondefault browser installation.
- A parent-provisioned, writable **leaf** build cgroup with a finite `cpu.max`.
  The launcher does not create, configure or delete cgroups. Its aggregate quota
  must be at most 80% of available CPUs, including compiler/linker descendants.
  Four available CPUs permit `320000 100000`; a smaller quota is also accepted.
  Availability is bounded by process affinity, the build group's effective
  cpuset and visible ancestor CPU quotas of both the launcher and build group.
  The launcher itself must start outside the build group.
- Exclusive use of that cgroup for the whole run, including the manual hold.
  The launcher takes a nonblocking advisory lock on the cgroup directory inode
  and rejects a populated group, including descendants, before compiling. This
  coordinates other copies of this launcher. Other build tools must honor the
  same lock or be kept out of the lane by the parent: a cgroup is not itself a
  scheduler, and an uncooperative concurrent writer cannot be excluded by this
  script. It never waits for or stops another compiler. Do not run another build
  against its Cargo target directory concurrently either. Keep backend and web
  source unchanged during compilation/snapshotting so a run represents one set
  of inputs.
- A free TCP port 3003 and an absolute **nonexistent** run-directory path whose
  parent exists. Every invocation needs a new path; even a failed run is retained.
  Do not use a live media/data directory. The run contains test credentials and
  is created private to the invoking user.
- LAN mode additionally requires iproute2's `ip` command. It inspects the local
  interface address list without DNS, discovery or a network connection. The
  supplied IPv4 literal must belong to `10/8`, `172.16/12` or `192.168/16` and be
  assigned to an UP interface on this machine. Public, wildcard, loopback,
  link-local, hostname, IPv6, CIDR, port-suffixed and foreign-host values fail
  before compilation. A successful bind alone is not proof of assignment when
  `ip_nonlocal_bind` is enabled, so the backend also checks its interface list.

## Invocation

Run from the parent shell with the same exported test-profile/build settings
that produced the existing dependency artifacts. For example, if the parent
used `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_PROFILE_TEST_CODEGEN_UNITS=256` and
`CARGO_INCREMENTAL=0`, keep those exact values exported. These are examples,
not settings imposed by the launcher. It inherits the parent's build environment,
including `CARGO_PROFILE_TEST_*`, `CARGO_TARGET_DIR`, allocator and ordinary
compiler settings; it forces only `SQLX_OFFLINE=true`. Review custom compiler
wrappers before use. Known schema-export/rehearsal wrappers or flags are refused.

```bash
# Substitute the EXISTING idle group assigned by the parent; this does not create it.
./scripts/playback-acceptance.sh /sys/fs/cgroup/librarian-builds /tmp/playback-acceptance-001

# A separate invocation always gets a new directory.
./scripts/playback-acceptance.sh /sys/fs/cgroup/librarian-builds /tmp/playback-acceptance-002 --serve

# Opt-in physical LAN serving; 10.0.1.203 is the user-confirmed VM address.
# The launcher still verifies that it is actually assigned at invocation time.
./scripts/playback-acceptance.sh /sys/fs/cgroup/librarian-builds /tmp/playback-acceptance-003 --serve --lan-ip 10.0.1.203
```

`--lan-ip` requires `--serve`. It never launches the automated browser, discovers
receivers or loads a clip onto a TV. An inherited `LIBRARIAN_PLAYBACK_LAN_IP`
does not enable LAN mode in the launcher; only the explicit flag does.

Optional `--build-timeout SECONDS`, `--ready-timeout SECONDS` and
`--browser-timeout SECONDS` default to 7200, 300 and 900. The readiness timeout
applies separately to fixture generation and Vite startup. Manual hold ends on
Ctrl-C, a `stop` marker or the fixture's own two-hour limit after readiness. From
another shell, `touch /tmp/playback-acceptance-002/stop` requests normal shutdown.
Read login credentials from that run's `ready.json`; do not publish that file.
LAN runs generate a new random password rather than exposing the known loopback
test password.
Ctrl-C returns 130 even when cleanup succeeds.

## Exact execution contract

The build runs from `backend/` inside the supplied cgroup:

```text
cargo test --locked --offline --no-default-features --features sqlite \
  --test playback_acceptance --no-run --message-format=json-render-diagnostics -j 1
```

This builds the normal SQLite runtime without `schema-export-only`. There is no
schema-export command. Serial Cargo jobs help memory use; the cgroup's aggregate
quota is the CPU limit. Cargo JSON is retained and parsed for exactly one
`playback_acceptance` test artifact from `backend/Cargo.toml`, with the SQLite
feature and a successful build-finished event. The executable is copied to the
run directory, avoiding filename guesses and later target-directory replacement.

The launcher executes that binary directly, **outside** the build cgroup:

```text
playback_acceptance --exact real_media_fixture_and_stream_contract --nocapture --test-threads=1
```

`LIBRARIAN_PLAYBACK_ACCEPTANCE_DIR` points to the exclusively reserved run directory.
`TMPDIR` points inside the run. The current `TestApp` creates its own ephemeral
SQLite database, media, cache, storage and signing/encryption state there; it does
not load production configuration or start the torrent/acquisition workers. All
fixture catalogue writes use generated GraphQL operations. No live database is
copied, queried, migrated or modified by this launcher, and it issues no raw SQL.

Only in LAN mode does the launcher set `LIBRARIAN_PLAYBACK_LAN_IP`. Only
`backend/tests/playback_acceptance.rs` reads that variable and supplies explicit
`TestAppOptions { bind_ip: Some(ip), with_cast: true, ... }`. The shared harness
does not read it: its defaults remain `bind_ip: None`, `with_cast: false`.
The LAN listener binds exactly that assigned private address on port zero and
keeps the resulting ephemeral port reserved throughout startup. Config's host,
origin, advertised media URL and CastService's media base URL all use
`http://<LAN-IP>:<ephemeral-backend-port>`. Cast auto-discovery is false, including
the fixture's persisted settings row (updated through generated entity mutations).
There is no startup discovery or receiver LOAD.

`LIBRARIAN_ACCEPTANCE_FFMPEG` points to a generated wrapper that moves only its own
process into the supplied cgroup and execs the real FFmpeg. Fixture generation
and backend HLS packaging therefore share the same aggregate encoder/build cap.
Runtime `PATH` also starts with that wrapper's directory for bare `ffmpeg` calls.
The frontend production build also runs inside that group. The test server,
launcher, preview server and browser stay outside it.

The fixture generates real media and checks unauthorized stream rejection,
FFprobe-backed media info, an authenticated byte range, and actual HLS playlists
and segments for remux/transcode. Only then does it create `ready.json` with
`create_new`, writing version 1, its ephemeral `backendUrl`, fixture paths, IDs
and test credentials. The launcher tolerates the brief partially written JSON
window, validates the expected backend host (loopback by default, the exact
validated IP in LAN mode) and run-local fixture root, and monitors
server exits while waiting. The fixture polls for `stop` every 250 ms, holds for
at most two hours, and calls `app.shutdown()` afterward.

Vite builds and previews a copy of the web source/public assets/config in the
run directory. This avoids cold development-module discovery and HMR disrupting
an acceptance interaction. Installed dependencies are linked from
`web/node_modules`; no install, codegen script, `pnpm dev` or managed live-server
command runs. The copied generated documents are built as-is; the parent must
complete codegen/typechecking before acceptance. Router-generated files and Vite caches
belong to the disposable copy. Configuration uses the runner loader to avoid
bundled-config output in the shared dependency directory. Web `.env*` files are
not copied. The process explicitly receives:

```text
PORT=3003
BACKEND_PROXY_TARGET=<ready.json backendUrl>
DEV_SERVER_PUBLIC_URL=
```

Vite binds `127.0.0.1:3003` with `strictPort`; an occupied port causes failure,
never eviction or port fallback. A read-only GraphQL request verifies its proxy.
With `--serve --lan-ip`, preview instead binds exactly `<LAN-IP>:3003`. Its explicit
allowed host and HTTP CORS origin use that same
LAN address and port 3003; `DEV_SERVER_PUBLIC_URL` remains empty. Backend CORS
also allows `http://<LAN-IP>:3003`. No wildcard interface or public origin is used.
The browser runner is a snapshot of `web/e2e/playback-browser.mjs`, invoked with
this run's `ready.json` and `PLAYBACK_UI_URL=http://127.0.0.1:3003`. Runtime
environments exclude inherited production configuration and `PLAYBACK_CASE`, so
the default run attempts the complete runner rather than a parent-selected subset.

## Evidence and shutdown

The run retains `resources.json`, `build-command.json`, `binary-sha256.txt`, the
copied binary/web inputs, `logs/build.log`, `logs/cargo.jsonl`, `logs/fixture.log`,
`logs/vite.log` and `logs/browser.log`. A browser run also writes
`browser/results.json` and screenshots. Build diagnostics may be in both Cargo
stderr and JSON messages. Check the exit status and all result rows; a ready
manifest alone proves neither browser decode nor completed acceptance. `--serve`
produces no automatic browser verdict.

The current runner exercises real video frames/audio readiness, direct/remux/
transcode playback, seek, captions/audio selection, episode continuation and
identity, music queues, audiobook resume, missing files, retry recovery and
390/834-pixel layouts. Successful playback uses real UI/API/media. Its retry case
deliberately injects a temporary 503 for one media-info request and removes it
before verifying real decode; this is fault injection, not a media substitute.
Read the current runner and its recorded results when interpreting coverage.

Normal exit, startup failure, browser failure, timeout, SIGINT, SIGTERM and SIGHUP
all request fixture shutdown with `stop`, allow up to 30 seconds for graceful
completion, then terminate/wait only process sessions created by the launcher.
Their leader PIDs remain reserved until group cleanup to avoid signaling a reused
PID. Cleanup never kills by executable name or port, never writes `cgroup.kill`,
and never stops production or a foreign process. SIGKILL, machine loss and tools
that deliberately detach from their process sessions cannot receive this graceful
cleanup guarantee. Temporary fixture state normally disappears when TestApp is
dropped; forced termination can leave it under the retained run's `tmp/`.
The launcher retains its cgroup advisory lock through cleanup and preserves logs
and evidence. Remove an old run manually only after reviewing it and confirming
its processes have ended.

## Physical LAN receiver handoff (runtime acceptance pending)

Chromium layout emulation and decoded frames do not qualify iPhone/iPad Safari,
native HLS, native AirPlay, Android TV hardware, Google Cast receiver playback,
remote controls, audible language selection or actual device fullscreen behavior.
Record device model, OS/browser version, media path, observed decode/audio,
controls, resume and failures separately. DLNA discovery is informational only;
Librarian has no qualified DLNA playback adapter or first-party AirPlay controller.

The user has confirmed a TV with built-in Google Cast on `10.0.1.0/24`; the VM's
observed address is `10.0.1.203`. The suggested TV address is `10.0.1.223`, which
refused TCP connections on ports 8009 and 8008 during initial checks. Its awake
state and actual Cast endpoint still need confirmation. This launcher does
not guess a receiver, discover one automatically or select a target. After the
parent releases its serial validation lane, start the explicit LAN command above
and open `http://10.0.1.203:3003` manually. Authenticate with the disposable
credentials in that run's `ready.json`. The existing manual Discover action in
Settings → Casting can be used later to identify the confirmed TV. Discovery
itself does not request playback; select/verify the TV before manually casting.

The receiver must fetch the backend's authenticated/scoped `/api/media/...`
stream, playlist and segment URLs **directly** at the LAN backend origin printed
by the launcher and recorded as `backendUrl` in `ready.json`. Port 3003 is the UI
and GraphQL/browser proxy, not the advertised receiver media endpoint. A browser's
localhost URL, public UI origin or workstation-only SSH tunnel cannot supply
receiver reachability. The parent must separately establish LAN routing/firewall
and mDNS reachability; address assignment does not prove the TV can reach the VM.
`LIBRARIAN_ADVERTISED_MEDIA_URL` does not reconfigure this explicit test fixture.

Direct media requests retain the real socket peer IP via Axum ConnectInfo and
the existing receiver-bound Cast grants. `trusted_proxies` remains empty. The
launcher adds no media relay, forwarded-header trust or relaxed receiver-IP
binding. Do not route Cast receiver fetches through Vite or another proxy to
make a failed peer-IP check pass.

This is a **plain HTTP, no-TLS local test** on a trusted private LAN. The login
(`admin@example.test` / `Test-Password-123!`) and signing secrets are public,
known fixture values, solely for disposable generated content. Never reuse them
for a real account or include personal media, production data or secrets in this
fixture. LAN clients can reach the temporary UI/backend while it is serving.
Stop with Ctrl-C or the run's `stop` marker; the normal two-hour limit and owned
process cleanup still apply. Review retained evidence, confirm those processes
have ended, then destroy that exact run directory, including `ready.json`, copied
inputs and any leftover `tmp/` state. Do not reuse a run directory or credentials.

Launcher syntax and embedded Python can be statically checked without running
Cargo, tests, servers, browsers or network probes. A ready fixture does not prove TV discovery,
receiver authentication, decode, controls or completed physical acceptance.

## Recorded VM qualification — 2026-10-27

- Normal SQLite backend: 497 library tests passed, one existing ignored test.
  All 29 selected integration tests passed across acquisition pipeline/API,
  authorization, generated entity API and real media delivery. Formatting passed;
  no schema-export/runtime-stubbing feature was enabled. Audio selection was
  verified by decoding delivered audio and distinguishing the fixture tones.
- Full frontend suite: 586 passed, one existing skip, 76 files; TypeScript and
  production build passed. The build retains its chunk-size advisory.
- Production browser run `/tmp/librarian-playback-20261027-h`: all 17 real-media
  cases passed, including caption cues, audio-language delivery, episode identity,
  queue replacement, persisted checkpoints, error recovery and 390/834px controls.
  The launcher built the copied UI, started the actual backend, then cleaned up
  all its processes successfully. Per-case times were below four seconds.
- Twelve separate production navigation probes passed. Earlier cold-development
  runs were disrupted during module loading; the launcher now serves a production
  bundle. One earlier production run also observed an incomplete refresh request
  during navigation. Its cause was not proven; source review found no concrete
  database/progress lock cycle. The runner now waits for the authenticated shell
  before fixture mutations and further reloads. Two subsequent full production
  runs passed. This is not a claim of exhaustive session/network qualification.
- Physical iPhone/iPad, TV, receiver decoding, audible language selection,
  long-form media and reconnect behavior remain separate acceptance gates. The
  synthetic fixtures are short, low-resolution samples, not a performance claim
  for HDR/4K or multi-hour transcodes.
- Builds used the shared `320000 100000` CPU quota and 14 GiB memory limit, one
  Rust compiler at a time. The cgroup OOM-kill counter remained unchanged. No
  live database was copied, migrated or reset; normal application services stayed
  stopped. Superseded copied test binaries were removed while evidence was retained.
