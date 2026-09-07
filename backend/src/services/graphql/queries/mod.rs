//! GraphQL query modules (non-entity, domain-specific).

pub mod backup;
pub mod content_status;
pub mod filesystem;
pub mod quality;

pub use backup::BackupQueries;
pub use content_status::ContentStatusQueries;
pub use filesystem::FilesystemQueries;
pub use quality::QualityQueries;
