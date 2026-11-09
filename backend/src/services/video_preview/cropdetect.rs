//! Black bars baked into a video's picture (design.md Q123).
//!
//! ffmpeg's `cropdetect` filter reports, frame by frame, the smallest box holding everything
//! brighter than a limit. One reading is easily fooled: a dark scene looks like a small picture,
//! a studio logo on black looks like a tiny one, and an intro may be full frame while the film
//! is letterboxed. So the file is sampled at several points across its length (never only the
//! start), each point gives the union of a few consecutive frames, and [`stable_crop`] keeps a
//! rectangle only when most points agree and no real share of them shows a bigger picture.

use crate::graphql::entities::PictureCrop;

/// Sample points across a file: from 10% to 90% of its length, which keeps clear of most intros,
/// logos and credits.
pub const SAMPLE_POINTS: usize = 10;
/// Consecutive frames decoded at each point. Their union is the point's reading.
pub const FRAMES_PER_POINT: u32 = 6;
/// The `cropdetect` limit, as a fraction of the brightest value so it means the same for 8-bit
/// and 10-bit video (an integer limit of 24 would be nearly black on 8-bit and pure black on
/// 10-bit).
pub const CROPDETECT_LIMIT: &str = "0.1";
/// At least this many points must give a usable reading.
const MIN_READINGS: usize = 3;
/// The biggest group of agreeing readings must hold at least this share of them (60%).
const AGREEMENT_NUMERATOR: usize = 3;
const AGREEMENT_DENOMINATOR: usize = 5;
/// If this share (20%) of readings shows a picture bigger than the agreed one, the file changes
/// aspect (IMAX scenes, a full-frame intro that runs long) and no crop is kept.
const LARGER_NUMERATOR: usize = 1;
const LARGER_DENOMINATOR: usize = 5;
/// Bars narrower than this share of the frame (2%) are not worth zooming for.
const MIN_BAR_PERCENT: i32 = 2;

/// One reading: the inclusive edges of the bright area, in the frame's pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CropBox {
    pub x1: i32,
    pub x2: i32,
    pub y1: i32,
    pub y2: i32,
}

impl CropBox {
    pub fn width(&self) -> i32 {
        self.x2 - self.x1 + 1
    }

    pub fn height(&self) -> i32 {
        self.y2 - self.y1 + 1
    }

    fn union(self, other: CropBox) -> CropBox {
        CropBox {
            x1: self.x1.min(other.x1),
            x2: self.x2.max(other.x2),
            y1: self.y1.min(other.y1),
            y2: self.y2.max(other.y2),
        }
    }

    /// Whether every edge is within the tolerances of `other`'s.
    fn close_to(&self, other: &CropBox, tol_x: i32, tol_y: i32) -> bool {
        (self.x1 - other.x1).abs() <= tol_x
            && (self.x2 - other.x2).abs() <= tol_x
            && (self.y1 - other.y1).abs() <= tol_y
            && (self.y2 - other.y2).abs() <= tol_y
    }

    /// Whether this box reaches past `picture` by more than the tolerances on any side.
    fn extends_past(&self, picture: &CropBox, tol_x: i32, tol_y: i32) -> bool {
        self.x1 < picture.x1 - tol_x
            || self.x2 > picture.x2 + tol_x
            || self.y1 < picture.y1 - tol_y
            || self.y2 > picture.y2 + tol_y
    }
}

/// What the readings say about a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CropDecision {
    /// Black bars: the picture is this rectangle.
    Bars(PictureCrop),
    /// The picture fills the frame (or the bars are too thin to matter).
    FullFrame,
    /// The readings disagree: the aspect changes, or most points were too dark to read.
    Inconclusive,
}

impl CropDecision {
    pub fn crop(&self) -> Option<PictureCrop> {
        match self {
            CropDecision::Bars(crop) => Some(crop.clone()),
            _ => None,
        }
    }

    pub fn describe(&self) -> String {
        match self {
            CropDecision::Bars(crop) => format!(
                "{}x{} picture at {},{} in {}x{}",
                crop.width, crop.height, crop.left, crop.top, crop.frame_width, crop.frame_height
            ),
            CropDecision::FullFrame => "no black bars".to_string(),
            CropDecision::Inconclusive => "inconclusive".to_string(),
        }
    }
}

/// Where to sample a file of this length, in seconds.
pub fn sample_times(duration_secs: f64) -> Vec<f64> {
    if !duration_secs.is_finite() || duration_secs <= 0.0 {
        return Vec::new();
    }
    let points = SAMPLE_POINTS.max(2);
    (0..points)
        .map(|index| duration_secs * (0.1 + 0.8 * index as f64 / (points - 1) as f64))
        .collect()
}

/// One ffmpeg run's reading: the union of every usable `cropdetect` line in its log. `None` when
/// there is none (every frame was black, or the filter never ran).
///
/// A line looks like `[Parsed_cropdetect_0 @ 0x…] x1:0 x2:1919 y1:138 y2:941 w:1920 h:800 x:0
/// y:140 pts:… t:… limit:0.100000 crop=1920:800:0:140`. The edges are read rather than `crop=`,
/// which is already rounded. A black frame reports inverted edges (`x1` past `x2`), which are
/// skipped.
pub fn parse_cropdetect(output: &str) -> Option<CropBox> {
    let mut found: Option<CropBox> = None;
    for line in output.lines().filter(|line| line.contains("cropdetect")) {
        let value = |name: &str| -> Option<i32> {
            line.split_whitespace()
                .find_map(|token| token.strip_prefix(name)?.strip_prefix(':'))
                .and_then(|value| value.parse::<i32>().ok())
        };
        let (Some(x1), Some(x2), Some(y1), Some(y2)) =
            (value("x1"), value("x2"), value("y1"), value("y2"))
        else {
            continue;
        };
        if x1 < 0 || y1 < 0 || x2 < x1 || y2 < y1 {
            continue;
        }
        let reading = CropBox { x1, x2, y1, y2 };
        found = Some(found.map_or(reading, |box_so_far| box_so_far.union(reading)));
    }
    found
}

/// Settle the readings from across a file into one decision for a `width` x `height` frame.
pub fn stable_crop(readings: &[CropBox], width: i32, height: i32) -> CropDecision {
    if width <= 0 || height <= 0 {
        return CropDecision::Inconclusive;
    }
    // Readings clipped to the frame; a picture smaller than a quarter of either side is a dark
    // scene or a logo, not the picture.
    let usable: Vec<CropBox> = readings
        .iter()
        .map(|reading| CropBox {
            x1: reading.x1.clamp(0, width - 1),
            x2: reading.x2.clamp(0, width - 1),
            y1: reading.y1.clamp(0, height - 1),
            y2: reading.y2.clamp(0, height - 1),
        })
        .filter(|reading| reading.width() * 4 >= width && reading.height() * 4 >= height)
        .collect();
    if usable.len() < MIN_READINGS {
        return CropDecision::Inconclusive;
    }
    let tol_x = (width / 100).max(8);
    let tol_y = (height / 100).max(8);

    // The reading most others agree with; a tie goes to the bigger picture.
    let agreeing = |candidate: &CropBox| {
        usable
            .iter()
            .filter(|other| other.close_to(candidate, tol_x, tol_y))
            .count()
    };
    let Some(centre) = usable.iter().max_by_key(|candidate| {
        (
            agreeing(candidate),
            i64::from(candidate.width()) * i64::from(candidate.height()),
        )
    }) else {
        return CropDecision::Inconclusive;
    };
    let group: Vec<CropBox> = usable
        .iter()
        .copied()
        .filter(|other| other.close_to(centre, tol_x, tol_y))
        .collect();
    if group.len() * AGREEMENT_DENOMINATOR < usable.len() * AGREEMENT_NUMERATOR {
        return CropDecision::Inconclusive;
    }
    // The biggest picture the group saw, so a darker reading in it never trims the picture.
    let picture = group
        .iter()
        .copied()
        .reduce(CropBox::union)
        .unwrap_or(*centre);
    let larger = usable
        .iter()
        .filter(|other| other.extends_past(&picture, tol_x, tol_y))
        .count();
    if larger * LARGER_DENOMINATOR >= usable.len() * LARGER_NUMERATOR {
        return CropDecision::Inconclusive;
    }

    // Snap outwards to even pixels, and ignore thin bars along either axis.
    let mut left = picture.x1 & !1;
    let mut right = ((picture.x2 + 2) & !1).min(width);
    let mut top = picture.y1 & !1;
    let mut bottom = ((picture.y2 + 2) & !1).min(height);
    if (width - (right - left)) * 100 < width * MIN_BAR_PERCENT {
        left = 0;
        right = width;
    }
    if (height - (bottom - top)) * 100 < height * MIN_BAR_PERCENT {
        top = 0;
        bottom = height;
    }
    if left == 0 && right == width && top == 0 && bottom == height {
        return CropDecision::FullFrame;
    }
    CropDecision::Bars(PictureCrop {
        left,
        top,
        width: right - left,
        height: bottom - top,
        frame_width: width,
        frame_height: height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(x1: i32, x2: i32, y1: i32, y2: i32) -> String {
        format!(
            "[Parsed_cropdetect_0 @ 0x5581c0] x1:{x1} x2:{x2} y1:{y1} y2:{y2} w:{} h:{} x:{x1} y:{y1} pts:1536 t:0.120000 limit:0.100000 crop={}:{}:{x1}:{y1}",
            x2 - x1 + 1,
            y2 - y1 + 1,
            x2 - x1 + 1,
            y2 - y1 + 1
        )
    }

    fn reading(x1: i32, x2: i32, y1: i32, y2: i32) -> CropBox {
        CropBox { x1, x2, y1, y2 }
    }

    #[test]
    fn parses_a_letterboxed_run_as_the_union_of_its_frames() {
        let log = [
            "Input #0, matroska,webm, from 'Fallout.S01E01.mkv':".to_string(),
            line(0, 1919, 140, 939),
            line(0, 1919, 138, 941),
            "frame=    6 fps=0.0 q=-0.0 Lsize=N/A time=00:00:00.24".to_string(),
        ]
        .join("\n");
        assert_eq!(parse_cropdetect(&log), Some(reading(0, 1919, 138, 941)));
    }

    #[test]
    fn parses_a_pillarboxed_run() {
        let log = line(240, 1679, 0, 1079);
        let found = parse_cropdetect(&log).expect("reading");
        assert_eq!((found.width(), found.height()), (1440, 1080));
    }

    #[test]
    fn skips_black_frames_and_lines_without_edges() {
        // A black frame reports inverted edges; an unrelated log line has none.
        let log = [
            "[Parsed_cropdetect_0 @ 0x1] x1:1919 x2:0 y1:1079 y2:0 w:-1904 h:-1072 x:1914 y:1078 pts:0 t:0.000000 limit:0.100000 crop=-1904:-1072:1914:1078".to_string(),
            "[Parsed_cropdetect_0 @ 0x1] limit:0.100000".to_string(),
        ]
        .join("\n");
        assert_eq!(parse_cropdetect(&log), None);
        assert_eq!(parse_cropdetect(""), None);
    }

    #[test]
    fn a_letterboxed_film_with_a_full_frame_intro_and_dark_scenes_keeps_its_bars() {
        // Fallout S01E01: a 1920x1080 frame holding a 1920x804 picture.
        let mut readings = vec![reading(0, 1919, 138, 941); 7];
        readings[1] = reading(0, 1919, 140, 939); // a slightly darker shot
        readings.push(reading(0, 1919, 0, 1079)); // one full-frame shot
        readings.push(reading(700, 1200, 400, 700)); // a dark scene: ignored as too small
        readings.push(reading(2, 1917, 300, 800)); // a dim shot inside the picture
        let decision = stable_crop(&readings, 1920, 1080);
        assert_eq!(
            decision,
            CropDecision::Bars(PictureCrop {
                left: 0,
                top: 138,
                width: 1920,
                height: 804,
                frame_width: 1920,
                frame_height: 1080,
            })
        );
    }

    #[test]
    fn a_pillarboxed_picture_is_found() {
        let readings = vec![reading(240, 1679, 0, 1079); 6];
        let crop = stable_crop(&readings, 1920, 1080).crop().expect("bars");
        assert_eq!(
            (crop.left, crop.width, crop.top, crop.height),
            (240, 1440, 0, 1080)
        );
    }

    #[test]
    fn a_full_frame_picture_has_no_crop() {
        let readings = vec![reading(0, 1919, 0, 1079); 8];
        assert_eq!(stable_crop(&readings, 1920, 1080), CropDecision::FullFrame);
        // Bars a few pixels thick (an encoder's edge) are not worth zooming for.
        let thin = vec![reading(0, 1919, 6, 1073); 8];
        assert_eq!(stable_crop(&thin, 1920, 1080), CropDecision::FullFrame);
    }

    #[test]
    fn noisy_readings_that_disagree_are_inconclusive() {
        // Grain in the bars makes every reading different.
        let readings = vec![
            reading(0, 1919, 138, 941),
            reading(0, 1919, 60, 1010),
            reading(0, 1919, 200, 900),
            reading(0, 1919, 10, 1070),
            reading(0, 1919, 100, 980),
        ];
        assert_eq!(
            stable_crop(&readings, 1920, 1080),
            CropDecision::Inconclusive
        );
    }

    #[test]
    fn a_film_that_opens_up_for_imax_scenes_is_not_cropped() {
        let mut readings = vec![reading(0, 1919, 138, 941); 7];
        readings.extend([reading(0, 1919, 0, 1079); 3]);
        assert_eq!(
            stable_crop(&readings, 1920, 1080),
            CropDecision::Inconclusive
        );
    }

    #[test]
    fn too_few_usable_readings_are_inconclusive() {
        let readings = vec![reading(0, 1919, 138, 941), reading(800, 900, 500, 600)];
        assert_eq!(
            stable_crop(&readings, 1920, 1080),
            CropDecision::Inconclusive
        );
        assert_eq!(stable_crop(&[], 1920, 1080), CropDecision::Inconclusive);
    }

    #[test]
    fn odd_edges_snap_outwards_to_even_pixels() {
        let readings = vec![reading(0, 1919, 139, 940); 5];
        let crop = stable_crop(&readings, 1920, 1080).crop().expect("bars");
        assert_eq!((crop.top, crop.height), (138, 804));
    }

    #[test]
    fn samples_spread_across_the_file_and_skip_its_ends() {
        let times = sample_times(3600.0);
        assert_eq!(times.len(), SAMPLE_POINTS);
        assert!((times[0] - 360.0).abs() < 1e-6);
        assert!((times[times.len() - 1] - 3240.0).abs() < 1e-6);
        assert!(sample_times(0.0).is_empty());
        assert!(sample_times(f64::NAN).is_empty());
    }
}
