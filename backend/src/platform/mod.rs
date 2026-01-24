//! Platform-specific helpers.

#[cfg(windows)]
pub mod windows;

#[cfg(not(windows))]
pub mod windows;
