//! Librarian's versioned backup identity preserves two distinct pinned ORM contracts.
//!
//! The manifest SHA-256 covers this whole opaque identity. The physical component is
//! the introspected SchemaModel fingerprint used by migration source checks; the
//! descriptor component retains logical types, field mappings and backup policies.
//! The pinned backup orchestrator compares the complete string before restore,
//! including dry runs. Neither component is a substitute for the other.
use anyhow::{Result, bail, ensure};

const IDENTITY_VERSION: &str = "librarian-schema-v1";

pub(super) fn canonical_backend(backend: &str) -> Result<&'static str> {
    match backend {
        // The pinned ORM uses Debug for DatabaseBackend; the backup adapter's
        // documented backend identifier and Librarian's verifier use lowercase.
        "Sqlite" | "sqlite" => Ok("sqlite"),
        _ => bail!("Unsupported Librarian backup backend {backend:?}; expected sqlite"),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct SchemaIdentity<'a> {
    pub physical_schema_hash: &'a str,
    pub descriptor_schema_hash: &'a str,
}

impl<'a> SchemaIdentity<'a> {
    pub fn new(physical_schema_hash: &'a str, descriptor_schema_hash: &'a str) -> Self {
        Self {
            physical_schema_hash,
            descriptor_schema_hash,
        }
    }

    pub fn encode(&self) -> String {
        format!(
            "{IDENTITY_VERSION}:{}:{}",
            self.physical_schema_hash, self.descriptor_schema_hash
        )
    }

    pub fn parse(identity: &'a str) -> Result<Self> {
        let mut parts = identity.split(':');
        ensure!(
            parts.next() == Some(IDENTITY_VERSION),
            "Backup has no supported physical/descriptor schema identity; create a new full backup with a qualified source-schema recovery build"
        );
        let physical = parts.next().unwrap_or_default();
        let descriptor = parts.next().unwrap_or_default();
        // Both pinned functions produce lowercase, sixteen-digit FNV-1a64
        // fingerprints. A future format/algorithm needs a distinct version.
        let valid = |hash: &str| {
            hash.len() == 16
                && hash
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        ensure!(
            parts.next().is_none() && valid(physical) && valid(descriptor),
            "Malformed versioned Librarian backup schema identity"
        );
        Ok(Self::new(physical, descriptor))
    }
}
