//! The ffmpeg runs behind a file's previews: crop readings and storyboard frames.
//!
//! Every run reads only near one point of the file: the input is opened with a fast seek
//! (`-ss` before `-i`, `-noaccurate_seek`) and decodes a handful of frames (crop) or one keyframe
//! (`-skip_frame nokey`, storyboard), so a two-hour film on a share is never read end to end.
//! Each run is single-threaded at the lowest CPU priority. Frames come back as raw RGB on stdout
//! and are drawn into sheets here, so nothing is written next to the media.

use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

use crate::graphql::entities::PictureCrop;
use crate::services::library_storage::{TOOL_PROTOCOL_WHITELIST, is_tool_url};

use super::cropdetect::{
    CROPDETECT_LIMIT, CropBox, CropDecision, FRAMES_PER_POINT, parse_cropdetect, sample_times,
    stable_crop,
};
use super::storyboard::StoryboardLayout;

/// One run may take this long (a slow share, a slow seek).
const RUN_TIMEOUT: Duration = Duration::from_secs(60);
const STDERR_LIMIT: u64 = 512 * 1024;
/// More than this share of frames failing (a quarter) means the sheets are not worth keeping.
const MAX_FAILED_FRAMES_NUMERATOR: u32 = 1;
const MAX_FAILED_FRAMES_DENOMINATOR: u32 = 4;
/// JPEG quality of a sheet.
const SHEET_QUALITY: u8 = 72;

/// How to run ffmpeg.
#[derive(Debug, Clone)]
pub struct Tools {
    pub ffmpeg: String,
}

/// A finished sheet, as JPEG.
pub struct Sheet {
    pub index: u32,
    pub jpeg: Vec<u8>,
}

/// Run at the lowest CPU priority: the server shares its machine with builds and playback.
fn lowly(program: &str) -> Command {
    let mut command = Command::new(program);
    #[cfg(unix)]
    {
        // SAFETY: `setpriority` is async-signal-safe and touches only the child being started.
        unsafe {
            command.pre_exec(|| {
                libc::setpriority(libc::PRIO_PROCESS, 0, 19);
                Ok(())
            });
        }
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command
}

fn input_args(command: &mut Command, input: &str, seconds: f64) {
    // A file on a share is read through the loopback tool URL (design.md Q87).
    if is_tool_url(input) {
        command
            .arg("-protocol_whitelist")
            .arg(TOOL_PROTOCOL_WHITELIST);
    }
    command
        .arg("-noaccurate_seek")
        .arg("-ss")
        .arg(format!("{:.3}", seconds.max(0.0)))
        .arg("-i")
        .arg(input)
        .args(["-map", "0:v:0", "-an", "-sn", "-dn", "-threads", "1"]);
}

async fn read_limited<R: AsyncRead + Unpin>(reader: R, limit: u64) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    reader.take(limit).read_to_end(&mut output).await?;
    Ok(output)
}

/// Run to completion; stdout up to `stdout_limit` bytes, and stderr as text.
async fn run(mut command: Command, stdout_limit: u64) -> Result<(bool, Vec<u8>, String)> {
    let mut child = command.spawn().context("Could not start ffmpeg")?;
    let stdout = child.stdout.take().context("ffmpeg has no stdout")?;
    let stderr = child.stderr.take().context("ffmpeg has no stderr")?;
    let finished = tokio::time::timeout(RUN_TIMEOUT, async {
        let (stdout, stderr, status) = tokio::join!(
            read_limited(stdout, stdout_limit),
            read_limited(stderr, STDERR_LIMIT),
            child.wait()
        );
        Ok::<_, std::io::Error>((stdout?, stderr?, status?))
    })
    .await;
    match finished {
        Ok(Ok((stdout, stderr, status))) => Ok((
            status.success(),
            stdout,
            String::from_utf8_lossy(&stderr).into_owned(),
        )),
        Ok(Err(error)) => Err(error).context("ffmpeg failed"),
        Err(_) => {
            let _ = child.kill().await;
            bail!("ffmpeg took longer than {} seconds", RUN_TIMEOUT.as_secs())
        }
    }
}

/// Read the picture's shape at points across the file and settle it (design.md Q123).
pub async fn detect_crop(
    tools: &Tools,
    input: &str,
    duration_secs: f64,
    width: i32,
    height: i32,
) -> Result<CropDecision> {
    let mut readings: Vec<CropBox> = Vec::new();
    let mut runs = 0;
    for seconds in sample_times(duration_secs) {
        let mut command = lowly(&tools.ffmpeg);
        command.args(["-hide_banner", "-nostdin", "-nostats", "-loglevel", "info"]);
        input_args(&mut command, input, seconds);
        command
            .arg("-frames:v")
            .arg(FRAMES_PER_POINT.to_string())
            .arg("-vf")
            .arg(format!(
                "cropdetect=limit={CROPDETECT_LIMIT}:round=2:skip=0:reset=0"
            ))
            .args(["-f", "null", "-"]);
        let (_, _, log) = run(command, 0).await?;
        runs += 1;
        if let Some(reading) = parse_cropdetect(&log) {
            readings.push(reading);
        }
    }
    if runs == 0 {
        return Ok(CropDecision::Inconclusive);
    }
    Ok(stable_crop(&readings, width, height))
}

/// Whether this ffmpeg can tone-map HDR (it has `zscale`). Asked once.
async fn can_tonemap(tools: &Tools) -> bool {
    static ANSWER: tokio::sync::OnceCell<bool> = tokio::sync::OnceCell::const_new();
    *ANSWER
        .get_or_init(|| async {
            let mut command = lowly(&tools.ffmpeg);
            command.args(["-hide_banner", "-filters"]);
            match run(command, 1024 * 1024).await {
                Ok((_, stdout, _)) => String::from_utf8_lossy(&stdout)
                    .lines()
                    .any(|line| line.split_whitespace().nth(1) == Some("zscale")),
                Err(_) => false,
            }
        })
        .await
}

fn frame_filter(layout: &StoryboardLayout, crop: Option<&PictureCrop>, tonemap: bool) -> String {
    let mut filters = Vec::new();
    if let Some(crop) = crop {
        filters.push(format!(
            "crop={}:{}:{}:{}",
            crop.width, crop.height, crop.left, crop.top
        ));
    }
    filters.push(format!(
        "scale={}:{}:flags=bilinear",
        layout.tile_width, layout.tile_height
    ));
    if tonemap {
        // HDR (PQ/HLG) to SDR, so previews are not washed out. Done after scaling: cheap.
        filters.push(
            "zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,tonemap=tonemap=hable:desat=0,zscale=t=bt709:m=bt709:r=tv"
                .to_string(),
        );
    }
    filters.push("format=rgb24".to_string());
    filters.join(",")
}

/// One tile's pixels (RGB, `tile_width * tile_height * 3` bytes) from the keyframe at or before
/// `seconds`. `None` if ffmpeg produced no frame there.
async fn grab_frame(
    tools: &Tools,
    input: &str,
    seconds: f64,
    filter: &str,
    expected: usize,
) -> Result<Option<Vec<u8>>> {
    let mut command = lowly(&tools.ffmpeg);
    command.args([
        "-hide_banner",
        "-nostdin",
        "-nostats",
        "-loglevel",
        "error",
        "-skip_frame",
        "nokey",
    ]);
    input_args(&mut command, input, seconds);
    command
        .args(["-frames:v", "1", "-vf", filter])
        .args(["-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1"]);
    let (_, stdout, _) = run(command, expected as u64 + 1).await?;
    Ok((stdout.len() == expected).then_some(stdout))
}

/// Draw the storyboard's sheets for `input`. Fails when too many frames could not be taken.
pub async fn render_storyboard(
    tools: &Tools,
    input: &str,
    duration_secs: f64,
    layout: &StoryboardLayout,
    crop: Option<&PictureCrop>,
    hdr: bool,
) -> Result<Vec<Sheet>> {
    let expected = (layout.tile_width * layout.tile_height * 3) as usize;
    let mut filter = frame_filter(layout, crop, hdr && can_tonemap(tools).await);
    let mut tiles: Vec<Option<Vec<u8>>> = Vec::with_capacity(layout.frame_count as usize);
    let mut failed = 0_u32;
    for index in 0..layout.frame_count {
        let seconds = layout.frame_time(index, duration_secs);
        let mut frame = grab_frame(tools, input, seconds, &filter, expected).await?;
        if frame.is_none() && hdr && filter.contains("zscale") && index == 0 {
            // This ffmpeg could not tone-map the stream; draw it as it is.
            filter = frame_filter(layout, crop, false);
            frame = grab_frame(tools, input, seconds, &filter, expected).await?;
        }
        if frame.is_none() {
            failed += 1;
            if failed * MAX_FAILED_FRAMES_DENOMINATOR
                > layout.frame_count * MAX_FAILED_FRAMES_NUMERATOR
            {
                bail!(
                    "ffmpeg gave no frame for {failed} of {} preview points",
                    layout.frame_count
                );
            }
        }
        tiles.push(frame);
    }
    let layout = *layout;
    tokio::task::spawn_blocking(move || draw_sheets(&layout, &tiles))
        .await
        .context("Drawing the preview sheets stopped")?
}

/// Lay the tiles out on their sheets. A missing tile repeats the one before it (or stays black).
pub fn draw_sheets(layout: &StoryboardLayout, tiles: &[Option<Vec<u8>>]) -> Result<Vec<Sheet>> {
    let (tile_width, tile_height) = (layout.tile_width, layout.tile_height);
    let mut sheets = Vec::with_capacity(layout.sheet_count as usize);
    let mut previous: Option<&Vec<u8>> = None;
    let mut canvas: Option<(u32, image::RgbImage)> = None;
    let finish = |sheet: u32, image: image::RgbImage| -> Result<Sheet> {
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, SHEET_QUALITY)
            .encode_image(&image)
            .context("Could not encode a preview sheet")?;
        Ok(Sheet { index: sheet, jpeg })
    };
    for index in 0..layout.frame_count {
        let position = layout.position(index);
        if canvas.as_ref().map(|(sheet, _)| *sheet) != Some(position.sheet) {
            if let Some((sheet, image)) = canvas.take() {
                sheets.push(finish(sheet, image)?);
            }
            let (width, height) = layout.sheet_size(position.sheet);
            canvas = Some((position.sheet, image::RgbImage::new(width, height)));
        }
        let pixels = tiles
            .get(index as usize)
            .and_then(Option::as_ref)
            .or(previous);
        if let (Some(pixels), Some((_, image))) = (pixels, canvas.as_mut())
            && let Some(tile) = image::RgbImage::from_raw(tile_width, tile_height, pixels.clone())
        {
            image::imageops::replace(image, &tile, i64::from(position.x), i64::from(position.y));
            previous = Some(pixels);
        }
    }
    if let Some((sheet, image)) = canvas.take() {
        sheets.push(finish(sheet, image)?);
    }
    Ok(sheets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::video_preview::storyboard;

    #[test]
    fn sheets_are_drawn_with_missing_tiles_repeating_the_last_one() {
        let layout = storyboard::plan(1_050.0, 16.0 / 9.0).expect("layout");
        assert_eq!((layout.frame_count, layout.sheet_count), (105, 2));
        let tile = |shade: u8| {
            Some(vec![
                shade;
                (layout.tile_width * layout.tile_height * 3) as usize
            ])
        };
        let mut tiles: Vec<Option<Vec<u8>>> = (0..105).map(|i| tile(i as u8)).collect();
        tiles[101] = None;
        let sheets = draw_sheets(&layout, &tiles).expect("sheets");
        assert_eq!(sheets.len(), 2);
        let second = image::load_from_memory(&sheets[1].jpeg)
            .expect("jpeg")
            .to_rgb8();
        assert_eq!(second.dimensions(), (2400, 136));
        // Tile 101 (the second on sheet 2) repeats tile 100's shade.
        let shade = second.get_pixel(240 + 120, 68)[0];
        assert!((i32::from(shade) - 100).abs() <= 3, "shade {shade}");
    }

    /// Makes a small letterboxed video with ffmpeg and runs the real crop detection and
    /// storyboard on it. Skipped where ffmpeg is not installed.
    #[tokio::test]
    async fn previews_are_made_from_a_synthetic_letterboxed_video() {
        let tools = Tools {
            ffmpeg: std::env::var("FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string()),
        };
        if std::process::Command::new(&tools.ffmpeg)
            .arg("-version")
            .output()
            .is_err()
        {
            eprintln!("ffmpeg is not installed; skipping");
            return;
        }
        let dir = tempfile::tempdir().expect("temp dir");
        let video = dir.path().join("letterboxed.mp4");
        // A 320x134 picture (2.39:1) padded to 320x180 with black bars, 60 seconds long.
        let made = std::process::Command::new(&tools.ffmpeg)
            .args(["-hide_banner", "-v", "error", "-y", "-f", "lavfi", "-i"])
            .arg("testsrc2=size=320x134:rate=25:duration=60")
            .args([
                "-vf",
                "pad=320:180:0:24:black",
                "-c:v",
                "libx264",
                "-g",
                "50",
            ])
            .args(["-pix_fmt", "yuv420p"])
            .arg(&video)
            .status()
            .expect("ffmpeg runs");
        assert!(made.success(), "could not make the test video");
        let input = video.to_string_lossy().into_owned();

        let decision = detect_crop(&tools, &input, 60.0, 320, 180)
            .await
            .expect("crop detection runs");
        let crop = decision.crop().expect("black bars are found");
        assert_eq!((crop.left, crop.width), (0, 320));
        assert!((22..=24).contains(&crop.top), "top {}", crop.top);
        assert!((134..=138).contains(&crop.height), "height {}", crop.height);

        let aspect = storyboard::picture_aspect(320, 180, None, Some((crop.width, crop.height)));
        let layout = storyboard::plan(60.0, aspect).expect("layout");
        assert_eq!(layout.frame_count, 6);
        let sheets = render_storyboard(&tools, &input, 60.0, &layout, Some(&crop), false)
            .await
            .expect("storyboard renders");
        assert_eq!(sheets.len(), 1);
        let sheet = image::load_from_memory(&sheets[0].jpeg)
            .expect("jpeg")
            .to_rgb8();
        assert_eq!(
            sheet.dimensions(),
            (6 * layout.tile_width, layout.tile_height)
        );
        // The bars were cropped away: the tile's top row is picture, not black.
        let top_row_brightness: u32 = (0..layout.tile_width)
            .map(|x| {
                u32::from(
                    sheet
                        .get_pixel(x, 1)
                        .0
                        .iter()
                        .map(|v| u16::from(*v))
                        .sum::<u16>(),
                )
            })
            .sum();
        assert!(
            top_row_brightness / layout.tile_width > 60,
            "top row looks black"
        );

        // A file with no bars says so.
        let plain = dir.path().join("plain.mp4");
        let made = std::process::Command::new(&tools.ffmpeg)
            .args(["-hide_banner", "-v", "error", "-y", "-f", "lavfi", "-i"])
            .arg("testsrc2=size=320x180:rate=25:duration=30")
            .args(["-c:v", "libx264", "-g", "50", "-pix_fmt", "yuv420p"])
            .arg(&plain)
            .status()
            .expect("ffmpeg runs");
        assert!(made.success());
        let decision = detect_crop(&tools, &plain.to_string_lossy(), 30.0, 320, 180)
            .await
            .expect("crop detection runs");
        assert_eq!(decision, CropDecision::FullFrame);
    }
}
