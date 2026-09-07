//! Quality profiles + upgrade automation (`docs/tier1-features-plan.md` §2).
//!
//! - [`scoring`]: release title parsing (`ParsedRelease`), fuzzy title
//!   similarity, and the Q25 `is_upgrade` comparison. Shared by auto-download
//!   candidate ranking, torrent-import duplicate handling (Q44), and profile
//!   evaluation.
//! - [`profile`]: `QualityProfile` evaluation (Q23/Q26) and profile
//!   resolution (per-entity override > `Library.quality_profile_id` > seeded
//!   default).
//!
//! This replaces the phase-A inline scorer that used to live at
//! `jobs::scoring` (deleted — see the "Implemented" note in
//! `docs/tier1-features-plan.md` §2).

pub mod profile;
pub mod scoring;
