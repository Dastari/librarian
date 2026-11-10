//! What this server is (its version, how it was built, where it runs) and how release versions
//! compare.
//!
//! - **The version.** A release build made by `.github/workflows/release.yml` carries its tag
//!   (`LIBRARIAN_RELEASE_TAG`, read at compile time), so `v0.6.1-dev.2` reports `0.6.1-dev.2`. Any
//!   other build reports the Cargo version, which never has a pre-release part.
//! - **The build kind.** `release`, `debug`, `docker`, `dev` (`LIBRARIAN_DEV` on a release build) or
//!   `source` (a release-profile build without the web app embedded, or one run from a cargo
//!   `target/` folder). Only `release` may replace itself.
//! - **Ordering.** Semver rules: `0.6.0-dev.1 < 0.6.0-dev.2 < 0.6.0-rc.1 < 0.6.0 < 0.6.1-dev.1`.

use std::path::{Path, PathBuf};

use semver::Version;

/// The tag the release workflow built this binary from, when it did.
pub const RELEASE_TAG: Option<&str> = option_env!("LIBRARIAN_RELEASE_TAG");

/// Parse a release tag (`v0.6.0`, `v0.6.0-dev.1`, `0.6.0`) as a version. Build metadata is ignored.
pub fn parse_tag(tag: &str) -> Option<Version> {
    let text = tag.trim();
    let text = text.strip_prefix('v').unwrap_or(text);
    let mut version = Version::parse(text).ok()?;
    version.build = semver::BuildMetadata::EMPTY;
    Some(version)
}

/// The version this binary reports: its release tag when it has one, else the Cargo version.
pub fn running_version() -> Version {
    RELEASE_TAG
        .and_then(parse_tag)
        .or_else(|| parse_tag(env!("CARGO_PKG_VERSION")))
        .unwrap_or_else(|| Version::new(0, 0, 0))
}

/// Which releases an install follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Plain `v*` releases only.
    Stable,
    /// Pre-releases (`v*-*`) as well.
    Dev,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Dev => "dev",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().trim_matches('"').to_ascii_lowercase().as_str() {
            "stable" => Some(Self::Stable),
            "dev" => Some(Self::Dev),
            _ => None,
        }
    }

    /// Stable, unless this server already runs a pre-release: then it follows those.
    pub fn default_for(running: &Version) -> Self {
        if running.pre.is_empty() {
            Self::Stable
        } else {
            Self::Dev
        }
    }

    /// Whether a release belongs on this channel. Stable takes a plain version that GitHub does
    /// not flag as a pre-release either.
    pub fn includes(self, version: &Version, flagged_prerelease: bool) -> bool {
        match self {
            Self::Dev => true,
            Self::Stable => version.pre.is_empty() && !flagged_prerelease,
        }
    }
}

/// How this binary was built and where it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildKind {
    Release,
    Debug,
    Docker,
    Dev,
    Source,
}

impl BuildKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::Debug => "debug",
            Self::Docker => "docker",
            Self::Dev => "dev",
            Self::Source => "source",
        }
    }
}

/// Everything that decides the build kind and whether self-upgrade is allowed, gathered once so the
/// decision is a pure function.
#[derive(Debug, Clone)]
pub struct BuildFacts {
    pub debug_assertions: bool,
    pub dev_env: bool,
    pub in_docker: bool,
    pub embeds_web: bool,
    /// The real file the process runs from, symlinks resolved.
    pub exe_path: Option<PathBuf>,
    pub os: &'static str,
    pub arch: &'static str,
    /// The terminal dashboard owns the terminal; a restart would leave it in a mess.
    pub tui: bool,
}

impl BuildFacts {
    /// The facts for this process.
    pub fn current(tui: bool) -> Self {
        Self {
            debug_assertions: cfg!(debug_assertions),
            dev_env: crate::config::env_bool("LIBRARIAN_DEV", false),
            in_docker: in_docker(),
            embeds_web: cfg!(feature = "embed-frontend"),
            exe_path: current_exe(),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            tui,
        }
    }

    pub fn kind(&self) -> BuildKind {
        if self.in_docker {
            BuildKind::Docker
        } else if self.debug_assertions {
            BuildKind::Debug
        } else if self.dev_env {
            BuildKind::Dev
        } else if !self.embeds_web || self.exe_path.as_deref().is_some_and(in_cargo_target) {
            BuildKind::Source
        } else {
            BuildKind::Release
        }
    }

    /// `linux-x86_64`, `windows-x86_64`, …
    pub fn platform(&self) -> String {
        format!("{}-{}", self.os, self.arch)
    }
}

/// Why this install cannot replace itself, and what to do instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocked {
    pub reason: String,
    pub instead: String,
}

/// `None` when this install may replace its own binary.
pub fn self_upgrade_block(facts: &BuildFacts) -> Option<Blocked> {
    let blocked = |reason: &str, instead: &str| {
        Some(Blocked {
            reason: reason.to_string(),
            instead: instead.to_string(),
        })
    };
    match facts.kind() {
        BuildKind::Docker => {
            return blocked(
                "Librarian is running in a container, which is replaced, not upgraded in place.",
                "Pull the new image and recreate the container: docker compose pull && docker compose up -d.",
            );
        }
        BuildKind::Debug => {
            return blocked(
                "This is a debug build from a source checkout.",
                "Update the checkout (git pull) and rebuild, or run a release binary instead.",
            );
        }
        BuildKind::Dev => {
            return blocked(
                "Dev mode is on (LIBRARIAN_DEV is set).",
                "Unset LIBRARIAN_DEV and restart to let a release binary upgrade itself.",
            );
        }
        BuildKind::Source => {
            return blocked(
                "This binary was built from source (no embedded web app, or it runs from a cargo target folder).",
                "Update the checkout (git pull) and rebuild with make build.",
            );
        }
        BuildKind::Release => {}
    }
    if super::github::platform_asset_pattern(facts.os, facts.arch).is_none() {
        return blocked(
            &format!("No release build is published for {}.", facts.platform()),
            "Build Librarian from source for this platform.",
        );
    }
    if facts.tui {
        return blocked(
            "Librarian is running with the terminal dashboard.",
            "Run it headless (as a service, or with LIBRARIAN_HEADLESS=1) to let it upgrade itself.",
        );
    }
    if facts.exe_path.is_none() {
        return blocked(
            "Librarian can't tell which file it is running from.",
            "Download the release and replace the binary by hand.",
        );
    }
    None
}

/// The file this process runs from, with every symlink resolved, so the new binary is written next
/// to the real file (`current -> v0.6.0/` writes into `v0.6.0/`).
pub fn current_exe() -> Option<PathBuf> {
    let path = std::env::current_exe().ok()?;
    Some(std::fs::canonicalize(&path).unwrap_or(path))
}

fn in_cargo_target(path: &Path) -> bool {
    let parts: Vec<_> = path
        .components()
        .map(|part| part.as_os_str().to_string_lossy().to_string())
        .collect();
    parts.windows(2).any(|pair| {
        pair[0] == "target" && (pair[1] == "release" || pair[1] == "debug" || pair[1].contains('-'))
    })
}

/// Docker leaves `/.dockerenv`, Podman `/run/.containerenv`, and the project image sets
/// `LIBRARIAN_IN_DOCKER`. An LXC system container has none of these and counts as a host.
fn in_docker() -> bool {
    crate::config::env_bool("LIBRARIAN_IN_DOCKER", false)
        || Path::new("/.dockerenv").exists()
        || Path::new("/run/.containerenv").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        parse_tag(text).unwrap()
    }

    fn release_facts() -> BuildFacts {
        BuildFacts {
            debug_assertions: false,
            dev_env: false,
            in_docker: false,
            embeds_web: true,
            exe_path: Some(PathBuf::from("/opt/librarian/librarian")),
            os: "linux",
            arch: "x86_64",
            tui: false,
        }
    }

    #[test]
    fn tags_parse_with_or_without_the_v_and_ignore_build_metadata() {
        assert_eq!(v("v0.6.0"), Version::new(0, 6, 0));
        assert_eq!(v("0.6.0"), Version::new(0, 6, 0));
        assert_eq!(v("v0.6.0-dev.1").pre.as_str(), "dev.1");
        assert_eq!(v("v0.6.0+abc"), Version::new(0, 6, 0));
        assert!(parse_tag("tv-v1.0.0").is_none());
        assert!(parse_tag("latest").is_none());
    }

    #[test]
    fn pre_releases_order_by_semver_rules() {
        assert!(v("0.6.0-dev.1") < v("0.6.0-dev.2"));
        assert!(v("0.6.0-dev.2") < v("0.6.0-dev.10"));
        assert!(v("0.6.0-dev.10") < v("0.6.0-rc.1"));
        assert!(v("0.6.0-rc.1") < v("0.6.0"));
        assert!(v("0.6.0") < v("0.6.1-dev.1"));
        assert!(v("0.5.9") < v("0.6.0-dev.1"));
    }

    #[test]
    fn the_default_channel_follows_the_running_version() {
        assert_eq!(Channel::default_for(&v("0.6.0")), Channel::Stable);
        assert_eq!(Channel::default_for(&v("0.6.0-dev.1")), Channel::Dev);
        assert_eq!(Channel::parse("\"dev\""), Some(Channel::Dev));
        assert_eq!(Channel::parse("Stable"), Some(Channel::Stable));
        assert_eq!(Channel::parse("nightly"), None);
    }

    #[test]
    fn stable_takes_only_plain_unflagged_releases() {
        assert!(Channel::Stable.includes(&v("0.6.0"), false));
        assert!(!Channel::Stable.includes(&v("0.6.0-dev.1"), true));
        assert!(!Channel::Stable.includes(&v("0.6.0-dev.1"), false));
        // A plain tag that its maintainer flagged as a pre-release stays off Stable.
        assert!(!Channel::Stable.includes(&v("0.3.2"), true));
        assert!(Channel::Dev.includes(&v("0.6.0-dev.1"), true));
        assert!(Channel::Dev.includes(&v("0.6.0"), false));
    }

    #[test]
    fn a_published_release_binary_may_upgrade_itself() {
        let facts = release_facts();
        assert_eq!(facts.kind(), BuildKind::Release);
        assert_eq!(self_upgrade_block(&facts), None);
        let windows = BuildFacts {
            os: "windows",
            exe_path: Some(PathBuf::from(r"C:\Librarian\librarian.exe")),
            ..release_facts()
        };
        assert_eq!(self_upgrade_block(&windows), None);
    }

    #[test]
    fn debug_dev_docker_and_source_builds_refuse_and_say_what_to_do() {
        let debug = BuildFacts {
            debug_assertions: true,
            ..release_facts()
        };
        assert_eq!(debug.kind(), BuildKind::Debug);
        let blocked = self_upgrade_block(&debug).unwrap();
        assert!(blocked.instead.contains("git pull"), "{blocked:?}");

        let dev = BuildFacts {
            dev_env: true,
            ..release_facts()
        };
        assert_eq!(dev.kind(), BuildKind::Dev);
        assert!(
            self_upgrade_block(&dev)
                .unwrap()
                .reason
                .contains("LIBRARIAN_DEV")
        );

        // Docker wins over everything else: even a debug build in a container is replaced.
        let docker = BuildFacts {
            in_docker: true,
            debug_assertions: true,
            ..release_facts()
        };
        assert_eq!(docker.kind(), BuildKind::Docker);
        assert!(
            self_upgrade_block(&docker)
                .unwrap()
                .instead
                .contains("docker compose pull")
        );

        let no_web = BuildFacts {
            embeds_web: false,
            ..release_facts()
        };
        assert_eq!(no_web.kind(), BuildKind::Source);
        let from_target = BuildFacts {
            exe_path: Some(PathBuf::from(
                "/root/librarian/backend/target/release/librarian",
            )),
            ..release_facts()
        };
        assert_eq!(from_target.kind(), BuildKind::Source);
        assert!(self_upgrade_block(&from_target).is_some());
    }

    #[test]
    fn unsupported_platforms_and_the_dashboard_refuse() {
        let arm = BuildFacts {
            arch: "aarch64",
            ..release_facts()
        };
        assert!(
            self_upgrade_block(&arm)
                .unwrap()
                .reason
                .contains("linux-aarch64")
        );
        let mac = BuildFacts {
            os: "macos",
            ..release_facts()
        };
        assert!(self_upgrade_block(&mac).is_some());
        let tui = BuildFacts {
            tui: true,
            ..release_facts()
        };
        assert!(
            self_upgrade_block(&tui)
                .unwrap()
                .reason
                .contains("dashboard")
        );
    }
}
