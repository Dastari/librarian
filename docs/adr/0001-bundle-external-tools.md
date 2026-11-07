# ADR 0001: Ship external tools beside the binary, in one installer

- **Status:** Proposed. Nothing here is built yet.
- **Date:** 2026-11-06
- **Decision needed from:** Project owner, on the [open questions](#open-questions)
- **Related:** [Distribution plan](../distribution-plan.md), [design.md](../design.md) Q52c

## Context

Librarian is meant to install as one self-contained package, but several features run external
programs:

| Dependency | Used for | Size | Where it comes from today |
|---|---|---|---|
| `ffmpeg` | Remuxing and transcoding for playback; cutting clips and finding pauses for listening | Depends on the build | Docker: apt. Native releases: the user installs it |
| `ffprobe` | Media analysis during scans | Depends on the build | Same as `ffmpeg` |
| `whisper-cli` | Listening: checking a book's chapters and identifying a book by its opening | 2.5 MB | Installed by hand on the live server. Not in any release |
| `ggml-tiny.en.bin` | The speech model `whisper-cli` loads | 78 MB | Same as `whisper-cli` |

`fpcalc` also appears in `services/fingerprint.rs`, but it isn't wired into the service graph. It
stays out of scope until it is.

### What releases contain today

- **Linux:** a tarball holding only the binary.
- **Windows:** a portable zip holding the binary and a README.
- **Docker:** Debian's `ffmpeg` (which includes `ffprobe`), for amd64 and arm64. No Whisper.
- **Windows installer:** `installer/windows/librarian.iss` and `librarian.wxs` exist but aren't part
  of a release. They install FFmpeg through `winget`, so they aren't self-contained.

`docs/distribution-plan.md` allows "one external dependency: `ffprobe`". That is out of date:
playback needs `ffmpeg`, and listening adds Whisper.

## Decision

Ship **one installer, not one executable**. Librarian stays a single binary with its frontend
embedded. Each package places the tools in a `tools/` folder beside the binary.

```text
librarian[.exe]
tools/
  ffmpeg[.exe]
  ffprobe[.exe]
  whisper-cli[.exe]
  ggml-tiny.en.bin
  licenses/
```

| Package | Contents |
|---|---|
| Windows installer | Everything above, with no `winget` step |
| Linux tarball | Everything above |
| Docker image | Debian's `ffmpeg`, plus `whisper-cli` and the model, on amd64 and arm64 |

### Rules

1. **Finding a tool.** Use the configured path if there is one, then `tools/` beside the running
   binary, then `PATH`. All tools use the same resolver, and it never depends on the working
   directory.
2. **A missing tool turns off one feature.** The server still starts, and the UI says what is
   unavailable and why. A check only passes if the tool actually runs successfully.
3. **Whisper stays a separate process.** A crash or hang can't take the server down, and a stuck run
   can be killed.
4. **CI builds `whisper-cli`** for each target, statically linked where the platform allows it, and
   not tuned to the build machine's CPU. A release must not fail with an illegal instruction on an
   older CPU.
5. **The model ships in the package**, so listening works offline. `tiny.en` understands only
   English; other languages would need a larger model as an optional download.
6. **FFmpeg binaries are pinned and mirrored.** BtbN's autobuilds expire, keeping 14 daily builds and
   one build a month for two years. We need our own copy of the exact binary and its source for every
   release.

## Options considered

| Option | Result | Why |
|---|---|---|
| **A. Tools beside the binary, in one installer** | **Chosen** | One mechanism for every tool, no native build inside Cargo, crash isolation, simple to replace a tool |
| B. Embed the tools in the executable and unpack on first run | Not now | A true single file, but about 240 MB, and an executable unpacking executables is more likely to trip antivirus. It uses the same `tools/` layout, so it can be added later without rework |
| C. Compile Whisper into Librarian with `whisper-rs` | Rejected | Adds a C++ build to every Rust target, and a crash could take the server down unless Librarian re-runs itself as a worker. ffmpeg still has to ship, so it doesn't deliver one file |
| D. Download tools on first run | Rejected as the default | Not self-contained, and fails offline. Could later be a "repair" action, with checksums |
| E. Keep asking users to install the tools | Rejected | The current state. Features silently depend on what the host has |

Replacing ffmpeg for listening alone, with a Rust decoder, was also ruled out. Playback still needs
ffmpeg, so it would add code without removing the dependency.

## Consequences

### Size

Roughly **240 MB** for a package using a prebuilt FFmpeg:

| Part | Size |
|---|---|
| Librarian | About 19 MB (compressed 0.2.0 release, likely larger now) |
| FFmpeg | About 142 MiB (BtbN's Windows LGPL static archive, which also holds `ffplay`) |
| `whisper-cli` | 2.5 MB |
| Model | 78 MB |

This is an estimate from those parts and hasn't been measured as a real package. A custom FFmpeg with
only the codecs we need should be much smaller, but at the cost of owning that build and its upkeep.

### Licensing

- Transcoding uses `libx264` (`services/transcode/ffmpeg.rs`), which requires a **GPL** build of
  FFmpeg. An LGPL build would only work if we changed the encoder.
- Shipping a GPL FFmpeg means shipping its licence and the matching source, or hosting the source
  beside the release.
- Running FFmpeg as a separate program is normally treated as keeping Librarian itself outside the
  GPL. This should be checked by someone qualified before distributing it.
- whisper.cpp and the model weights are understood to be MIT licensed. Confirm this and include the
  notices in `tools/licenses/`.
- **The repository has no licence of its own**, according to GitHub. Settle that before distributing.

### Other costs

- The release workflow gains a tool-build job per target, including arm64 for Docker.
- Tool versions become something we track for security updates, because users no longer get them from
  their package manager.
- The Windows installer depends on packaging work that isn't finished, including service mode.

## Fixes needed regardless of packaging

These are defects in the current code, and packaging alone won't solve them:

| Problem | Where | Effect |
|---|---|---|
| Listening runs through `nice` | `services/whisper.rs`: `Whisper::words`, `pauses`, `cut_clip` | Always fails on Windows, which has no `nice` |
| Listening ignores `FFMPEG_PATH` | `services/whisper.rs`: `pauses`, `cut_clip` | A configured FFmpeg is used for playback but not listening |
| `ffprobe` has no override | `services/library_scan.rs`: `check_ffprobe_available` and the analysis call | It can only be found on `PATH` |
| Default Whisper paths are relative | `services/whisper.rs`: `LOCAL_CLI`, `LOCAL_MODEL` | Depend on the working directory. In Docker that is `/app`, not `/data` |
| The availability check is too weak | `services/whisper.rs`: `Whisper::check` | Passes if the process launches, even if it exits with an error, and never checks for ffmpeg |
| The live `whisper-cli` isn't static | `backend/data/models/whisper-cli` | Needs `libgomp` and `libstdc++`, so it can't simply be copied into a release |

## Implementation order

1. Fix the code defects above and add the shared tool resolver.
2. Add Whisper and the model to the Docker image.
3. Add a CI job that builds `whisper-cli`, pins FFmpeg and collects the licences.
4. Add `tools/` to the Linux tarball.
5. Add `tools/` to the Windows zip and remove `winget` from the installer scripts.
6. Update `docs/distribution-plan.md`, `docs/design.md` and the README.

## How we'll know it works

- **Clean machines:** a package runs on a clean Debian or Ubuntu container and a clean Windows VM,
  with no FFmpeg or Whisper installed, and does all of the following:
  - scans a library
  - transcodes a file for playback
  - checks an audiobook's chapters by listening
- **Different working directory:** the same checks pass when started from another folder.
- **Older CPU:** `whisper-cli` runs on a CPU older than the build machine, or under an emulated
  baseline CPU.
- **Broken tools:** deleting or corrupting each tool in turn leaves the server running, with a clear
  reason shown and only that feature off.
- **Overrides:** a configured path takes priority over `tools/`, and `tools/` over `PATH`.
- **Licences:** every package contains the licence files, and the matching FFmpeg source is available
  for that release.

## Open questions

1. **Is about 240 MB acceptable?** If not, build the smaller custom FFmpeg first.
2. **Which licence will Librarian use?** This must be settled before the packages are distributed.
3. **Should the model always be included?** Recommended yes, unless size forces it to be a separate
   download.
4. **Which FFmpeg build?** BtbN's GPL builds are the simplest start. A custom build is smaller but
   ours to maintain.
5. **Is `whisper-rs` worth revisiting later?** Only if a single-file executable becomes a firm
   requirement. Before relying on it, confirm it exposes word-level timestamps and builds cleanly on
   Windows. Its repository moved to Codeberg, and the old GitHub repository is archived.

## Sources

- [BtbN FFmpeg builds](https://github.com/BtbN/FFmpeg-Builds)
- [BtbN latest release and sizes](https://github.com/BtbN/FFmpeg-Builds/releases/tag/latest)
- [FFmpeg licence and legal checklist](https://ffbox0-bg.ffmpeg.org/legal.html)
- [FFmpeg licence file (mirror)](https://github.com/Akemi/FFmpeg/blob/master/LICENSE.md)
- [whisper-rs](https://github.com/tazz4843/whisper-rs)
