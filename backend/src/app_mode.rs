//! Application run modes for platform-specific behavior.

use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Server,
    Tray,
    Service,
}

impl RunMode {
    pub fn from_env() -> Self {
        match env::var("RUN_MODE").ok().as_deref() {
            Some("tray") => RunMode::Tray,
            Some("service") => RunMode::Service,
            Some("server") => RunMode::Server,
            _ => RunMode::Server,
        }
    }

    pub fn from_arg(value: &str) -> Option<Self> {
        match value {
            "tray" => Some(RunMode::Tray),
            "service" => Some(RunMode::Service),
            "server" => Some(RunMode::Server),
            _ => None,
        }
    }
}
