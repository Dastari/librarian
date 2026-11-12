//! A library root, as a person types it: a folder on this machine, or a folder on an SMB share.
//!
//! `/data/media/Movies` is local. `smb://truenas.localdomain/media/Movies` is the folder `Movies`
//! on the share `media` of the host `truenas.localdomain` (port 445 unless `:port` is given).
//! Host and share are stored in lower case so the same share is always the same string; folder
//! names keep their case. A URI carries no user name or password: the sign-in belongs to the
//! share and lives in the encrypted credential store, found through [`CredentialRef`].

use std::path::PathBuf;

use super::path::{SMB_SCHEME, StorageKind, StoragePath, is_smb_uri};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LocationError {
    #[error("The location is empty")]
    Empty,
    #[error("'{0}' is not an absolute path or an smb:// address")]
    NotAbsolute(String),
    #[error("An smb:// address needs a host and a share: smb://host/share/folder")]
    MissingShare,
    #[error("'{0}' is not a valid SMB host name")]
    InvalidHost(String),
    #[error("'{0}' is not a valid SMB port")]
    InvalidPort(String),
    #[error("An smb:// address must not contain a user name or password; save the sign-in instead")]
    CredentialsInUri,
    #[error("'{0}' is not allowed in a library folder path")]
    InvalidComponent(String),
}

/// Where the sign-in for a share is kept. One sign-in per share: `smb://host[:port]/share`.
///
/// It is the key the encrypted network-path credential store already uses (the digest of this
/// text), so a share's credentials are saved once and every library on that share finds them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CredentialRef(String);

impl CredentialRef {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CredentialRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A folder (or file) on an SMB share.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SmbLocation {
    /// Lower-case host name or address.
    pub host: String,
    /// `None` is the standard port, 445.
    pub port: Option<u16>,
    /// Lower-case share name.
    pub share: String,
    /// `/`-separated path inside the share, with no leading or trailing slash. Empty is the share
    /// root.
    pub path: String,
}

impl SmbLocation {
    pub fn parse(raw: &str) -> Result<Self, LocationError> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(LocationError::Empty);
        }
        if !is_smb_uri(raw) {
            return Err(LocationError::NotAbsolute(raw.to_string()));
        }
        let rest = &raw[SMB_SCHEME.len()..];
        let (authority, tail) = match rest.split_once('/') {
            Some((authority, tail)) => (authority, tail),
            None => return Err(LocationError::MissingShare),
        };
        if authority.contains('@') {
            return Err(LocationError::CredentialsInUri);
        }
        let (host, port) = match authority.rsplit_once(':') {
            // An IPv6 literal is `[::1]` and carries colons of its own.
            Some((host, port)) if !port.contains(']') => {
                let parsed = port
                    .parse::<u16>()
                    .ok()
                    .filter(|port| *port != 0)
                    .ok_or_else(|| LocationError::InvalidPort(port.to_string()))?;
                (host, Some(parsed))
            }
            _ => (authority, None),
        };
        let host = host.trim();
        if host.is_empty() || host.chars().any(|c| c.is_whitespace() || c == '\\') {
            return Err(LocationError::InvalidHost(host.to_string()));
        }
        let mut components = tail.split('/').filter(|part| !part.is_empty());
        let share = components.next().ok_or(LocationError::MissingShare)?;
        if share.contains('\\') || share.chars().any(char::is_control) {
            return Err(LocationError::InvalidComponent(share.to_string()));
        }
        let mut folders = Vec::new();
        for component in components {
            validate_component(component)?;
            folders.push(component);
        }
        Ok(Self {
            host: host.to_ascii_lowercase(),
            port: port.filter(|port| *port != 445),
            share: share.to_ascii_lowercase(),
            path: folders.join("/"),
        })
    }

    /// `smb://host[:port]/share[/path]`.
    pub fn to_uri(&self) -> String {
        let mut uri = format!("{SMB_SCHEME}{}", self.host);
        if let Some(port) = self.port {
            uri.push_str(&format!(":{port}"));
        }
        uri.push('/');
        uri.push_str(&self.share);
        if !self.path.is_empty() {
            uri.push('/');
            uri.push_str(&self.path);
        }
        uri
    }

    /// The share's own root.
    pub fn share_root(&self) -> SmbLocation {
        SmbLocation {
            path: String::new(),
            ..self.clone()
        }
    }

    /// Where this share's sign-in is kept.
    pub fn credential_ref(&self) -> CredentialRef {
        CredentialRef(self.share_root().to_uri())
    }

    /// `host:port` to connect to.
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port.unwrap_or(445))
    }

    /// One more component below this location. Separators in `name` are dropped, and `.` and `..`
    /// never climb: a join stays inside the share.
    pub fn join(&self, name: &str) -> SmbLocation {
        let mut path = self.path.clone();
        for part in name.split(['/', '\\']) {
            if part.is_empty() || part == "." || part == ".." {
                continue;
            }
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
        }
        SmbLocation {
            path,
            ..self.clone()
        }
    }

    pub fn parent(&self) -> Option<SmbLocation> {
        if self.path.is_empty() {
            return None;
        }
        let path = self
            .path
            .rsplit_once('/')
            .map(|(parent, _)| parent.to_string())
            .unwrap_or_default();
        Some(SmbLocation {
            path,
            ..self.clone()
        })
    }

    pub fn file_name(&self) -> Option<&str> {
        if self.path.is_empty() {
            return None;
        }
        Some(self.path.rsplit('/').next().unwrap_or(&self.path))
    }

    /// The path below `root` on the same share, `/`-separated.
    pub fn relative_to(&self, root: &SmbLocation) -> Option<String> {
        if self.host != root.host || self.port != root.port || self.share != root.share {
            return None;
        }
        if root.path.is_empty() {
            return Some(self.path.clone());
        }
        if self.path == root.path {
            return Some(String::new());
        }
        self.path
            .strip_prefix(&root.path)
            .and_then(|rest| rest.strip_prefix('/'))
            .map(str::to_string)
    }

    pub fn storage_path(&self) -> StoragePath {
        StoragePath::new(self.to_uri())
    }
}

fn validate_component(component: &str) -> Result<(), LocationError> {
    if component == "." || component == ".." || component.chars().any(char::is_control) {
        return Err(LocationError::InvalidComponent(component.to_string()));
    }
    Ok(())
}

/// A library root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageLocation {
    Local(PathBuf),
    Smb(SmbLocation),
}

impl StorageLocation {
    /// A local absolute path or an `smb://` address. A relative path is refused: a library root
    /// must not depend on the working directory.
    pub fn parse(raw: &str) -> Result<Self, LocationError> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(LocationError::Empty);
        }
        if is_smb_uri(raw) {
            return SmbLocation::parse(raw).map(StorageLocation::Smb);
        }
        let path = PathBuf::from(raw);
        if !path.is_absolute() {
            return Err(LocationError::NotAbsolute(raw.to_string()));
        }
        Ok(StorageLocation::Local(path))
    }

    pub fn kind(&self) -> StorageKind {
        match self {
            StorageLocation::Local(_) => StorageKind::Local,
            StorageLocation::Smb(_) => StorageKind::Smb,
        }
    }

    /// The string kept in `Library.path` and as the prefix of every `MediaFile.path`.
    pub fn to_path_string(&self) -> String {
        match self {
            StorageLocation::Local(path) => path.to_string_lossy().into_owned(),
            StorageLocation::Smb(location) => location.to_uri(),
        }
    }

    pub fn root_path(&self) -> StoragePath {
        StoragePath::new(self.to_path_string())
    }

    /// Where the sign-in is kept, for a location that needs one.
    pub fn credential_ref(&self) -> Option<CredentialRef> {
        match self {
            StorageLocation::Local(_) => None,
            StorageLocation::Smb(location) => Some(location.credential_ref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_roots_must_be_absolute() {
        assert_eq!(
            StorageLocation::parse("/data/media").unwrap(),
            StorageLocation::Local(PathBuf::from("/data/media"))
        );
        assert_eq!(
            StorageLocation::parse("media/Movies"),
            Err(LocationError::NotAbsolute("media/Movies".into()))
        );
        assert_eq!(StorageLocation::parse("  "), Err(LocationError::Empty));
    }

    #[test]
    fn smb_roots_parse_and_normalise() {
        let location = SmbLocation::parse("smb://TrueNAS.localdomain/Media//Movies/").unwrap();
        assert_eq!(location.host, "truenas.localdomain");
        assert_eq!(location.share, "media");
        assert_eq!(location.path, "Movies");
        assert_eq!(location.port, None);
        assert_eq!(location.to_uri(), "smb://truenas.localdomain/media/Movies");
        assert_eq!(location.address(), "truenas.localdomain:445");
    }

    #[test]
    fn standard_port_is_dropped_and_other_ports_kept() {
        assert_eq!(
            SmbLocation::parse("smb://host:445/share").unwrap().to_uri(),
            "smb://host/share"
        );
        let location = SmbLocation::parse("smb://10.0.1.190:1445/media/TV").unwrap();
        assert_eq!(location.to_uri(), "smb://10.0.1.190:1445/media/TV");
        assert_eq!(location.address(), "10.0.1.190:1445");
        assert!(matches!(
            SmbLocation::parse("smb://host:0/share"),
            Err(LocationError::InvalidPort(_))
        ));
        assert!(matches!(
            SmbLocation::parse("smb://host:abc/share"),
            Err(LocationError::InvalidPort(_))
        ));
    }

    #[test]
    fn a_share_is_required() {
        assert_eq!(
            SmbLocation::parse("smb://host"),
            Err(LocationError::MissingShare)
        );
        assert_eq!(
            SmbLocation::parse("smb://host/"),
            Err(LocationError::MissingShare)
        );
    }

    #[test]
    fn credentials_never_ride_in_the_uri() {
        assert_eq!(
            SmbLocation::parse("smb://toby:secret@host/share"),
            Err(LocationError::CredentialsInUri)
        );
        assert_eq!(
            SmbLocation::parse("smb://toby@host/share"),
            Err(LocationError::CredentialsInUri)
        );
    }

    #[test]
    fn parent_directory_components_are_refused() {
        assert!(matches!(
            SmbLocation::parse("smb://host/share/../other"),
            Err(LocationError::InvalidComponent(_))
        ));
        let root = SmbLocation::parse("smb://host/share/Movies").unwrap();
        assert_eq!(root.join("../../x").to_uri(), "smb://host/share/Movies/x");
        assert_eq!(root.join("a\\b").to_uri(), "smb://host/share/Movies/a/b");
    }

    #[test]
    fn libraries_on_one_share_share_one_credential() {
        let movies = StorageLocation::parse("smb://truenas.localdomain/media/Movies").unwrap();
        let music = StorageLocation::parse("smb://TRUENAS.localdomain/media/Music").unwrap();
        assert_eq!(movies.credential_ref(), music.credential_ref());
        assert_eq!(
            movies.credential_ref().unwrap().as_str(),
            "smb://truenas.localdomain/media"
        );
        let other = StorageLocation::parse("smb://truenas.localdomain/backups").unwrap();
        assert_ne!(movies.credential_ref(), other.credential_ref());
        assert!(
            StorageLocation::parse("/data/media")
                .unwrap()
                .credential_ref()
                .is_none()
        );
    }

    #[test]
    fn relative_paths_stop_at_the_share() {
        let root = SmbLocation::parse("smb://host/share/Movies").unwrap();
        let file = SmbLocation::parse("smb://host/share/Movies/A/b.mkv").unwrap();
        assert_eq!(file.relative_to(&root).as_deref(), Some("A/b.mkv"));
        let sibling = SmbLocation::parse("smb://host/share/Movies2/b.mkv").unwrap();
        assert_eq!(sibling.relative_to(&root), None);
        let other_share = SmbLocation::parse("smb://host/other/Movies/b.mkv").unwrap();
        assert_eq!(other_share.relative_to(&root), None);
        assert_eq!(
            file.parent().unwrap().parent().unwrap().to_uri(),
            "smb://host/share/Movies"
        );
    }
}
