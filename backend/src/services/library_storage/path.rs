//! Where a library file lives: a plain absolute path on this machine, or an `smb://` URI.
//!
//! The database keeps a full path string for every media file, folder and library root. Local
//! paths stay exactly as they are today (`/data/media/Movies/Film (2020)/Film.mkv`); a file on a
//! network share is `smb://host[:port]/share/dir/file.mkv`. [`StoragePath`] is that string with
//! the lexical operations the pipeline needs (join, parent, file name, extension, relative to a
//! root). It never touches the disk.
//!
//! A URI never carries a password. A user name may appear (`smb://toby@host/share`); the secret
//! lives in the encrypted credential store (see `docs/design.md` Q85).

use std::path::{Path, PathBuf};

use super::location::SmbLocation;

/// A library file or folder on some storage, as the string kept in the database.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StoragePath(String);

/// Which kind of storage a path points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StorageKind {
    /// A path on this machine's filesystem (a mount counts as local).
    Local,
    /// A folder on an SMB share, reached without an OS mount.
    Smb,
}

impl StorageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            StorageKind::Local => "local",
            StorageKind::Smb => "smb",
        }
    }
}

pub(crate) const SMB_SCHEME: &str = "smb://";

impl StoragePath {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }

    pub fn kind(&self) -> StorageKind {
        if is_smb_uri(&self.0) {
            StorageKind::Smb
        } else {
            StorageKind::Local
        }
    }

    pub fn is_local(&self) -> bool {
        self.kind() == StorageKind::Local
    }

    /// The path on this machine's filesystem, for a local path.
    pub fn as_local(&self) -> Option<&Path> {
        self.is_local().then(|| Path::new(&self.0))
    }

    /// The share and path inside it, for an `smb://` URI.
    pub fn as_smb(&self) -> Option<SmbLocation> {
        SmbLocation::parse(&self.0).ok()
    }

    /// This path with one more component. `name` must be a single component.
    pub fn join(&self, name: &str) -> StoragePath {
        match self.kind() {
            StorageKind::Local => {
                StoragePath(Path::new(&self.0).join(name).to_string_lossy().into_owned())
            }
            StorageKind::Smb => match self.as_smb() {
                Some(location) => StoragePath(location.join(name).to_uri()),
                None => StoragePath(format!("{}/{}", self.0.trim_end_matches('/'), name)),
            },
        }
    }

    /// The containing folder, or `None` at the root of a filesystem or share.
    pub fn parent(&self) -> Option<StoragePath> {
        match self.kind() {
            StorageKind::Local => Path::new(&self.0)
                .parent()
                .map(|parent| StoragePath(parent.to_string_lossy().into_owned())),
            StorageKind::Smb => self
                .as_smb()?
                .parent()
                .map(|location| StoragePath(location.to_uri())),
        }
    }

    /// The last component, or `None` at a root.
    pub fn file_name(&self) -> Option<String> {
        match self.kind() {
            StorageKind::Local => Path::new(&self.0)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()),
            StorageKind::Smb => self.as_smb()?.file_name().map(str::to_owned),
        }
    }

    /// The text after the last dot of the file name, without the dot.
    pub fn extension(&self) -> Option<String> {
        match self.kind() {
            StorageKind::Local => Path::new(&self.0)
                .extension()
                .map(|extension| extension.to_string_lossy().into_owned()),
            StorageKind::Smb => {
                let name = self.file_name()?;
                let (stem, extension) = name.rsplit_once('.')?;
                (!stem.is_empty() && !extension.is_empty()).then(|| extension.to_owned())
            }
        }
    }

    /// Whether `root` is this path or one of its ancestors (whole components, not text).
    pub fn starts_with(&self, root: &StoragePath) -> bool {
        self.relative_to(root).is_some()
    }

    /// The part below `root`, `/`-separated, or `None` when this path is not under it.
    /// A path relative to itself is the empty string.
    pub fn relative_to(&self, root: &StoragePath) -> Option<String> {
        if self.kind() != root.kind() {
            return None;
        }
        match self.kind() {
            StorageKind::Local => Path::new(&self.0)
                .strip_prefix(&root.0)
                .ok()
                .map(|relative| relative.to_string_lossy().into_owned()),
            StorageKind::Smb => {
                let (this, root) = (self.as_smb()?, root.as_smb()?);
                this.relative_to(&root)
            }
        }
    }
}

impl std::fmt::Display for StoragePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for StoragePath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Whether a stored path string is an SMB URI rather than a local path.
pub fn is_smb_uri(raw: &str) -> bool {
    raw.get(..SMB_SCHEME.len())
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case(SMB_SCHEME))
}

/// Anything the pipeline already holds a path in.
pub trait IntoStoragePath {
    fn into_storage_path(self) -> StoragePath;
}

impl IntoStoragePath for StoragePath {
    fn into_storage_path(self) -> StoragePath {
        self
    }
}
impl IntoStoragePath for &StoragePath {
    fn into_storage_path(self) -> StoragePath {
        self.clone()
    }
}
impl IntoStoragePath for &str {
    fn into_storage_path(self) -> StoragePath {
        StoragePath::new(self)
    }
}
impl IntoStoragePath for String {
    fn into_storage_path(self) -> StoragePath {
        StoragePath(self)
    }
}
impl IntoStoragePath for &String {
    fn into_storage_path(self) -> StoragePath {
        StoragePath::new(self.as_str())
    }
}
impl IntoStoragePath for &Path {
    fn into_storage_path(self) -> StoragePath {
        StoragePath(self.to_string_lossy().into_owned())
    }
}
impl IntoStoragePath for PathBuf {
    fn into_storage_path(self) -> StoragePath {
        StoragePath(self.to_string_lossy().into_owned())
    }
}
impl IntoStoragePath for &PathBuf {
    fn into_storage_path(self) -> StoragePath {
        StoragePath(self.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_paths_behave_like_std_paths() {
        let root = StoragePath::new("/data/media/Movies");
        let file = root.join("Film (2020)").join("Film.mkv");
        assert_eq!(file.as_str(), "/data/media/Movies/Film (2020)/Film.mkv");
        assert_eq!(file.file_name().as_deref(), Some("Film.mkv"));
        assert_eq!(file.extension().as_deref(), Some("mkv"));
        assert_eq!(
            file.parent().unwrap().as_str(),
            "/data/media/Movies/Film (2020)"
        );
        assert_eq!(
            file.relative_to(&root).as_deref(),
            Some("Film (2020)/Film.mkv")
        );
        assert!(file.starts_with(&root));
        assert!(!StoragePath::new("/data/media/Movies2/x.mkv").starts_with(&root));
        assert!(file.is_local());
        assert_eq!(file.as_local(), Some(Path::new(file.as_str())));
    }

    #[test]
    fn smb_paths_use_slashes_and_stay_inside_the_share() {
        let root = StoragePath::new("smb://truenas.localdomain/media/Movies");
        assert_eq!(root.kind(), StorageKind::Smb);
        let file = root.join("Film (2020)").join("Film.mkv");
        assert_eq!(
            file.as_str(),
            "smb://truenas.localdomain/media/Movies/Film (2020)/Film.mkv"
        );
        assert_eq!(file.file_name().as_deref(), Some("Film.mkv"));
        assert_eq!(file.extension().as_deref(), Some("mkv"));
        assert_eq!(
            file.parent().unwrap().as_str(),
            "smb://truenas.localdomain/media/Movies/Film (2020)"
        );
        assert_eq!(
            file.relative_to(&root).as_deref(),
            Some("Film (2020)/Film.mkv")
        );
        assert_eq!(root.relative_to(&root).as_deref(), Some(""));
        // The share itself has no parent: a path never climbs out of it.
        let share = StoragePath::new("smb://truenas.localdomain/media");
        assert!(share.parent().is_none());
        assert!(share.file_name().is_none());
        assert!(file.as_local().is_none());
    }

    #[test]
    fn local_and_remote_paths_are_never_under_each_other() {
        let local = StoragePath::new("/data/media");
        let remote = StoragePath::new("smb://host/media");
        assert!(!remote.starts_with(&local));
        assert!(!local.starts_with(&remote));
    }

    #[test]
    fn host_and_share_names_compare_without_case() {
        let root = StoragePath::new("smb://TrueNAS.localdomain/Media/Movies");
        let file = StoragePath::new("smb://truenas.localdomain/media/Movies/a.mkv");
        assert_eq!(file.relative_to(&root).as_deref(), Some("a.mkv"));
        // The folders inside a share are case sensitive on this server.
        let other = StoragePath::new("smb://truenas.localdomain/media/movies/a.mkv");
        assert!(other.relative_to(&root).is_none());
    }
}
