//! Bounded, gated archive extraction for the import pipeline.
//!
//! Scene releases still ship a lot of content inside `.rar` / `.zip` / `.7z`
//! containers. Before this module the importer silently skipped those files:
//! `is_supported_import_extension` returned false, no `MediaFile` row was
//! created and the torrent ended `unmatched` with no explanation.
//!
//! `docs/design.md` ("Archive Extraction") requires every one of these gates
//! before an archive may be unpacked, and all of them are implemented here:
//!
//! 1. **Bounded staging** — output is confined to a caller-supplied staging
//!    directory (`<download dir>/.librarian-extract/<info hash>/<archive>/`).
//!    Every destination path is re-checked with [`safe_destination`] so a
//!    crafted entry cannot escape it.
//! 2. **Entry safety** — absolute paths, `..` traversal, Windows path
//!    prefixes, symlinks, hardlinks and device/special entries are rejected.
//! 3. **Unpacked-byte budget** — enforced both from the archive's declared
//!    sizes (pre-flight) and from bytes actually written (headers can lie).
//! 4. **Entry-count limit** — guards against archive bombs with millions of
//!    tiny entries.
//! 5. **Free-space check** — refuses to start when the target filesystem
//!    cannot hold the declared unpacked size plus a safety margin.
//! 6. **Never deletes the archive** — the torrent must keep seeding.
//! 7. **Idempotent** — a completed extraction drops a marker file; re-running
//!    returns the previous result instead of unpacking again.
//! 8. **Durable, explanatory failures** — [`ExtractError`] always says exactly
//!    what happened (password protected / unsupported / budget exceeded /
//!    corrupt / unsafe entry) so it can be written to
//!    `Torrent.post_process_error` and shown in the UI.
//!
//! No external binaries are used: `zip` and `sevenz-rust2` are pure Rust and
//! `unrar` statically links the vendored RARLAB unrar C++ source.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Directory (relative to the download directory) that holds all staging trees.
pub const STAGING_DIR_NAME: &str = ".librarian-extract";

/// Marker written into a staging directory once extraction fully succeeded.
const MARKER_FILE_NAME: &str = ".librarian-extract.json";

/// Default unpacked-byte budget (`extract.max_unpacked_gb`).
pub const DEFAULT_MAX_UNPACKED_GB: u64 = 60;

/// Default per-archive entry-count limit (`extract.max_entries`).
pub const DEFAULT_MAX_ENTRIES: usize = 20_000;

/// Extra free space we insist on keeping after the extraction completes.
const FREE_SPACE_MARGIN_BYTES: u64 = 1024 * 1024 * 1024;

/// Chunk size used when streaming an entry to disk.
const COPY_CHUNK_BYTES: usize = 128 * 1024;

/// Runtime limits for a single extraction.
#[derive(Debug, Clone, Copy)]
pub struct ExtractionLimits {
    /// Maximum total unpacked bytes for one archive.
    pub max_unpacked_bytes: u64,
    /// Maximum number of entries (files + directories) in one archive.
    pub max_entries: usize,
}

impl Default for ExtractionLimits {
    fn default() -> Self {
        Self {
            max_unpacked_bytes: DEFAULT_MAX_UNPACKED_GB * 1024 * 1024 * 1024,
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }
}

impl ExtractionLimits {
    /// Build limits from `extract.*` app settings, falling back to defaults.
    pub fn from_settings(max_unpacked_gb: Option<u64>, max_entries: Option<usize>) -> Self {
        let default = Self::default();
        Self {
            max_unpacked_bytes: max_unpacked_gb
                .filter(|value| *value > 0)
                .map(|gb| gb.saturating_mul(1024 * 1024 * 1024))
                .unwrap_or(default.max_unpacked_bytes),
            max_entries: max_entries
                .filter(|value| *value > 0)
                .unwrap_or(default.max_entries),
        }
    }
}

/// Archive container formats the importer knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    /// `.zip`
    Zip,
    /// `.rar`, `.r00`…, `.partNN.rar`
    Rar,
    /// `.7z`
    SevenZip,
}

impl ArchiveKind {
    /// Human-readable name used in log lines and durable error messages.
    pub fn label(self) -> &'static str {
        match self {
            Self::Zip => "ZIP",
            Self::Rar => "RAR",
            Self::SevenZip => "7z",
        }
    }
}

/// A single file produced by an extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedFile {
    /// Absolute path inside the staging directory.
    pub path: PathBuf,
    /// Path relative to the staging directory (stable across runs).
    pub relative_path: String,
    /// Size on disk in bytes.
    pub size: u64,
}

/// Result of extracting one archive.
#[derive(Debug, Clone)]
pub struct ExtractionOutcome {
    /// Directory the archive was unpacked into.
    pub staging_dir: PathBuf,
    /// Regular files written (directories are not listed).
    pub files: Vec<ExtractedFile>,
    /// True when a completed extraction was reused instead of re-running.
    pub reused: bool,
}

/// Every way an extraction can fail, phrased so the message can be surfaced
/// verbatim in `Torrent.post_process_error`.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    /// The archive (or one of its entries) needs a password.
    #[error(
        "'{archive}' is password protected ({kind} archive); Librarian never stores archive passwords, extract it manually and re-run the import"
    )]
    PasswordProtected {
        /// Archive file name.
        archive: String,
        /// Container format.
        kind: &'static str,
    },

    /// Container format Librarian cannot unpack.
    #[error("'{archive}' is not a supported archive format ({reason})")]
    Unsupported {
        /// Archive file name.
        archive: String,
        /// Why it is unsupported.
        reason: String,
    },

    /// Declared or written unpacked size exceeded the configured budget.
    #[error(
        "'{archive}' would unpack to at least {needed_bytes} bytes which exceeds the configured extract.max_unpacked_gb budget of {budget_bytes} bytes"
    )]
    BudgetExceeded {
        /// Archive file name.
        archive: String,
        /// Bytes required (declared or already written).
        needed_bytes: u64,
        /// Configured budget in bytes.
        budget_bytes: u64,
    },

    /// Entry count exceeded the configured limit.
    #[error(
        "'{archive}' contains {entries} entries which exceeds the configured extract.max_entries limit of {limit}"
    )]
    TooManyEntries {
        /// Archive file name.
        archive: String,
        /// Entries declared by the archive.
        entries: usize,
        /// Configured limit.
        limit: usize,
    },

    /// An entry could not be written safely inside the staging directory.
    #[error("'{archive}' contains an unsafe entry '{entry}' ({reason}); extraction aborted")]
    UnsafeEntry {
        /// Archive file name.
        archive: String,
        /// Offending entry name as stored in the archive.
        entry: String,
        /// Why the entry was rejected.
        reason: String,
    },

    /// Not enough free disk space to unpack.
    #[error(
        "not enough free space to extract '{archive}': {needed_bytes} bytes required (plus margin) but only {available_bytes} bytes are available on the download volume"
    )]
    InsufficientSpace {
        /// Archive file name.
        archive: String,
        /// Declared unpacked bytes.
        needed_bytes: u64,
        /// Free bytes on the staging filesystem.
        available_bytes: u64,
    },

    /// The archive is damaged or truncated (missing volume, bad CRC, …).
    #[error("'{archive}' could not be read ({kind} archive): {reason}")]
    Corrupt {
        /// Archive file name.
        archive: String,
        /// Container format.
        kind: &'static str,
        /// Underlying error text.
        reason: String,
    },

    /// Filesystem error while writing into the staging directory.
    #[error("failed to write extracted data for '{archive}' into '{path}': {reason}")]
    Io {
        /// Archive file name.
        archive: String,
        /// Path being written.
        path: String,
        /// Underlying error text.
        reason: String,
    },
}

/// Serialized marker written after a successful extraction (idempotency gate).
#[derive(Debug, Serialize, Deserialize)]
struct ExtractionMarker {
    archive: String,
    completed_at: String,
    files: Vec<MarkerFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MarkerFile {
    relative_path: String,
    size: u64,
}

/// Classify a path by extension. Returns `None` for non-archives.
pub fn archive_kind(path: &Path) -> Option<ArchiveKind> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    if name.ends_with(".zip") {
        return Some(ArchiveKind::Zip);
    }
    if name.ends_with(".7z") {
        return Some(ArchiveKind::SevenZip);
    }
    if name.ends_with(".rar") {
        return Some(ArchiveKind::Rar);
    }
    // `.r00`, `.r01`, … continuation volumes of the classic `.rar` + `.rNN` set.
    if let Some((_, ext)) = name.rsplit_once('.')
        && ext.len() >= 3
        && ext.starts_with('r')
        && ext[1..].chars().all(|c| c.is_ascii_digit())
    {
        return Some(ArchiveKind::Rar);
    }
    None
}

/// Convenience wrapper for string paths.
pub fn is_archive_path(path: &str) -> bool {
    archive_kind(Path::new(path)).is_some()
}

/// True when this path is the volume extraction must start from.
///
/// Multi-volume RAR sets must be opened at their first part; the continuation
/// volumes are pulled in automatically by the unrar library. Opening (and
/// therefore staging) each volume separately would extract the same payload
/// many times.
pub fn is_extraction_entry_point(path: &Path) -> bool {
    match archive_kind(path) {
        None => false,
        Some(ArchiveKind::Zip) | Some(ArchiveKind::SevenZip) => true,
        Some(ArchiveKind::Rar) => {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                return false;
            };
            let lower = name.to_ascii_lowercase();
            if let Some(stripped) = lower.strip_suffix(".rar") {
                // `foo.part01.rar` / `foo.part1.rar`: only part 1 is an entry point.
                if let Some(index) = stripped.rfind(".part") {
                    let digits = &stripped[index + 5..];
                    if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                        return digits.parse::<u32>().map(|n| n == 1).unwrap_or(false);
                    }
                }
                return true;
            }
            // `.r00`, `.r01`, … are always continuation volumes of a `.rar`.
            false
        }
    }
}

/// Extract `archive` into `staging_root/<archive stem>/`.
///
/// Runs the blocking extraction on the blocking pool. The archive itself is
/// never modified or deleted (the torrent keeps seeding from it).
pub async fn extract_archive(
    archive: &Path,
    staging_root: &Path,
    limits: ExtractionLimits,
) -> Result<ExtractionOutcome, ExtractError> {
    let archive = archive.to_path_buf();
    let staging_root = staging_root.to_path_buf();
    match tokio::task::spawn_blocking(move || {
        extract_archive_blocking(&archive, &staging_root, limits)
    })
    .await
    {
        Ok(result) => result,
        Err(join_error) => Err(ExtractError::Io {
            archive: "<unknown>".to_string(),
            path: "<staging>".to_string(),
            reason: format!("extraction task panicked: {join_error}"),
        }),
    }
}

/// Blocking implementation of [`extract_archive`].
pub fn extract_archive_blocking(
    archive: &Path,
    staging_root: &Path,
    limits: ExtractionLimits,
) -> Result<ExtractionOutcome, ExtractError> {
    let archive_name = file_name_of(archive);
    let Some(kind) = archive_kind(archive) else {
        return Err(ExtractError::Unsupported {
            archive: archive_name,
            reason: "unrecognized archive extension".to_string(),
        });
    };

    let staging_dir = staging_root.join(staging_subdir_name(archive));

    if let Some(outcome) = reuse_completed_extraction(&staging_dir) {
        debug!(
            archive = %archive.display(),
            staging_dir = %staging_dir.display(),
            files = outcome.files.len(),
            "Reusing previous archive extraction for '{}': staging_dir={}, files={}",
            archive_name,
            staging_dir.display(),
            outcome.files.len()
        );
        return Ok(outcome);
    }

    std::fs::create_dir_all(&staging_dir).map_err(|error| ExtractError::Io {
        archive: archive_name.clone(),
        path: staging_dir.display().to_string(),
        reason: error.to_string(),
    })?;

    info!(
        archive = %archive.display(),
        kind = kind.label(),
        staging_dir = %staging_dir.display(),
        "Extracting {} archive '{}' into staging directory '{}'",
        kind.label(),
        archive_name,
        staging_dir.display()
    );

    let result = match kind {
        ArchiveKind::Zip => extract_zip(archive, &staging_dir, limits),
        ArchiveKind::SevenZip => extract_7z(archive, &staging_dir, limits),
        ArchiveKind::Rar => extract_rar(archive, &staging_dir, limits),
    };

    match result {
        Ok(files) => {
            write_marker(&staging_dir, &archive_name, &files);
            info!(
                archive = %archive.display(),
                staging_dir = %staging_dir.display(),
                files = files.len(),
                bytes = files.iter().map(|f| f.size).sum::<u64>(),
                "Extracted {} file(s) from '{}' into '{}'",
                files.len(),
                archive_name,
                staging_dir.display()
            );
            Ok(ExtractionOutcome {
                staging_dir,
                files,
                reused: false,
            })
        }
        Err(error) => {
            // Partial output is useless and takes space; drop it so a retry
            // starts clean. The archive itself is never touched.
            if let Err(cleanup_error) = std::fs::remove_dir_all(&staging_dir) {
                warn!(
                    archive = %archive.display(),
                    staging_dir = %staging_dir.display(),
                    error = %cleanup_error,
                    "Failed to clean partial extraction staging directory for '{}': staging_dir={}, error={}",
                    archive_name,
                    staging_dir.display(),
                    cleanup_error
                );
            }
            Err(error)
        }
    }
}

/// Remove a staging directory once its files have been imported.
pub async fn cleanup_staging_dir(staging_dir: &Path) {
    let staging_dir = staging_dir.to_path_buf();
    if !staging_dir
        .components()
        .any(|c| c.as_os_str() == STAGING_DIR_NAME)
    {
        warn!(
            staging_dir = %staging_dir.display(),
            "Refusing to clean a directory outside the extraction staging tree: staging_dir={}",
            staging_dir.display()
        );
        return;
    }
    if let Err(error) = tokio::fs::remove_dir_all(&staging_dir).await {
        if error.kind() != std::io::ErrorKind::NotFound {
            warn!(
                staging_dir = %staging_dir.display(),
                error = %error,
                "Failed to remove extraction staging directory: staging_dir={}, error={}",
                staging_dir.display(),
                error
            );
        }
        return;
    }
    debug!(
        staging_dir = %staging_dir.display(),
        "Removed extraction staging directory after import: staging_dir={}",
        staging_dir.display()
    );
}

// ---------------------------------------------------------------------------
// Safety helpers
// ---------------------------------------------------------------------------

/// Validate an archive entry name and map it to a path inside `staging_dir`.
///
/// Rejects absolute paths, `..` traversal, Windows drive/UNC prefixes, empty
/// names, and anything that would resolve outside the staging directory.
pub fn safe_destination(staging_dir: &Path, entry_name: &str) -> Result<PathBuf, String> {
    let normalized = entry_name.replace('\\', "/");
    let trimmed = normalized.trim_matches('/');
    if trimmed.is_empty() {
        return Err("entry has an empty name".to_string());
    }
    if normalized.starts_with('/') {
        return Err("entry uses an absolute path".to_string());
    }
    if normalized.contains('\0') {
        return Err("entry name contains a NUL byte".to_string());
    }

    let candidate = Path::new(trimmed);
    let mut relative = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => {
                let text = part.to_str().ok_or("entry name is not valid UTF-8")?;
                if text == ".." {
                    return Err("entry uses '..' path traversal".to_string());
                }
                relative.push(text);
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err("entry uses '..' path traversal".to_string());
            }
            Component::RootDir => {
                return Err("entry uses an absolute path".to_string());
            }
            Component::Prefix(_) => {
                return Err("entry uses a Windows drive or UNC path prefix".to_string());
            }
        }
    }

    if relative.as_os_str().is_empty() {
        return Err("entry resolves to an empty path".to_string());
    }

    let destination = staging_dir.join(&relative);
    // Belt and braces: the component walk above already guarantees this, but a
    // second check costs nothing and documents the invariant.
    if !destination.starts_with(staging_dir) {
        return Err("entry resolves outside the staging directory".to_string());
    }
    Ok(destination)
}

/// Unix mode bits (when present) that mean "not a plain file or directory".
fn unix_mode_is_special(mode: u32) -> Option<&'static str> {
    const S_IFMT: u32 = 0o170000;
    match mode & S_IFMT {
        0o120000 => Some("entry is a symbolic link"),
        0o140000 => Some("entry is a socket"),
        0o010000 => Some("entry is a FIFO"),
        0o020000 => Some("entry is a character device"),
        0o060000 => Some("entry is a block device"),
        _ => None,
    }
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<archive>")
        .to_string()
}

/// Deterministic per-archive staging subdirectory name.
fn staging_subdir_name(archive: &Path) -> String {
    let name = file_name_of(archive);
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.len() > 120 {
        sanitized[sanitized.len() - 120..].to_string()
    } else {
        sanitized
    }
}

fn reuse_completed_extraction(staging_dir: &Path) -> Option<ExtractionOutcome> {
    let marker_path = staging_dir.join(MARKER_FILE_NAME);
    let raw = std::fs::read_to_string(&marker_path).ok()?;
    let marker: ExtractionMarker = serde_json::from_str(&raw).ok()?;
    let mut files = Vec::with_capacity(marker.files.len());
    for entry in marker.files {
        let path = staging_dir.join(&entry.relative_path);
        if !path.is_file() {
            // Something removed the staged output; redo the extraction.
            return None;
        }
        files.push(ExtractedFile {
            path,
            relative_path: entry.relative_path,
            size: entry.size,
        });
    }
    Some(ExtractionOutcome {
        staging_dir: staging_dir.to_path_buf(),
        files,
        reused: true,
    })
}

fn write_marker(staging_dir: &Path, archive_name: &str, files: &[ExtractedFile]) {
    let marker = ExtractionMarker {
        archive: archive_name.to_string(),
        completed_at: chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
        files: files
            .iter()
            .map(|file| MarkerFile {
                relative_path: file.relative_path.clone(),
                size: file.size,
            })
            .collect(),
    };
    match serde_json::to_string(&marker) {
        Ok(json) => {
            if let Err(error) = std::fs::write(staging_dir.join(MARKER_FILE_NAME), json) {
                warn!(
                    staging_dir = %staging_dir.display(),
                    error = %error,
                    "Failed to write extraction marker for '{}': staging_dir={}, error={}",
                    archive_name,
                    staging_dir.display(),
                    error
                );
            }
        }
        Err(error) => warn!(
            archive = %archive_name,
            error = %error,
            "Failed to serialize extraction marker for '{}': {}",
            archive_name,
            error
        ),
    }
}

/// Free bytes available on the filesystem holding `path` (its nearest
/// existing ancestor). Returns `None` when the platform cannot report it.
pub fn available_space_bytes(path: &Path) -> Option<u64> {
    #[cfg(unix)]
    {
        let mut probe = path.to_path_buf();
        while !probe.exists() {
            match probe.parent() {
                Some(parent) => probe = parent.to_path_buf(),
                None => return None,
            }
        }
        let c_path = std::ffi::CString::new(probe.as_os_str().as_encoded_bytes()).ok()?;
        // SAFETY: `c_path` is a valid NUL-terminated string and `stat` is a
        // zeroed, correctly sized `statvfs` we exclusively own.
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(c_path.as_ptr(), &mut stat) != 0 {
                return None;
            }
            Some((stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64))
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

fn check_preflight(
    archive_name: &str,
    staging_dir: &Path,
    entries: usize,
    declared_bytes: u64,
    limits: ExtractionLimits,
) -> Result<(), ExtractError> {
    if entries > limits.max_entries {
        return Err(ExtractError::TooManyEntries {
            archive: archive_name.to_string(),
            entries,
            limit: limits.max_entries,
        });
    }
    if declared_bytes > limits.max_unpacked_bytes {
        return Err(ExtractError::BudgetExceeded {
            archive: archive_name.to_string(),
            needed_bytes: declared_bytes,
            budget_bytes: limits.max_unpacked_bytes,
        });
    }
    if let Some(available) = available_space_bytes(staging_dir)
        && available < declared_bytes.saturating_add(FREE_SPACE_MARGIN_BYTES)
    {
        return Err(ExtractError::InsufficientSpace {
            archive: archive_name.to_string(),
            needed_bytes: declared_bytes,
            available_bytes: available,
        });
    }
    Ok(())
}

/// Stream `reader` into `destination`, enforcing the remaining byte budget.
///
/// Declared sizes in archive headers cannot be trusted, so the budget is
/// re-checked against bytes actually written.
fn write_entry<R: Read + ?Sized>(
    archive_name: &str,
    destination: &Path,
    reader: &mut R,
    written_total: &mut u64,
    limits: ExtractionLimits,
) -> Result<u64, ExtractError> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|error| ExtractError::Io {
            archive: archive_name.to_string(),
            path: parent.display().to_string(),
            reason: error.to_string(),
        })?;
    }
    let file = File::create(destination).map_err(|error| ExtractError::Io {
        archive: archive_name.to_string(),
        path: destination.display().to_string(),
        reason: error.to_string(),
    })?;
    let mut writer = BufWriter::new(file);
    let mut buffer = vec![0u8; COPY_CHUNK_BYTES];
    let mut entry_bytes = 0u64;
    loop {
        let read = reader.read(&mut buffer).map_err(|error| ExtractError::Io {
            archive: archive_name.to_string(),
            path: destination.display().to_string(),
            reason: error.to_string(),
        })?;
        if read == 0 {
            break;
        }
        *written_total += read as u64;
        entry_bytes += read as u64;
        if *written_total > limits.max_unpacked_bytes {
            return Err(ExtractError::BudgetExceeded {
                archive: archive_name.to_string(),
                needed_bytes: *written_total,
                budget_bytes: limits.max_unpacked_bytes,
            });
        }
        writer
            .write_all(&buffer[..read])
            .map_err(|error| ExtractError::Io {
                archive: archive_name.to_string(),
                path: destination.display().to_string(),
                reason: error.to_string(),
            })?;
    }
    writer.flush().map_err(|error| ExtractError::Io {
        archive: archive_name.to_string(),
        path: destination.display().to_string(),
        reason: error.to_string(),
    })?;
    Ok(entry_bytes)
}

fn relative_of(staging_dir: &Path, path: &Path) -> String {
    path.strip_prefix(staging_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

// ---------------------------------------------------------------------------
// ZIP
// ---------------------------------------------------------------------------

fn extract_zip(
    archive: &Path,
    staging_dir: &Path,
    limits: ExtractionLimits,
) -> Result<Vec<ExtractedFile>, ExtractError> {
    let archive_name = file_name_of(archive);
    let file = File::open(archive).map_err(|error| ExtractError::Io {
        archive: archive_name.clone(),
        path: archive.display().to_string(),
        reason: error.to_string(),
    })?;
    let mut zip =
        zip::ZipArchive::new(BufReader::new(file)).map_err(|error| ExtractError::Corrupt {
            archive: archive_name.clone(),
            kind: "ZIP",
            reason: error.to_string(),
        })?;

    // Pre-flight: entry count, declared sizes, entry safety and encryption.
    let mut declared_bytes = 0u64;
    for index in 0..zip.len() {
        let entry = zip
            .by_index_raw(index)
            .map_err(|error| ExtractError::Corrupt {
                archive: archive_name.clone(),
                kind: "ZIP",
                reason: error.to_string(),
            })?;
        if entry.encrypted() {
            return Err(ExtractError::PasswordProtected {
                archive: archive_name.clone(),
                kind: "ZIP",
            });
        }
        declared_bytes = declared_bytes.saturating_add(entry.size());
    }
    check_preflight(
        &archive_name,
        staging_dir,
        zip.len(),
        declared_bytes,
        limits,
    )?;

    let mut written_total = 0u64;
    let mut files = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|error| ExtractError::Corrupt {
            archive: archive_name.clone(),
            kind: "ZIP",
            reason: error.to_string(),
        })?;
        let raw_name = entry.name().to_string();
        if entry.is_symlink() {
            return Err(ExtractError::UnsafeEntry {
                archive: archive_name.clone(),
                entry: raw_name,
                reason: "entry is a symbolic link".to_string(),
            });
        }
        if let Some(mode) = entry.unix_mode()
            && let Some(reason) = unix_mode_is_special(mode)
        {
            return Err(ExtractError::UnsafeEntry {
                archive: archive_name.clone(),
                entry: raw_name,
                reason: reason.to_string(),
            });
        }
        // `enclosed_name` is zip-rs' own traversal check; `safe_destination`
        // repeats it so all three formats share one rule.
        if entry.enclosed_name().is_none() {
            return Err(ExtractError::UnsafeEntry {
                archive: archive_name.clone(),
                entry: raw_name,
                reason: "entry name escapes the archive root".to_string(),
            });
        }
        let destination = safe_destination(staging_dir, &raw_name).map_err(|reason| {
            ExtractError::UnsafeEntry {
                archive: archive_name.clone(),
                entry: raw_name.clone(),
                reason,
            }
        })?;

        if entry.is_dir() {
            std::fs::create_dir_all(&destination).map_err(|error| ExtractError::Io {
                archive: archive_name.clone(),
                path: destination.display().to_string(),
                reason: error.to_string(),
            })?;
            continue;
        }

        let size = write_entry(
            &archive_name,
            &destination,
            &mut entry,
            &mut written_total,
            limits,
        )?;
        files.push(ExtractedFile {
            relative_path: relative_of(staging_dir, &destination),
            path: destination,
            size,
        });
    }

    Ok(files)
}

// ---------------------------------------------------------------------------
// 7z
// ---------------------------------------------------------------------------

fn extract_7z(
    archive: &Path,
    staging_dir: &Path,
    limits: ExtractionLimits,
) -> Result<Vec<ExtractedFile>, ExtractError> {
    use sevenz_rust2::{ArchiveReader, Error as SevenZError, Password};

    let archive_name = file_name_of(archive);
    let mut reader = ArchiveReader::open(archive, Password::empty())
        .map_err(|error| map_sevenz_error(&archive_name, error))?;

    let entries = reader.archive().files.clone();
    let declared_bytes = entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .fold(0u64, |acc, entry| acc.saturating_add(entry.size));
    check_preflight(
        &archive_name,
        staging_dir,
        entries.len(),
        declared_bytes,
        limits,
    )?;

    // Pre-flight entry safety: fail before writing anything.
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    const FILE_ATTRIBUTE_UNIX_EXTENSION: u32 = 0x8000;
    for entry in &entries {
        if entry.has_windows_attributes {
            if entry.windows_attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Err(ExtractError::UnsafeEntry {
                    archive: archive_name.clone(),
                    entry: entry.name.clone(),
                    reason: "entry is a reparse point / symbolic link".to_string(),
                });
            }
            if entry.windows_attributes & FILE_ATTRIBUTE_UNIX_EXTENSION != 0 {
                let mode = entry.windows_attributes >> 16;
                if let Some(reason) = unix_mode_is_special(mode) {
                    return Err(ExtractError::UnsafeEntry {
                        archive: archive_name.clone(),
                        entry: entry.name.clone(),
                        reason: reason.to_string(),
                    });
                }
            }
        }
        safe_destination(staging_dir, &entry.name).map_err(|reason| ExtractError::UnsafeEntry {
            archive: archive_name.clone(),
            entry: entry.name.clone(),
            reason,
        })?;
    }

    let mut written_total = 0u64;
    let mut files: Vec<ExtractedFile> = Vec::new();
    let mut failure: Option<ExtractError> = None;
    let archive_name_cb = archive_name.clone();
    let result = reader.for_each_entries(|entry, entry_reader| {
        if entry.is_directory {
            let destination = staging_dir.join(entry.name.replace('\\', "/"));
            if let Err(error) = std::fs::create_dir_all(&destination) {
                failure = Some(ExtractError::Io {
                    archive: archive_name_cb.clone(),
                    path: destination.display().to_string(),
                    reason: error.to_string(),
                });
                return Ok(false);
            }
            return Ok(true);
        }
        let destination = match safe_destination(staging_dir, &entry.name) {
            Ok(path) => path,
            Err(reason) => {
                failure = Some(ExtractError::UnsafeEntry {
                    archive: archive_name_cb.clone(),
                    entry: entry.name.clone(),
                    reason,
                });
                return Ok(false);
            }
        };
        match write_entry(
            &archive_name_cb,
            &destination,
            entry_reader,
            &mut written_total,
            limits,
        ) {
            Ok(size) => {
                files.push(ExtractedFile {
                    relative_path: relative_of(staging_dir, &destination),
                    path: destination,
                    size,
                });
                Ok(true)
            }
            Err(error) => {
                failure = Some(error);
                Ok(false)
            }
        }
    });

    if let Some(error) = failure {
        return Err(error);
    }
    result.map_err(|error| map_sevenz_error(&archive_name, error))?;

    Ok(files)
}

fn map_sevenz_error(archive_name: &str, error: sevenz_rust2::Error) -> ExtractError {
    use sevenz_rust2::Error as SevenZError;
    match error {
        SevenZError::PasswordRequired | SevenZError::MaybeBadPassword(_) => {
            ExtractError::PasswordProtected {
                archive: archive_name.to_string(),
                kind: "7z",
            }
        }
        SevenZError::UnsupportedCompressionMethod(method) => ExtractError::Unsupported {
            archive: archive_name.to_string(),
            reason: format!("unsupported 7z compression method '{method}'"),
        },
        SevenZError::ExternalUnsupported => ExtractError::Unsupported {
            archive: archive_name.to_string(),
            reason: "archive uses an external (unsupported) codec".to_string(),
        },
        SevenZError::Unsupported(reason) => ExtractError::Unsupported {
            archive: archive_name.to_string(),
            reason: reason.to_string(),
        },
        other => ExtractError::Corrupt {
            archive: archive_name.to_string(),
            kind: "7z",
            reason: other.to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// RAR
// ---------------------------------------------------------------------------

fn extract_rar(
    archive: &Path,
    staging_dir: &Path,
    limits: ExtractionLimits,
) -> Result<Vec<ExtractedFile>, ExtractError> {
    use unrar::Archive;

    let archive_name = file_name_of(archive);

    // Pass 1 — list: entry count, declared sizes, encryption and entry safety.
    let listing = Archive::new(archive)
        .open_for_listing()
        .map_err(|error| map_unrar_error(&archive_name, &error))?;
    let mut declared_bytes = 0u64;
    let mut entry_count = 0usize;
    let mut planned: BTreeSet<String> = BTreeSet::new();
    for header in listing {
        let header = header.map_err(|error| map_unrar_error(&archive_name, &error))?;
        entry_count += 1;
        let name = header.filename.to_string_lossy().to_string();
        if header.is_encrypted() {
            return Err(ExtractError::PasswordProtected {
                archive: archive_name.clone(),
                kind: "RAR",
            });
        }
        if !header.is_file() && !header.is_directory() {
            return Err(ExtractError::UnsafeEntry {
                archive: archive_name.clone(),
                entry: name,
                reason: "entry is neither a regular file nor a directory".to_string(),
            });
        }
        // On unix the unrar library stores the host mode in the high bits of
        // `file_attr`; reject links, devices, FIFOs and sockets.
        if let Some(reason) = unix_mode_is_special(header.file_attr >> 16) {
            return Err(ExtractError::UnsafeEntry {
                archive: archive_name.clone(),
                entry: name,
                reason: reason.to_string(),
            });
        }
        safe_destination(staging_dir, &name).map_err(|reason| ExtractError::UnsafeEntry {
            archive: archive_name.clone(),
            entry: name.clone(),
            reason,
        })?;
        if header.is_file() {
            declared_bytes = declared_bytes.saturating_add(header.unpacked_size);
            planned.insert(name);
        }
    }
    check_preflight(
        &archive_name,
        staging_dir,
        entry_count,
        declared_bytes,
        limits,
    )?;

    // Pass 2 — process. `extract_to` takes the full destination path, which is
    // what keeps output inside the staging directory (the crate's
    // `extract_with_base` would use the archive's own relative path).
    let mut cursor = Archive::new(archive)
        .open_for_processing()
        .map_err(|error| map_unrar_error(&archive_name, &error))?;
    let mut files = Vec::new();
    let mut written_total = 0u64;
    while let Some(header) = cursor
        .read_header()
        .map_err(|error| map_unrar_error(&archive_name, &error))?
    {
        let entry = header.entry();
        let name = entry.filename.to_string_lossy().to_string();
        if entry.is_directory() {
            cursor = header
                .skip()
                .map_err(|error| map_unrar_error(&archive_name, &error))?;
            continue;
        }
        let destination =
            safe_destination(staging_dir, &name).map_err(|reason| ExtractError::UnsafeEntry {
                archive: archive_name.clone(),
                entry: name.clone(),
                reason,
            })?;
        if written_total.saturating_add(entry.unpacked_size) > limits.max_unpacked_bytes {
            return Err(ExtractError::BudgetExceeded {
                archive: archive_name.clone(),
                needed_bytes: written_total.saturating_add(entry.unpacked_size),
                budget_bytes: limits.max_unpacked_bytes,
            });
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| ExtractError::Io {
                archive: archive_name.clone(),
                path: parent.display().to_string(),
                reason: error.to_string(),
            })?;
        }
        cursor = header
            .extract_to(&destination)
            .map_err(|error| map_unrar_error(&archive_name, &error))?;
        let size = std::fs::metadata(&destination)
            .map(|meta| meta.len())
            .unwrap_or(0);
        written_total = written_total.saturating_add(size);
        if written_total > limits.max_unpacked_bytes {
            return Err(ExtractError::BudgetExceeded {
                archive: archive_name.clone(),
                needed_bytes: written_total,
                budget_bytes: limits.max_unpacked_bytes,
            });
        }
        files.push(ExtractedFile {
            relative_path: relative_of(staging_dir, &destination),
            path: destination,
            size,
        });
    }

    Ok(files)
}

fn map_unrar_error(archive_name: &str, error: &unrar::error::UnrarError) -> ExtractError {
    use unrar::error::Code;
    match error.code {
        Code::MissingPassword | Code::BadPassword => ExtractError::PasswordProtected {
            archive: archive_name.to_string(),
            kind: "RAR",
        },
        Code::BadArchive | Code::BadData | Code::UnknownFormat => ExtractError::Corrupt {
            archive: archive_name.to_string(),
            kind: "RAR",
            reason: error.to_string(),
        },
        Code::EOpen => ExtractError::Corrupt {
            archive: archive_name.to_string(),
            kind: "RAR",
            reason: format!("{error} (a continuation volume may be missing or still downloading)"),
        },
        _ => ExtractError::Corrupt {
            archive: archive_name.to_string(),
            kind: "RAR",
            reason: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn zip_fixture(dir: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
        use zip::write::SimpleFileOptions;
        let path = dir.join("fixture.zip");
        let file = File::create(&path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
        path
    }

    fn sevenz_fixture(dir: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
        use sevenz_rust2::{ArchiveEntry, ArchiveWriter};
        let path = dir.join("fixture.7z");
        let mut writer = ArchiveWriter::create(&path).unwrap();
        for (name, data) in entries {
            writer
                .push_archive_entry(
                    ArchiveEntry::new_file(name),
                    Some(Cursor::new(data.to_vec())),
                )
                .unwrap();
        }
        writer.finish().unwrap();
        path
    }

    #[test]
    fn archive_kind_classifies_supported_extensions() {
        assert_eq!(archive_kind(Path::new("a/b.zip")), Some(ArchiveKind::Zip));
        assert_eq!(
            archive_kind(Path::new("a/b.7z")),
            Some(ArchiveKind::SevenZip)
        );
        assert_eq!(archive_kind(Path::new("a/b.rar")), Some(ArchiveKind::Rar));
        assert_eq!(archive_kind(Path::new("a/b.r00")), Some(ArchiveKind::Rar));
        assert_eq!(archive_kind(Path::new("a/b.R15")), Some(ArchiveKind::Rar));
        assert_eq!(archive_kind(Path::new("a/b.mkv")), None);
        assert_eq!(archive_kind(Path::new("a/b.rarely")), None);
    }

    #[test]
    fn only_first_rar_volume_is_an_entry_point() {
        assert!(is_extraction_entry_point(Path::new("show.rar")));
        assert!(is_extraction_entry_point(Path::new("show.part01.rar")));
        assert!(is_extraction_entry_point(Path::new("show.part1.rar")));
        assert!(!is_extraction_entry_point(Path::new("show.part02.rar")));
        assert!(!is_extraction_entry_point(Path::new("show.r00")));
        assert!(!is_extraction_entry_point(Path::new("show.r01")));
        assert!(is_extraction_entry_point(Path::new("show.zip")));
    }

    #[test]
    fn safe_destination_rejects_traversal_and_absolute_paths() {
        let staging = Path::new("/tmp/staging");
        assert!(safe_destination(staging, "../escape.mkv").is_err());
        assert!(safe_destination(staging, "sub/../../escape.mkv").is_err());
        assert!(safe_destination(staging, "/etc/passwd").is_err());
        assert!(safe_destination(staging, "..\\escape.mkv").is_err());
        assert!(safe_destination(staging, "").is_err());
        assert_eq!(
            safe_destination(staging, "sub/file.mkv").unwrap(),
            staging.join("sub").join("file.mkv")
        );
        assert_eq!(
            safe_destination(staging, "./sub/file.mkv").unwrap(),
            staging.join("sub").join("file.mkv")
        );
    }

    #[test]
    fn unix_mode_special_detection() {
        assert_eq!(unix_mode_is_special(0o100644), None);
        assert_eq!(unix_mode_is_special(0o040755), None);
        assert!(unix_mode_is_special(0o120777).is_some());
        assert!(unix_mode_is_special(0o020666).is_some());
    }

    #[test]
    fn extract_zip_writes_files_and_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let archive = zip_fixture(
            temp.path(),
            &[
                ("Show.S01E01.mkv", b"video-one" as &[u8]),
                ("sub/Show.S01E02.mkv", b"video-two"),
            ],
        );
        let staging_root = temp.path().join(STAGING_DIR_NAME);
        let outcome =
            extract_archive_blocking(&archive, &staging_root, ExtractionLimits::default()).unwrap();
        assert!(!outcome.reused);
        assert_eq!(outcome.files.len(), 2);
        let mut names: Vec<_> = outcome
            .files
            .iter()
            .map(|f| f.relative_path.clone())
            .collect();
        names.sort();
        assert_eq!(names, vec!["Show.S01E01.mkv", "sub/Show.S01E02.mkv"]);
        assert!(outcome.files.iter().all(|f| f.path.is_file()));
        assert!(archive.is_file(), "archive must never be deleted");

        let again =
            extract_archive_blocking(&archive, &staging_root, ExtractionLimits::default()).unwrap();
        assert!(again.reused);
        assert_eq!(again.files.len(), 2);
    }

    #[test]
    fn extract_zip_rejects_traversal_entry() {
        let temp = tempfile::tempdir().unwrap();
        let archive = zip_fixture(temp.path(), &[("../escape.mkv", b"nope" as &[u8])]);
        let staging_root = temp.path().join(STAGING_DIR_NAME);
        let error = extract_archive_blocking(&archive, &staging_root, ExtractionLimits::default())
            .unwrap_err();
        assert!(
            matches!(error, ExtractError::UnsafeEntry { .. }),
            "expected UnsafeEntry, got {error:?}"
        );
        assert!(!temp.path().join("escape.mkv").exists());
    }

    #[test]
    fn extract_zip_enforces_unpacked_budget() {
        let temp = tempfile::tempdir().unwrap();
        let archive = zip_fixture(temp.path(), &[("big.mkv", &vec![7u8; 64 * 1024])]);
        let staging_root = temp.path().join(STAGING_DIR_NAME);
        let limits = ExtractionLimits {
            max_unpacked_bytes: 1024,
            max_entries: 10,
        };
        let error = extract_archive_blocking(&archive, &staging_root, limits).unwrap_err();
        assert!(
            matches!(error, ExtractError::BudgetExceeded { .. }),
            "expected BudgetExceeded, got {error:?}"
        );
    }

    #[test]
    fn extract_zip_enforces_entry_limit() {
        let temp = tempfile::tempdir().unwrap();
        let archive = zip_fixture(
            temp.path(),
            &[("a.mkv", b"a" as &[u8]), ("b.mkv", b"b"), ("c.mkv", b"c")],
        );
        let staging_root = temp.path().join(STAGING_DIR_NAME);
        let limits = ExtractionLimits {
            max_unpacked_bytes: 1024 * 1024,
            max_entries: 2,
        };
        let error = extract_archive_blocking(&archive, &staging_root, limits).unwrap_err();
        assert!(
            matches!(error, ExtractError::TooManyEntries { .. }),
            "expected TooManyEntries, got {error:?}"
        );
    }

    #[test]
    fn extract_7z_writes_files() {
        let temp = tempfile::tempdir().unwrap();
        let archive = sevenz_fixture(
            temp.path(),
            &[
                ("Movie.2020.mkv", b"movie-bytes" as &[u8]),
                ("extras/note.txt", b"hello"),
            ],
        );
        let staging_root = temp.path().join(STAGING_DIR_NAME);
        let outcome =
            extract_archive_blocking(&archive, &staging_root, ExtractionLimits::default()).unwrap();
        assert_eq!(outcome.files.len(), 2);
        let movie = outcome
            .files
            .iter()
            .find(|f| f.relative_path == "Movie.2020.mkv")
            .expect("movie entry extracted");
        assert_eq!(std::fs::read(&movie.path).unwrap(), b"movie-bytes");
        assert!(archive.is_file(), "archive must never be deleted");
    }

    #[test]
    fn extract_7z_rejects_traversal_entry() {
        let temp = tempfile::tempdir().unwrap();
        let archive = sevenz_fixture(temp.path(), &[("../escape.mkv", b"nope" as &[u8])]);
        let staging_root = temp.path().join(STAGING_DIR_NAME);
        let error = extract_archive_blocking(&archive, &staging_root, ExtractionLimits::default())
            .unwrap_err();
        assert!(
            matches!(error, ExtractError::UnsafeEntry { .. }),
            "expected UnsafeEntry, got {error:?}"
        );
    }

    #[test]
    fn unsupported_extension_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("release.tar.gz");
        std::fs::write(&archive, b"not-an-archive").unwrap();
        let error = extract_archive_blocking(
            &archive,
            &temp.path().join(STAGING_DIR_NAME),
            ExtractionLimits::default(),
        )
        .unwrap_err();
        assert!(matches!(error, ExtractError::Unsupported { .. }));
    }

    #[test]
    fn corrupt_archive_reports_corrupt() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("broken.zip");
        std::fs::write(&archive, b"PK\x03\x04 definitely not a zip").unwrap();
        let error = extract_archive_blocking(
            &archive,
            &temp.path().join(STAGING_DIR_NAME),
            ExtractionLimits::default(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ExtractError::Corrupt { .. }),
            "expected Corrupt, got {error:?}"
        );
    }

    // RAR fixtures cannot be produced by any Rust crate (the `unrar` crate is
    // decompress-only and no pure-Rust RAR compressor exists), and no `rar`
    // binary is installed in this environment. `backend/tests/fixtures/rar/`
    // therefore carries the MIT-licensed test archives shipped with the
    // `unrar` crate itself.
    fn rar_fixture(name: &str) -> Option<PathBuf> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/rar")
            .join(name);
        path.is_file().then_some(path)
    }

    #[test]
    fn extract_rar_writes_files() {
        let Some(archive) = rar_fixture("version.rar") else {
            eprintln!("skipping: tests/fixtures/rar/version.rar is not present");
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let staging_root = temp.path().join(STAGING_DIR_NAME);
        let outcome =
            extract_archive_blocking(&archive, &staging_root, ExtractionLimits::default()).unwrap();
        assert!(!outcome.files.is_empty());
        assert!(outcome.files.iter().all(|f| f.path.is_file()));
        assert!(
            outcome
                .files
                .iter()
                .all(|f| f.path.starts_with(&staging_root)),
            "all output must stay inside the staging root"
        );
        assert!(archive.is_file(), "archive must never be deleted");
    }

    #[test]
    fn extract_rar_reports_password_protected() {
        let Some(archive) = rar_fixture("crypted.rar") else {
            eprintln!("skipping: tests/fixtures/rar/crypted.rar is not present");
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let error = extract_archive_blocking(
            &archive,
            &temp.path().join(STAGING_DIR_NAME),
            ExtractionLimits::default(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ExtractError::PasswordProtected { .. }),
            "expected PasswordProtected, got {error:?}"
        );
    }

    /// The multi-volume fixture ships without its continuation volumes, which
    /// is exactly the state a still-downloading / incomplete release is in.
    /// Extraction must fail with an explanatory durable error rather than
    /// silently producing a truncated file.
    #[test]
    fn extract_rar_multipart_reports_missing_volume() {
        let Some(archive) = rar_fixture("archive.part1.rar") else {
            eprintln!("skipping: tests/fixtures/rar/archive.part1.rar is not present");
            return;
        };
        assert!(is_extraction_entry_point(&archive));
        assert!(!is_extraction_entry_point(Path::new("archive.part2.rar")));
        let temp = tempfile::tempdir().unwrap();
        let error = extract_archive_blocking(
            &archive,
            &temp.path().join(STAGING_DIR_NAME),
            ExtractionLimits::default(),
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(
            matches!(error, ExtractError::Corrupt { .. }),
            "expected Corrupt, got {error:?}"
        );
        assert!(
            message.contains("continuation volume"),
            "error must explain the missing volume, got: {message}"
        );
    }
}
