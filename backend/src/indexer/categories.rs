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

/// A Torznab category definition
#[derive(Debug, Clone)]
pub struct TorznabCategory {
    pub id: i32,
    pub name: &'static str,
    pub parent_id: Option<i32>,
}

impl TorznabCategory {
    pub const fn new(id: i32, name: &'static str, parent_id: Option<i32>) -> Self {
        Self {
            id,
            name,
            parent_id,
        }
    }

    /// Check if this is a parent category
    pub fn is_parent(&self) -> bool {
        self.parent_id.is_none()
    }
}

/// Standard Torznab categories (based on Newznab spec)
pub static TORZNAB_CATEGORIES: &[TorznabCategory] = &[
    // Console (1000)
    TorznabCategory::new(1000, "Console", None),
    TorznabCategory::new(1010, "Console/NDS", Some(1000)),
    TorznabCategory::new(1020, "Console/PSP", Some(1000)),
    TorznabCategory::new(1030, "Console/Wii", Some(1000)),
    TorznabCategory::new(1040, "Console/Xbox", Some(1000)),
    TorznabCategory::new(1050, "Console/Xbox 360", Some(1000)),
    TorznabCategory::new(1060, "Console/WiiWare", Some(1000)),
    TorznabCategory::new(1070, "Console/Xbox 360 DLC", Some(1000)),
    TorznabCategory::new(1080, "Console/PS3", Some(1000)),
    TorznabCategory::new(1090, "Console/Other", Some(1000)),
    TorznabCategory::new(1110, "Console/3DS", Some(1000)),
    TorznabCategory::new(1120, "Console/PS Vita", Some(1000)),
    TorznabCategory::new(1130, "Console/WiiU", Some(1000)),
    TorznabCategory::new(1140, "Console/Xbox One", Some(1000)),
    TorznabCategory::new(1150, "Console/PS4", Some(1000)),
    TorznabCategory::new(1180, "Console/Switch", Some(1000)),
    // Movies (2000)
    TorznabCategory::new(2000, "Movies", None),
    TorznabCategory::new(2010, "Movies/Foreign", Some(2000)),
    TorznabCategory::new(2020, "Movies/Other", Some(2000)),
    TorznabCategory::new(2030, "Movies/SD", Some(2000)),
    TorznabCategory::new(2040, "Movies/HD", Some(2000)),
    TorznabCategory::new(2045, "Movies/UHD", Some(2000)),
    TorznabCategory::new(2050, "Movies/BluRay", Some(2000)),
    TorznabCategory::new(2060, "Movies/3D", Some(2000)),
    TorznabCategory::new(2070, "Movies/DVD", Some(2000)),
    TorznabCategory::new(2080, "Movies/WEB-DL", Some(2000)),
    // Audio (3000)
    TorznabCategory::new(3000, "Audio", None),
    TorznabCategory::new(3010, "Audio/MP3", Some(3000)),
    TorznabCategory::new(3020, "Audio/Video", Some(3000)),
    TorznabCategory::new(3030, "Audio/Audiobook", Some(3000)),
    TorznabCategory::new(3040, "Audio/Lossless", Some(3000)),
    TorznabCategory::new(3050, "Audio/Other", Some(3000)),
    TorznabCategory::new(3060, "Audio/Foreign", Some(3000)),
    // PC (4000)
    TorznabCategory::new(4000, "PC", None),
    TorznabCategory::new(4010, "PC/0day", Some(4000)),
    TorznabCategory::new(4020, "PC/ISO", Some(4000)),
    TorznabCategory::new(4030, "PC/Mac", Some(4000)),
    TorznabCategory::new(4040, "PC/Mobile-Other", Some(4000)),
    TorznabCategory::new(4050, "PC/Games", Some(4000)),
    TorznabCategory::new(4060, "PC/Mobile-iOS", Some(4000)),
    TorznabCategory::new(4070, "PC/Mobile-Android", Some(4000)),
    // TV (5000)
    TorznabCategory::new(5000, "TV", None),
    TorznabCategory::new(5010, "TV/WEB-DL", Some(5000)),
    TorznabCategory::new(5020, "TV/Foreign", Some(5000)),
    TorznabCategory::new(5030, "TV/SD", Some(5000)),
    TorznabCategory::new(5040, "TV/HD", Some(5000)),
    TorznabCategory::new(5045, "TV/UHD", Some(5000)),
    TorznabCategory::new(5050, "TV/Other", Some(5000)),
    TorznabCategory::new(5060, "TV/Sport", Some(5000)),
    TorznabCategory::new(5070, "TV/Anime", Some(5000)),
    TorznabCategory::new(5080, "TV/Documentary", Some(5000)),
    // XXX (6000) - Adult content
    TorznabCategory::new(6000, "XXX", None),
    TorznabCategory::new(6010, "XXX/DVD", Some(6000)),
    TorznabCategory::new(6020, "XXX/WMV", Some(6000)),
    TorznabCategory::new(6030, "XXX/XviD", Some(6000)),
    TorznabCategory::new(6040, "XXX/x264", Some(6000)),
    TorznabCategory::new(6050, "XXX/Pack", Some(6000)),
    TorznabCategory::new(6060, "XXX/ImageSet", Some(6000)),
    TorznabCategory::new(6070, "XXX/Other", Some(6000)),
    TorznabCategory::new(6080, "XXX/SD", Some(6000)),
    TorznabCategory::new(6090, "XXX/WEB-DL", Some(6000)),
    // Books (7000)
    TorznabCategory::new(7000, "Books", None),
    TorznabCategory::new(7010, "Books/Mags", Some(7000)),
    TorznabCategory::new(7020, "Books/EBook", Some(7000)),
    TorznabCategory::new(7030, "Books/Comics", Some(7000)),
    TorznabCategory::new(7040, "Books/Technical", Some(7000)),
    TorznabCategory::new(7050, "Books/Other", Some(7000)),
    TorznabCategory::new(7060, "Books/Foreign", Some(7000)),
    // Other (8000)
    TorznabCategory::new(8000, "Other", None),
    TorznabCategory::new(8010, "Other/Misc", Some(8000)),
    TorznabCategory::new(8020, "Other/Hashed", Some(8000)),
];

/// Common category constants for easy reference
pub mod cats {
    // Main categories
    pub const CONSOLE: i32 = 1000;
    pub const MOVIES: i32 = 2000;
    pub const AUDIO: i32 = 3000;
    pub const PC: i32 = 4000;
    pub const TV: i32 = 5000;
    pub const XXX: i32 = 6000;
    pub const BOOKS: i32 = 7000;
    pub const OTHER: i32 = 8000;

    // Movies subcategories
    pub const MOVIES_FOREIGN: i32 = 2010;
    pub const MOVIES_OTHER: i32 = 2020;
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
    pub const TV_UHD: i32 = 5045;
    pub const TV_OTHER: i32 = 5050;
    pub const TV_SPORT: i32 = 5060;
    pub const TV_ANIME: i32 = 5070;
    pub const TV_DOCUMENTARY: i32 = 5080;

    // Audio subcategories
    pub const AUDIO_MP3: i32 = 3010;
    pub const AUDIO_VIDEO: i32 = 3020;
    pub const AUDIO_AUDIOBOOK: i32 = 3030;
    pub const AUDIO_LOSSLESS: i32 = 3040;
    pub const AUDIO_OTHER: i32 = 3050;
    pub const AUDIO_FOREIGN: i32 = 3060;

    // Books subcategories
    pub const BOOKS_MAGS: i32 = 7010;
    pub const BOOKS_EBOOK: i32 = 7020;
    pub const BOOKS_COMICS: i32 = 7030;
    pub const BOOKS_TECHNICAL: i32 = 7040;
    pub const BOOKS_OTHER: i32 = 7050;
    pub const BOOKS_FOREIGN: i32 = 7060;

    // PC subcategories
    pub const PC_0DAY: i32 = 4010;
    pub const PC_ISO: i32 = 4020;
    pub const PC_MAC: i32 = 4030;
    pub const PC_MOBILE_OTHER: i32 = 4040;
    pub const PC_GAMES: i32 = 4050;
    pub const PC_MOBILE_IOS: i32 = 4060;
    pub const PC_MOBILE_ANDROID: i32 = 4070;

    // Console subcategories
    pub const CONSOLE_NDS: i32 = 1010;
    pub const CONSOLE_PSP: i32 = 1020;
    pub const CONSOLE_WII: i32 = 1030;
    pub const CONSOLE_XBOX: i32 = 1040;
    pub const CONSOLE_XBOX360: i32 = 1050;
    pub const CONSOLE_PS3: i32 = 1080;
    pub const CONSOLE_PS4: i32 = 1150;
    pub const CONSOLE_SWITCH: i32 = 1180;
    pub const CONSOLE_OTHER: i32 = 1090;
}

/// Get a category by ID
pub fn get_category(id: i32) -> Option<&'static TorznabCategory> {
    TORZNAB_CATEGORIES.iter().find(|c| c.id == id)
}

/// Get all subcategories for a parent category
pub fn get_subcategories(parent_id: i32) -> Vec<&'static TorznabCategory> {
    TORZNAB_CATEGORIES
        .iter()
        .filter(|c| c.parent_id == Some(parent_id))
        .collect()
}

/// Get the parent category for a given category
pub fn get_parent_category(id: i32) -> Option<&'static TorznabCategory> {
    let cat = get_category(id)?;
    cat.parent_id.and_then(get_category)
}

/// Expand categories to include all subcategories
/// E.g., [2000] -> [2000, 2010, 2020, 2030, 2040, 2045, 2050, 2060, 2070, 2080]
pub fn expand_categories(categories: &[i32]) -> Vec<i32> {
    let mut expanded = vec![];

    for &cat in categories {
        expanded.push(cat);

        // If this is a parent category, add all subcategories
        for torznab_cat in TORZNAB_CATEGORIES {
            if torznab_cat.parent_id == Some(cat) {
                expanded.push(torznab_cat.id);
            }
        }
    }

    expanded.sort();
    expanded.dedup();
    expanded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_categories() {
        let expanded = expand_categories(&[2000]);
        assert!(expanded.contains(&2000));
        assert!(expanded.contains(&2040)); // Movies/HD
        assert!(expanded.contains(&2045)); // Movies/UHD
        assert!(!expanded.contains(&5000)); // TV is not included
    }

    #[test]
    fn test_get_subcategories() {
        let subs = get_subcategories(5000);
        assert!(subs.iter().any(|c| c.id == 5040)); // TV/HD
        assert!(subs.iter().any(|c| c.id == 5070)); // TV/Anime
    }
}
