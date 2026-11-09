//! Quality profiles + upgrade automation (`docs/tier1-features-plan.md` §2).
//!
//! - [`scoring`]: release title parsing (`ParsedRelease`), fuzzy title
//!   similarity, and the Q25 `is_upgrade` comparison. Shared by auto-download
//!   candidate ranking, torrent-import duplicate handling (Q44), and profile
//!   evaluation.
//! - [`title_match`]: whether a release names the wanted show, movie, album or book, word for
//!   word (the hard gate before any quality check; Q98).
//! - [`profile`]: `QualityProfile` evaluation (Q23/Q26) and profile
//!   resolution (per-entity override > `Library.quality_profile_id` > seeded
//!   default).
//!
//! This replaces the phase-A inline scorer that used to live at
//! `jobs::scoring` (deleted — see the "Implemented" note in
//! `docs/tier1-features-plan.md` §2).

pub mod profile;
pub mod recompute;
pub mod scoring;
pub mod title_match;
