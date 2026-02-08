//! Torznab category definitions and mappings
//!
//! Standard Torznab categories follow the Newznab numbering scheme.
//! Main categories are in thousands (1000, 2000, etc.) and subcategories
//! add tens (2010, 2020, etc.).

use serde::{Deserialize, Serialize};

/// A mapping from tracker-specific category to Torznab standard category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryMapping {
    /// The tracker's internal category ID (as string for flexibility)
    pub tracker_id: String,
    /// The Torznab standard category ID
    pub torznab_cat: i32,
    /// Description of the category
    pub description: Option<String>,
}

impl CategoryMapping {
    pub fn new(tracker_id: impl Into<String>, torznab_cat: i32, desc: impl Into<String>) -> Self {
        Self {
            tracker_id: tracker_id.into(),
            torznab_cat,
            description: Some(desc.into()),
        }
    }
}

/// Common category constants for easy reference
pub mod cats {
    // Main categories
    pub const CONSOLE: i32 = 1000;
    pub const MOVIES: i32 = 2000;
    pub const AUDIO: i32 = 3000;
    pub const PC: i32 = 4000;
    pub const TV: i32 = 5000;
    pub const BOOKS: i32 = 7000;
    pub const OTHER: i32 = 8000;

    // Movies subcategories
    pub const MOVIES_FOREIGN: i32 = 2010;
    pub const MOVIES_SD: i32 = 2030;
    pub const MOVIES_HD: i32 = 2040;
    pub const MOVIES_UHD: i32 = 2045;
    pub const MOVIES_BLURAY: i32 = 2050;
    pub const MOVIES_3D: i32 = 2060;
    pub const MOVIES_DVD: i32 = 2070;
    pub const MOVIES_WEBDL: i32 = 2080;

    // TV subcategories
    pub const TV_WEBDL: i32 = 5010;
    pub const TV_FOREIGN: i32 = 5020;
    pub const TV_SD: i32 = 5030;
    pub const TV_HD: i32 = 5040;
    pub const TV_SPORT: i32 = 5060;
    pub const TV_ANIME: i32 = 5070;
    pub const TV_DOCUMENTARY: i32 = 5080;

    // Audio subcategories
    pub const AUDIO_MP3: i32 = 3010;
    pub const AUDIO_VIDEO: i32 = 3020;
    pub const AUDIO_AUDIOBOOK: i32 = 3030;
    pub const AUDIO_LOSSLESS: i32 = 3040;
    pub const AUDIO_OTHER: i32 = 3050;

    // Books subcategories
    pub const BOOKS_MAGS: i32 = 7010;
    pub const BOOKS_COMICS: i32 = 7030;
    pub const BOOKS_OTHER: i32 = 7050;

    // PC subcategories
    pub const PC_0DAY: i32 = 4010;
    pub const PC_MAC: i32 = 4030;
    pub const PC_MOBILE_OTHER: i32 = 4040;
    pub const PC_GAMES: i32 = 4050;

    // Console subcategories
    pub const CONSOLE_OTHER: i32 = 1090;
    pub const CONSOLE_PS4: i32 = 1150;
    pub const CONSOLE_WII: i32 = 1030;
    pub const CONSOLE_XBOX: i32 = 1040;
}
