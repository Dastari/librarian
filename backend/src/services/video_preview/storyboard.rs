//! The layout of a video's seek-bar preview sheets (design.md Q122).
//!
//! A sheet is a grid of equally sized tiles, one frame per `interval` seconds, filled row by row.
//! A long video gets several sheets; a very long one a longer interval, so no file needs more
//! than [`MAX_FRAMES`] frames, each of which costs one seek.

/// One frame every this many seconds, unless the file is very long.
pub const BASE_INTERVAL_SECS: f64 = 10.0;
/// At most this many frames per file (a two-hour film gets one every 20 seconds).
pub const MAX_FRAMES: u32 = 360;
/// A tile is this wide, unless the picture is tall.
pub const TILE_WIDTH: u32 = 240;
/// And never taller than this.
pub const MAX_TILE_HEIGHT: u32 = 180;
/// Tiles per row and rows per sheet: 100 frames, a 2400x1350 JPEG for a 16:9 picture.
pub const COLUMNS: u32 = 10;
pub const ROWS: u32 = 10;
/// Bumped when the sheets would be drawn differently, so old caches are not reused.
pub const LAYOUT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StoryboardLayout {
    pub interval: f64,
    pub tile_width: u32,
    pub tile_height: u32,
    pub columns: u32,
    pub rows: u32,
    pub frame_count: u32,
    pub sheet_count: u32,
}

/// Where a tile sits: which sheet, and its top-left corner in that sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TilePosition {
    pub sheet: u32,
    pub x: u32,
    pub y: u32,
}

fn even(value: f64) -> u32 {
    ((value / 2.0).round() as u32 * 2).max(2)
}

/// A tile's size for a picture of this display aspect ratio (width / height).
pub fn tile_size(aspect: f64) -> (u32, u32) {
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        16.0 / 9.0
    };
    let height = even(f64::from(TILE_WIDTH) / aspect);
    if height <= MAX_TILE_HEIGHT {
        (TILE_WIDTH, height)
    } else {
        (
            even(f64::from(MAX_TILE_HEIGHT) * aspect).min(TILE_WIDTH),
            MAX_TILE_HEIGHT,
        )
    }
}

/// The display aspect ratio of the picture: the stream's display aspect ratio (`"16:9"`) where
/// ffprobe gave one, else its pixel size, narrowed to the picture inside any black bars.
pub fn picture_aspect(
    frame_width: i32,
    frame_height: i32,
    display_aspect: Option<&str>,
    crop: Option<(i32, i32)>,
) -> f64 {
    if frame_width <= 0 || frame_height <= 0 {
        return 16.0 / 9.0;
    }
    let frame_aspect = display_aspect
        .and_then(|value| {
            let (w, h) = value.split_once(':')?;
            let (w, h) = (w.trim().parse::<f64>().ok()?, h.trim().parse::<f64>().ok()?);
            (w > 0.0 && h > 0.0).then_some(w / h)
        })
        .unwrap_or(f64::from(frame_width) / f64::from(frame_height));
    match crop {
        Some((width, height)) if width > 0 && height > 0 => {
            frame_aspect * (f64::from(width) / f64::from(frame_width))
                / (f64::from(height) / f64::from(frame_height))
        }
        _ => frame_aspect,
    }
}

/// The sheets for a video of this length and picture shape. `None` without a usable length.
pub fn plan(duration_secs: f64, aspect: f64) -> Option<StoryboardLayout> {
    if !duration_secs.is_finite() || duration_secs < 1.0 {
        return None;
    }
    let interval = BASE_INTERVAL_SECS.max((duration_secs / f64::from(MAX_FRAMES)).ceil());
    let frame_count = ((duration_secs / interval).ceil() as u32).clamp(1, MAX_FRAMES);
    let per_sheet = COLUMNS * ROWS;
    let (tile_width, tile_height) = tile_size(aspect);
    Some(StoryboardLayout {
        interval,
        tile_width,
        tile_height,
        columns: COLUMNS,
        rows: ROWS,
        frame_count,
        sheet_count: frame_count.div_ceil(per_sheet),
    })
}

impl StoryboardLayout {
    /// The moment tile `index` is taken from: the middle of its stretch, inside the file.
    pub fn frame_time(&self, index: u32, duration_secs: f64) -> f64 {
        let middle = (f64::from(index) + 0.5) * self.interval;
        middle.min((duration_secs - 1.0).max(0.0))
    }

    /// Tiles on sheet `sheet` (the last may be partly filled).
    pub fn tiles_on_sheet(&self, sheet: u32) -> u32 {
        let per_sheet = self.columns * self.rows;
        self.frame_count
            .saturating_sub(sheet * per_sheet)
            .min(per_sheet)
    }

    /// A sheet's size in pixels. Only the last sheet may have fewer rows.
    pub fn sheet_size(&self, sheet: u32) -> (u32, u32) {
        let tiles = self.tiles_on_sheet(sheet);
        let rows = tiles.div_ceil(self.columns).max(1);
        let columns = if sheet == 0 && self.sheet_count == 1 {
            tiles.clamp(1, self.columns)
        } else {
            self.columns
        };
        (columns * self.tile_width, rows * self.tile_height)
    }

    /// Where tile `index` is drawn.
    pub fn position(&self, index: u32) -> TilePosition {
        let per_sheet = self.columns * self.rows;
        let within = index % per_sheet;
        TilePosition {
            sheet: index / per_sheet,
            x: (within % self.columns) * self.tile_width,
            y: (within / self.columns) * self.tile_height,
        }
    }

    /// The tile shown for a moment in the video, as the player looks it up.
    pub fn tile_for(&self, seconds: f64) -> TilePosition {
        let index = if seconds.is_finite() && seconds > 0.0 {
            ((seconds / self.interval).floor() as u32).min(self.frame_count.saturating_sub(1))
        } else {
            0
        };
        self.position(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_episode_gets_a_frame_every_ten_seconds() {
        let layout = plan(45.0 * 60.0, 16.0 / 9.0).expect("layout");
        assert_eq!(layout.interval, 10.0);
        assert_eq!(layout.frame_count, 270);
        assert_eq!(layout.sheet_count, 3);
        assert_eq!((layout.tile_width, layout.tile_height), (240, 136));
    }

    #[test]
    fn a_long_film_gets_fewer_frames() {
        let layout = plan(2.0 * 3600.0, 16.0 / 9.0).expect("layout");
        assert_eq!(layout.interval, 20.0);
        assert_eq!(layout.frame_count, 360);
        assert_eq!(layout.sheet_count, 4);
        let epic = plan(4.0 * 3600.0 + 1.0, 2.39).expect("layout");
        assert!(epic.frame_count <= MAX_FRAMES);
        assert_eq!(epic.interval, 41.0);
    }

    #[test]
    fn a_short_clip_fits_one_partly_filled_sheet() {
        let layout = plan(35.0, 16.0 / 9.0).expect("layout");
        assert_eq!(layout.frame_count, 4);
        assert_eq!(layout.sheet_count, 1);
        assert_eq!(layout.sheet_size(0), (4 * 240, 136));
        assert!(plan(0.0, 1.0).is_none());
        assert!(plan(f64::NAN, 1.0).is_none());
    }

    #[test]
    fn tiles_follow_the_picture_shape() {
        assert_eq!(tile_size(16.0 / 9.0), (240, 136));
        assert_eq!(tile_size(1920.0 / 804.0), (240, 100));
        assert_eq!(tile_size(4.0 / 3.0), (240, 180));
        assert_eq!(tile_size(9.0 / 16.0), (102, 180));
        assert_eq!(tile_size(0.0), (240, 136));
    }

    #[test]
    fn the_picture_aspect_uses_the_display_ratio_and_the_crop() {
        let letterboxed = picture_aspect(1920, 1080, Some("16:9"), Some((1920, 804)));
        assert!((letterboxed - 1920.0 / 804.0).abs() < 1e-9);
        // An anamorphic DVD: 720x576 shown at 16:9.
        let anamorphic = picture_aspect(720, 576, Some("16:9"), None);
        assert!((anamorphic - 16.0 / 9.0).abs() < 1e-9);
        let unknown = picture_aspect(1280, 720, Some("0:1"), None);
        assert!((unknown - 16.0 / 9.0).abs() < 1e-9);
    }

    #[test]
    fn tiles_are_found_across_sheets_as_the_player_finds_them() {
        let layout = plan(2.0 * 3600.0, 16.0 / 9.0).expect("layout");
        assert_eq!(
            layout.tile_for(0.0),
            TilePosition {
                sheet: 0,
                x: 0,
                y: 0
            }
        );
        // 25 minutes in: tile 75, row 7, column 5 of the first sheet.
        assert_eq!(
            layout.tile_for(25.0 * 60.0),
            TilePosition {
                sheet: 0,
                x: 5 * 240,
                y: 7 * 136
            }
        );
        // Tile 100 starts the second sheet.
        assert_eq!(
            layout.tile_for(2000.0),
            TilePosition {
                sheet: 1,
                x: 0,
                y: 0
            }
        );
        // Past the end, the last tile.
        assert_eq!(
            layout.tile_for(99_999.0),
            TilePosition {
                sheet: 3,
                x: 9 * 240,
                y: 5 * 136
            }
        );
        assert_eq!(layout.tiles_on_sheet(3), 60);
        assert_eq!(layout.sheet_size(3), (2400, 6 * 136));
        assert_eq!(layout.sheet_size(0), (2400, 1360));
    }

    #[test]
    fn frames_are_taken_from_the_middle_of_their_stretch_inside_the_file() {
        let layout = plan(35.0, 16.0 / 9.0).expect("layout");
        assert_eq!(layout.frame_time(0, 35.0), 5.0);
        assert_eq!(layout.frame_time(3, 35.0), 34.0);
    }
}
