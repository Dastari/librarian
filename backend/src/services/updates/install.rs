//! Putting a verified binary in place, restarting into it, and rolling back when it fails.
//!
//! All paths are beside the real executable `<exe>` (symlinks resolved):
//!
//! | file | what it is |
//! | --- | --- |
//! | `<exe>.new` | the verified binary, about to be swapped in |
//! | `<exe>.previous` | the binary that ran before the last upgrade |
//! | `<exe>.failed` | a new binary that did not start, kept for a look |
//! | `<exe>.upgrade.json` | the upgrade marker (below) |
//! | `.librarian-upgrade/` | the download and unpack folder, removed afterwards |
//!
//! **The marker.** Written as `pending` (attempts 0) just before the swap. The new binary, on its
//! first start, raises attempts to 1 and deletes the marker once its HTTP server is listening
//! ("started ok"). A start that finds `pending` with attempts ≥ 1 knows the previous start never got
//! there, so it puts `.previous` back, marks `rolled_back` and restarts into the old binary, which
//! reports what happened. A start that hangs is rolled back the same way by a watchdog after
//! [`READY_DEADLINE`].
//!
//! **The restart.** On Unix the process re-executes itself in place (same PID, same arguments and
//! environment), so it works under any `Restart=` setting and when run by hand. If exec fails, or
//! `LIBRARIAN_UPGRADE_RESTART=exit` is set, it exits with [`EXIT_RESTART`] (75) instead, which a
//! unit with `Restart=on-failure` or `always` restarts. Windows cannot exec: it always exits with 75
//! and needs a service manager (NSSM, WinSW, a scheduled task set to restart) to start it again.

use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use semver::Version;
use serde::{Deserialize, Serialize};

/// The exit code that asks a service manager to start Librarian again (`EX_TEMPFAIL`).
pub const EXIT_RESTART: i32 = 75;
/// A new binary that has not started its HTTP server by now is rolled back.
pub const READY_DEADLINE: Duration = Duration::from_secs(5 * 60);
/// The binary inside an archive is never larger than this.
pub const BINARY_SIZE_CAP: u64 = 1024 * 1024 * 1024;

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(OsString::from).unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

/// The files around one executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPaths {
    pub exe: PathBuf,
}

impl InstallPaths {
    pub fn new(exe: impl Into<PathBuf>) -> Self {
        Self { exe: exe.into() }
    }
    pub fn incoming(&self) -> PathBuf {
        with_suffix(&self.exe, ".new")
    }
    pub fn previous(&self) -> PathBuf {
        with_suffix(&self.exe, ".previous")
    }
    pub fn failed(&self) -> PathBuf {
        with_suffix(&self.exe, ".failed")
    }
    pub fn marker(&self) -> PathBuf {
        with_suffix(&self.exe, ".upgrade.json")
    }
    pub fn staging(&self) -> PathBuf {
        self.exe.with_file_name(".librarian-upgrade")
    }
    pub fn dir(&self) -> &Path {
        self.exe.parent().unwrap_or(Path::new("."))
    }
}

// ===========================================================================
// The marker
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkerState {
    Pending,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Marker {
    pub state: MarkerState,
    pub from: String,
    pub to: String,
    /// Who asked for it (a username).
    pub by: String,
    /// When the swap happened (RFC 3339).
    pub at: String,
    /// Starts of the new binary so far.
    pub attempts: u32,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub backup_snapshot_id: Option<String>,
}

pub fn read_marker(paths: &InstallPaths) -> Option<Marker> {
    let bytes = fs::read(paths.marker()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Write the marker so that a crash mid-write leaves the old one, never half of the new one.
pub fn write_marker(paths: &InstallPaths, marker: &Marker) -> Result<()> {
    let target = paths.marker();
    let temporary = with_suffix(&target, ".tmp");
    {
        let mut file = fs::File::create(&temporary)
            .with_context(|| format!("Could not write {}", temporary.display()))?;
        file.write_all(&serde_json::to_vec_pretty(marker)?)?;
        file.sync_all()?;
    }
    fs::rename(&temporary, &target)
        .with_context(|| format!("Could not write {}", target.display()))?;
    Ok(())
}

pub fn remove_marker(paths: &InstallPaths) {
    let _ = fs::remove_file(paths.marker());
}

/// What a start should do about an upgrade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Startup {
    /// No upgrade in flight.
    Normal,
    /// This is the new binary's first start: count it, and report "started ok" when ready.
    Trial(Marker),
    /// The new binary already had its start and never got ready: put the old one back.
    RollBack(Marker),
    /// This is the old binary after a rollback: report it.
    RolledBack(Marker),
    /// A marker that no longer fits what is running (the swap never happened): report and drop it.
    Stale(Marker),
}

/// Decide from the marker and the running version. Pure.
pub fn startup_decision(marker: Option<Marker>, running: &Version) -> Startup {
    let Some(marker) = marker else {
        return Startup::Normal;
    };
    let running = running.to_string();
    match marker.state {
        MarkerState::RolledBack => Startup::RolledBack(marker),
        MarkerState::Pending if marker.to == running && marker.attempts == 0 => {
            Startup::Trial(marker)
        }
        MarkerState::Pending if marker.to == running => Startup::RollBack(marker),
        MarkerState::Pending => Startup::Stale(marker),
    }
}

// ===========================================================================
// Unpacking
// ===========================================================================

/// Take the one binary out of a release archive (`.tar.gz` or `.zip`) into `dest`. Only a regular
/// file named `name` at the top of the archive counts; nothing else is written anywhere.
pub fn extract_binary(archive: &Path, name: &str, dest: &Path) -> Result<()> {
    let file_name = archive
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        extract_from_tar_gz(archive, name, dest)
    } else if file_name.ends_with(".zip") {
        extract_from_zip(archive, name, dest)
    } else {
        bail!("{file_name} is not an archive Librarian knows how to unpack")
    }
}

fn top_level_name(path: &str) -> Option<&str> {
    let path = path.strip_prefix("./").unwrap_or(path);
    (!path.is_empty() && !path.contains('/') && !path.contains('\\')).then_some(path)
}

fn copy_capped(mut reader: impl Read, dest: &Path, size: u64) -> Result<()> {
    if size == 0 || size > BINARY_SIZE_CAP {
        bail!("the binary in the archive is {size} bytes, outside the limit");
    }
    let mut out =
        fs::File::create(dest).with_context(|| format!("Could not create {}", dest.display()))?;
    let copied = std::io::copy(&mut (&mut reader).take(size), &mut out)?;
    if copied != size {
        bail!("the archive ended early ({copied} of {size} bytes)");
    }
    out.sync_all()?;
    set_executable(dest)?;
    Ok(())
}

/// A small ustar/GNU reader: the release archive holds one file, and pulling in a tar crate for it
/// is not worth it. Long-name (`L`) and pax (`x`) records are honoured for the entry they precede.
fn extract_from_tar_gz(archive: &Path, name: &str, dest: &Path) -> Result<()> {
    let file = fs::File::open(archive)?;
    let mut reader = flate2::read::GzDecoder::new(std::io::BufReader::new(file));
    let mut header = [0u8; 512];
    let mut next_name: Option<String> = None;
    loop {
        if read_block(&mut reader, &mut header)?.is_none() || header.iter().all(|byte| *byte == 0) {
            bail!("the archive has no {name}");
        }
        let size = octal(&header[124..136]).context("the archive has a damaged header")?;
        let padded = size.div_ceil(512) * 512;
        let kind = header[156];
        let entry_name = next_name.take().unwrap_or_else(|| {
            let base = cstr(&header[0..100]);
            if &header[257..262] == b"ustar" {
                let prefix = cstr(&header[345..500]);
                if prefix.is_empty() {
                    base
                } else {
                    format!("{prefix}/{base}")
                }
            } else {
                base
            }
        });
        match kind {
            b'L' | b'x' => {
                if size > 64 * 1024 {
                    bail!("the archive has an oversized header record");
                }
                let mut data = vec![0u8; padded as usize];
                reader.read_exact(&mut data)?;
                data.truncate(size as usize);
                next_name = if kind == b'L' {
                    Some(cstr(&data))
                } else {
                    pax_path(&data)
                };
                continue;
            }
            b'0' | 0 if top_level_name(&entry_name) == Some(name) => {
                return copy_capped(&mut reader, dest, size);
            }
            _ => {
                std::io::copy(&mut (&mut reader).take(padded), &mut std::io::sink())?;
            }
        }
    }
}

fn read_block(reader: &mut impl Read, block: &mut [u8; 512]) -> Result<Option<()>> {
    let mut filled = 0;
    while filled < block.len() {
        let read = reader.read(&mut block[filled..])?;
        if read == 0 {
            return if filled == 0 {
                Ok(None)
            } else {
                bail!("the archive ends in the middle of a header")
            };
        }
        filled += read;
    }
    Ok(Some(()))
}

fn cstr(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

fn octal(field: &[u8]) -> Option<u64> {
    let text = cstr(field);
    let text = text.trim();
    if text.is_empty() {
        return Some(0);
    }
    u64::from_str_radix(text, 8).ok()
}

fn pax_path(data: &[u8]) -> Option<String> {
    String::from_utf8_lossy(data).lines().find_map(|line| {
        let (_, record) = line.split_once(' ')?;
        record.strip_prefix("path=").map(str::to_string)
    })
}

fn extract_from_zip(archive: &Path, name: &str, dest: &Path) -> Result<()> {
    let file = fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file).context("the archive is not a readable zip")?;
    let index = (0..zip.len())
        .find(|index| {
            zip.by_index(*index).is_ok_and(|entry| {
                entry.is_file()
                    && entry
                        .name()
                        .is_ok_and(|entry_name| top_level_name(&entry_name) == Some(name))
            })
        })
        .with_context(|| format!("the archive has no {name}"))?;
    let entry = zip.by_index(index)?;
    let size = entry.size();
    copy_capped(entry, dest, size)
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// Run the verified binary with `--version` (nothing else) and check it reports `expected`. This
/// catches a build for the wrong system or one that cannot load before anything is swapped.
pub async fn preflight(binary: &Path, expected: &Version) -> Result<()> {
    let output = tokio::time::timeout(
        Duration::from_secs(20),
        tokio::process::Command::new(binary)
            .arg("--version")
            .env("LIBRARIAN_HEADLESS", "1")
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("the new binary did not answer --version within 20 seconds")?
    .context("the new binary would not run")?;
    let text = String::from_utf8_lossy(&output.stdout);
    let reported = text.trim();
    let wanted = format!("librarian {expected}");
    if !output.status.success() || reported != wanted {
        bail!(
            "the new binary reported '{}' instead of '{wanted}'",
            reported.chars().take(80).collect::<String>()
        );
    }
    Ok(())
}

// ===========================================================================
// Swapping and rolling back
// ===========================================================================

/// How to keep the running file while replacing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapStyle {
    /// Unix: hard-link the current file as `.previous`, then rename the new one over it, so the
    /// path always holds a whole binary.
    LinkThenReplace,
    /// Windows: a running `.exe` can be renamed but not replaced, so move it aside first.
    RenameAside,
}

impl SwapStyle {
    pub fn for_this_platform() -> Self {
        if cfg!(windows) {
            Self::RenameAside
        } else {
            Self::LinkThenReplace
        }
    }
}

fn keep_copy(from: &Path, to: &Path) -> Result<()> {
    let _ = fs::remove_file(to);
    if fs::hard_link(from, to).is_err() {
        fs::copy(from, to).with_context(|| format!("Could not keep a copy at {}", to.display()))?;
    }
    Ok(())
}

/// Put `paths.incoming()` in place of the executable, keeping the current one as `.previous`.
pub fn swap_in(paths: &InstallPaths, style: SwapStyle) -> Result<()> {
    let incoming = paths.incoming();
    if !incoming.is_file() {
        bail!("{} is missing", incoming.display());
    }
    match style {
        SwapStyle::LinkThenReplace => {
            keep_copy(&paths.exe, &paths.previous())?;
            fs::rename(&incoming, &paths.exe).with_context(|| {
                format!("Could not move the new binary to {}", paths.exe.display())
            })?;
        }
        SwapStyle::RenameAside => {
            let _ = fs::remove_file(paths.previous());
            fs::rename(&paths.exe, paths.previous())
                .context("Could not move the running binary aside")?;
            if let Err(error) = fs::rename(&incoming, &paths.exe) {
                let _ = fs::rename(paths.previous(), &paths.exe);
                return Err(error).context("Could not move the new binary in place");
            }
        }
    }
    sync_dir(paths.dir());
    Ok(())
}

/// Put `.previous` back, keeping the binary that failed as `.failed`.
pub fn roll_back(paths: &InstallPaths, style: SwapStyle) -> Result<()> {
    let previous = paths.previous();
    if !previous.is_file() {
        bail!("there is no {} to go back to", previous.display());
    }
    match style {
        SwapStyle::LinkThenReplace => {
            let _ = keep_copy(&paths.exe, &paths.failed());
            fs::rename(&previous, &paths.exe).context("Could not restore the previous binary")?;
        }
        SwapStyle::RenameAside => {
            let _ = fs::remove_file(paths.failed());
            fs::rename(&paths.exe, paths.failed())
                .context("Could not move the failed binary aside")?;
            if let Err(error) = fs::rename(&previous, &paths.exe) {
                let _ = fs::rename(paths.failed(), &paths.exe);
                return Err(error).context("Could not restore the previous binary");
            }
        }
    }
    sync_dir(paths.dir());
    Ok(())
}

#[cfg(unix)]
fn sync_dir(dir: &Path) {
    if let Ok(handle) = fs::File::open(dir) {
        let _ = handle.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) {}

/// Whether Librarian can write beside its executable, which an upgrade needs.
pub fn dir_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".librarian-write-test-{}", std::process::id()));
    let ok = fs::File::create(&probe).is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

/// The roll-back path a start takes: restore `.previous`, mark the marker `rolled_back` with
/// `reason`, and restart into the restored binary. Returns only if the rollback itself fails.
pub fn roll_back_and_restart(paths: &InstallPaths, mut marker: Marker, reason: &str) -> Result<()> {
    roll_back(paths, SwapStyle::for_this_platform())?;
    marker.state = MarkerState::RolledBack;
    marker.reason = Some(reason.to_string());
    write_marker(paths, &marker)?;
    restart_into(&paths.exe)
}

/// Replace this process with `exe` (Unix), or exit for the service manager to start it (Windows,
/// or `LIBRARIAN_UPGRADE_RESTART=exit`). Never returns.
pub fn restart_into(exe: &Path) -> ! {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let exit_instead = std::env::var("LIBRARIAN_UPGRADE_RESTART")
            .is_ok_and(|value| value.trim().eq_ignore_ascii_case("exit"));
        if !exit_instead {
            // Everything Librarian opened is close-on-exec already; this also covers descriptors a
            // library may have opened without it (Linux 5.11+; ignored where unsupported).
            #[cfg(target_os = "linux")]
            unsafe {
                const CLOSE_RANGE_CLOEXEC: libc::c_uint = 4;
                libc::syscall(
                    libc::SYS_close_range,
                    3 as libc::c_uint,
                    libc::c_uint::MAX,
                    CLOSE_RANGE_CLOEXEC,
                );
            }
            let error = std::process::Command::new(exe)
                .args(std::env::args_os().skip(1))
                .exec();
            eprintln!(
                "Librarian upgrade: could not restart into {}: {error}; exiting with {EXIT_RESTART} for the service manager",
                exe.display()
            );
        }
    }
    #[cfg(not(unix))]
    let _ = exe;
    std::process::exit(EXIT_RESTART)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(state: MarkerState, to: &str, attempts: u32) -> Marker {
        Marker {
            state,
            from: "0.6.0".to_string(),
            to: to.to_string(),
            by: "admin".to_string(),
            at: "2026-11-09T12:00:00Z".to_string(),
            attempts,
            reason: None,
            backup_snapshot_id: None,
        }
    }

    fn setup() -> (tempfile::TempDir, InstallPaths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = InstallPaths::new(dir.path().join("librarian"));
        fs::write(&paths.exe, b"old binary").unwrap();
        (dir, paths)
    }

    #[test]
    fn paths_sit_beside_the_executable() {
        let paths = InstallPaths::new("/opt/v0.6.0/librarian");
        assert_eq!(
            paths.previous(),
            PathBuf::from("/opt/v0.6.0/librarian.previous")
        );
        assert_eq!(paths.incoming(), PathBuf::from("/opt/v0.6.0/librarian.new"));
        assert_eq!(
            paths.marker(),
            PathBuf::from("/opt/v0.6.0/librarian.upgrade.json")
        );
        assert_eq!(
            paths.staging(),
            PathBuf::from("/opt/v0.6.0/.librarian-upgrade")
        );
        let windows = InstallPaths::new("/srv/librarian.exe");
        assert_eq!(
            windows.previous(),
            PathBuf::from("/srv/librarian.exe.previous")
        );
    }

    #[test]
    fn the_start_after_a_swap_is_a_trial_and_the_one_after_that_rolls_back() {
        let new = Version::parse("0.7.0").unwrap();
        let old = Version::parse("0.6.0").unwrap();
        assert_eq!(startup_decision(None, &new), Startup::Normal);
        let pending = marker(MarkerState::Pending, "0.7.0", 0);
        assert_eq!(
            startup_decision(Some(pending.clone()), &new),
            Startup::Trial(pending)
        );
        let tried = marker(MarkerState::Pending, "0.7.0", 1);
        assert_eq!(
            startup_decision(Some(tried.clone()), &new),
            Startup::RollBack(tried)
        );
        // The old binary is still running: the swap never took.
        let stale = marker(MarkerState::Pending, "0.7.0", 0);
        assert_eq!(
            startup_decision(Some(stale.clone()), &old),
            Startup::Stale(stale)
        );
        let rolled = marker(MarkerState::RolledBack, "0.7.0", 1);
        assert_eq!(
            startup_decision(Some(rolled.clone()), &old),
            Startup::RolledBack(rolled)
        );
    }

    #[test]
    fn the_marker_round_trips_and_is_replaced_whole() {
        let (_dir, paths) = setup();
        assert_eq!(read_marker(&paths), None);
        let first = marker(MarkerState::Pending, "0.7.0", 0);
        write_marker(&paths, &first).unwrap();
        assert_eq!(read_marker(&paths), Some(first));
        let second = marker(MarkerState::Pending, "0.7.0", 1);
        write_marker(&paths, &second).unwrap();
        assert_eq!(read_marker(&paths), Some(second));
        assert!(!with_suffix(&paths.marker(), ".tmp").exists());
        remove_marker(&paths);
        assert_eq!(read_marker(&paths), None);
    }

    #[test]
    fn a_swap_keeps_the_old_binary_and_a_rollback_restores_it() {
        for style in [SwapStyle::LinkThenReplace, SwapStyle::RenameAside] {
            let (_dir, paths) = setup();
            fs::write(paths.incoming(), b"new binary").unwrap();
            swap_in(&paths, style).unwrap();
            assert_eq!(fs::read(&paths.exe).unwrap(), b"new binary", "{style:?}");
            assert_eq!(
                fs::read(paths.previous()).unwrap(),
                b"old binary",
                "{style:?}"
            );
            assert!(!paths.incoming().exists());

            roll_back(&paths, style).unwrap();
            assert_eq!(fs::read(&paths.exe).unwrap(), b"old binary", "{style:?}");
            assert_eq!(
                fs::read(paths.failed()).unwrap(),
                b"new binary",
                "{style:?}"
            );
            assert!(!paths.previous().exists());
            // Nothing to go back to a second time.
            assert!(roll_back(&paths, style).is_err());
            assert_eq!(fs::read(&paths.exe).unwrap(), b"old binary");
        }
    }

    #[test]
    fn a_second_upgrade_replaces_the_previous_copy() {
        let (_dir, paths) = setup();
        fs::write(paths.previous(), b"older binary").unwrap();
        fs::write(paths.incoming(), b"new binary").unwrap();
        swap_in(&paths, SwapStyle::LinkThenReplace).unwrap();
        assert_eq!(fs::read(paths.previous()).unwrap(), b"old binary");
    }

    #[test]
    fn a_swap_without_a_verified_file_changes_nothing() {
        let (_dir, paths) = setup();
        assert!(swap_in(&paths, SwapStyle::LinkThenReplace).is_err());
        assert!(swap_in(&paths, SwapStyle::RenameAside).is_err());
        assert_eq!(fs::read(&paths.exe).unwrap(), b"old binary");
        assert!(!paths.previous().exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_swap_through_a_symlinked_folder_writes_beside_the_real_file() {
        let root = tempfile::tempdir().unwrap();
        let real = root.path().join("v0.6.0");
        fs::create_dir(&real).unwrap();
        fs::write(real.join("librarian"), b"old binary").unwrap();
        std::os::unix::fs::symlink(&real, root.path().join("current")).unwrap();
        let exe = fs::canonicalize(root.path().join("current/librarian")).unwrap();
        let paths = InstallPaths::new(exe);
        assert_eq!(paths.dir(), fs::canonicalize(&real).unwrap());
        fs::write(paths.incoming(), b"new binary").unwrap();
        swap_in(&paths, SwapStyle::LinkThenReplace).unwrap();
        assert_eq!(
            fs::read(root.path().join("current/librarian")).unwrap(),
            b"new binary"
        );
        assert!(real.join("librarian.previous").exists());
        assert!(
            fs::symlink_metadata(root.path().join("current"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    fn tar_entry(name: &str, kind: u8, data: &[u8]) -> Vec<u8> {
        let mut header = [0u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..107].copy_from_slice(b"0000755");
        header[124..135].copy_from_slice(format!("{:011o}", data.len()).as_bytes());
        header[156] = kind;
        header[257..263].copy_from_slice(b"ustar\0");
        let mut out = header.to_vec();
        out.extend_from_slice(data);
        out.resize(out.len().div_ceil(512) * 512, 0);
        out
    }

    fn tar_gz(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut tar: Vec<u8> = entries.concat();
        tar.extend_from_slice(&[0u8; 1024]);
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&tar).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn the_binary_comes_out_of_a_tar_gz() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("librarian-0.7.0-linux-x86_64.tar.gz");
        fs::write(
            &archive,
            tar_gz(&[
                tar_entry("README.txt", b'0', b"read me"),
                tar_entry("nested/librarian", b'0', b"not this one"),
                tar_entry("./librarian", b'0', b"\x7fELF the binary"),
            ]),
        )
        .unwrap();
        let dest = dir.path().join("librarian.new");
        extract_binary(&archive, "librarian", &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"\x7fELF the binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&dest).unwrap().permissions().mode() & 0o777,
                0o755
            );
        }
    }

    #[test]
    fn a_gnu_long_name_applies_to_the_next_entry_only() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("x.tar.gz");
        fs::write(
            &archive,
            tar_gz(&[
                tar_entry("././@LongLink", b'L', b"some/deep/path/librarian\0"),
                tar_entry("librarian", b'0', b"wrong: its real name is nested"),
                tar_entry("librarian", b'0', b"right"),
            ]),
        )
        .unwrap();
        let dest = dir.path().join("out");
        extract_binary(&archive, "librarian", &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"right");
    }

    #[test]
    fn an_archive_without_the_binary_or_with_a_link_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("x.tar.gz");
        fs::write(
            &archive,
            tar_gz(&[
                tar_entry("librarian", b'2', b""),
                tar_entry("other", b'0', b"x"),
            ]),
        )
        .unwrap();
        assert!(extract_binary(&archive, "librarian", &dir.path().join("out")).is_err());
        let truncated = dir.path().join("y.tar.gz");
        let mut bytes = tar_gz(&[tar_entry("librarian", b'0', &[1u8; 4096])]);
        bytes.truncate(bytes.len() / 2);
        fs::write(&truncated, bytes).unwrap();
        assert!(extract_binary(&truncated, "librarian", &dir.path().join("out2")).is_err());
        assert!(
            extract_binary(
                &dir.path().join("x.rar"),
                "librarian",
                &dir.path().join("o")
            )
            .is_err()
        );
    }

    #[test]
    fn the_binary_comes_out_of_a_zip() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("librarian-0.7.0-windows-server-x86_64.zip");
        {
            let mut writer = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("README.txt", options).unwrap();
            writer.write_all(b"read me").unwrap();
            writer.start_file("librarian.exe", options).unwrap();
            writer.write_all(b"MZ the binary").unwrap();
            writer.finish().unwrap();
        }
        let dest = dir.path().join("librarian.exe.new");
        extract_binary(&archive, "librarian.exe", &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"MZ the binary");
        assert!(extract_binary(&archive, "librarian", &dir.path().join("o")).is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn preflight_runs_only_version_and_checks_the_answer() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("fake");
        fs::write(
            &script,
            "#!/bin/sh\n[ \"$1\" = --version ] && echo 'librarian 0.7.0'\n",
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        preflight(&script, &Version::parse("0.7.0").unwrap())
            .await
            .unwrap();
        let wrong = preflight(&script, &Version::parse("0.7.1").unwrap())
            .await
            .unwrap_err();
        assert!(wrong.to_string().contains("librarian 0.7.0"), "{wrong}");
        assert!(
            preflight(&dir.path().join("missing"), &Version::new(0, 7, 0))
                .await
                .is_err()
        );
    }
}
