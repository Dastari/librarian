//! Background job logic for content acquisition (auto-download loop).
//!
//! `auto_download` holds candidate discovery + grab logic and ranks/filters
//! release candidates using `services::quality::scoring`/`profile` (the
//! phase-A inline scorer that used to live here as `jobs::scoring` has been
//! deleted — see the "Implemented" note in `docs/tier1-features-plan.md` §2).
//! `download_monitor` is the long-running `Service` (`AutoDownloadService`)
//! that runs `auto_download::run_once` on a timer plus a retry/reconciliation
//! sweep.
//!
//! See `docs/tier1-features-plan.md` §1/§2 and `docs/design.md`'s Media
//! Pipeline Decision Guide for the design this implements.

pub mod auto_download;
pub mod download_monitor;

pub use download_monitor::AutoDownloadServiceConfig;

pub mod schedule_sync;
