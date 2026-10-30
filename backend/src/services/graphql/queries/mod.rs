//! GraphQL query modules (non-entity, domain-specific).

pub mod backup;
pub mod content_status;
pub mod filesystem;
pub mod genres;
pub mod resume;
pub mod source_capabilities;

pub use backup::BackupQueries;
pub use content_status::ContentStatusQueries;
pub use filesystem::FilesystemQueries;
pub use genres::GenreQueries;
pub use resume::ResumeQueries;
pub use source_capabilities::SourceCapabilityQueries;
