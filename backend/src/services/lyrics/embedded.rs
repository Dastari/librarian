//! Lyrics inside an audio file's tags, read with `lofty` through the storage layer.
//!
//! The file is opened with `library_storage` and read through a blocking bridge, so a track on
//! an SMB share is read the same way as a local one, and only the parts of the file the tags are
//! in are fetched. ID3v2 `SYLT` (synced) and `USLT`, Vorbis/FLAC `LYRICS` and `UNSYNCEDLYRICS`,
//! APE `Lyrics` and MP4 `©lyr` are read. An LRC text in a plain tag counts as synced.

use std::io::Seek;

use anyhow::{Context, Result};
use lofty::config::ParseOptions;
use lofty::file::{AudioFile, FileType, TaggedFileExt};
use lofty::id3::v2::{Frame, SyncTextContentType, SynchronizedTextFrame, TimestampFormat};
use lofty::mpeg::MpegFile;
use lofty::probe::Probe;
use lofty::tag::ItemKey;
use tokio_util::io::SyncIoBridge;

use crate::services::library_storage::{IntoStoragePath, library_storage};

use super::lrc::{self, LyricLine};

/// Lyrics found in tags.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EmbeddedLyrics {
    pub synced: Option<String>,
    pub plain: Option<String>,
    pub language: Option<String>,
}

impl EmbeddedLyrics {
    pub fn is_empty(&self) -> bool {
        self.synced.is_none() && self.plain.is_none()
    }
}

fn options() -> ParseOptions {
    // Only tags: no audio properties (an MP3 would be scanned for frames) and no pictures.
    ParseOptions::new()
        .read_properties(false)
        .read_cover_art(false)
}

/// Read lyrics from a file's tags. `Ok(None)` when it has none or its format has no tags lofty
/// knows.
pub async fn read(path: impl IntoStoragePath) -> Result<Option<EmbeddedLyrics>> {
    let opened = library_storage()
        .open_read(path)
        .await
        .context("Could not open the file to read its tags")?;
    let bridge = SyncIoBridge::new(opened.reader);
    tokio::task::spawn_blocking(move || read_blocking(bridge))
        .await
        .context("Reading tags stopped")?
}

fn read_blocking<R: std::io::Read + Seek>(reader: R) -> Result<Option<EmbeddedLyrics>> {
    // Small reads through the bridge are each a round trip to a share: buffer them.
    let mut reader = std::io::BufReader::with_capacity(64 * 1024, reader);
    let probe = Probe::new(&mut reader)
        .options(options())
        .guess_file_type()?;
    let file_type = probe.file_type();
    let mut found = EmbeddedLyrics::default();
    if file_type == Some(FileType::Mpeg) {
        reader.rewind()?;
        let file = MpegFile::read_from(&mut reader, options())?;
        if let Some(tag) = file.id3v2() {
            for frame in tag {
                if frame.id().as_str() != "SYLT" {
                    continue;
                }
                if let Frame::Binary(binary) = frame
                    && let Ok(sylt) = SynchronizedTextFrame::parse(&binary.data, frame.flags())
                    && let Some(text) = sylt_to_lrc(&sylt)
                {
                    found.language = std::str::from_utf8(&sylt.language)
                        .ok()
                        .map(str::to_owned)
                        .filter(|code| code.chars().all(|c| c.is_ascii_alphabetic()));
                    found.synced = Some(text);
                    break;
                }
            }
        }
        reader.rewind()?;
    }
    let tagged = Probe::new(&mut reader)
        .options(options())
        .guess_file_type()?
        .read()?;
    for tag in tagged.tags() {
        for key in [ItemKey::Lyrics, ItemKey::UnsyncLyrics] {
            for text in tag.get_strings(key) {
                take_text(&mut found, text);
            }
        }
        if found.language.is_none()
            && let Some(language) = tag.get_string(ItemKey::Language)
        {
            found.language = Some(language.to_owned());
        }
    }
    Ok((!found.is_empty()).then_some(found))
}

/// A tag's text: LRC when it has timed lines, plain otherwise. The first of each wins.
fn take_text(found: &mut EmbeddedLyrics, text: &str) {
    if let Some(synced) = lrc::normalise(text) {
        if found.synced.is_none() {
            found.synced = Some(synced);
        }
    } else if found.plain.is_none()
        && let Some(plain) = lrc::normalise_plain(text)
    {
        found.plain = Some(plain);
    }
}

/// A `SYLT` frame of lyrics in milliseconds as stored LRC. Frames timed in MPEG frames, or
/// holding something other than lyrics, are not used.
fn sylt_to_lrc(frame: &SynchronizedTextFrame<'_>) -> Option<String> {
    if frame.timestamp_format != TimestampFormat::MS
        || !matches!(
            frame.content_type,
            SyncTextContentType::Lyrics | SyncTextContentType::TextTranscription
        )
    {
        return None;
    }
    let lines = sylt_lines(&frame.content);
    (lines.len() >= 2).then(|| lrc::to_lrc(&lines))
}

/// SYLT entries are often a syllable or word each, with the line break inside the text. Join
/// them into lines that start where their first piece does.
pub(crate) fn sylt_lines(content: &[(u32, String)]) -> Vec<LyricLine> {
    // Most taggers write one entry per line, with no breaks at all.
    if !content
        .iter()
        .any(|(_, piece)| piece.contains(['\n', '\r']))
    {
        return content
            .iter()
            .map(|(stamp, text)| LyricLine {
                start_ms: i64::from(*stamp),
                text: text.split_whitespace().collect::<Vec<_>>().join(" "),
            })
            .collect();
    }
    let mut lines: Vec<LyricLine> = Vec::new();
    let mut current: Option<LyricLine> = None;
    for (stamp, piece) in content {
        let mut piece = piece.as_str();
        let starts_line = piece.starts_with(['\n', '\r']);
        piece = piece.trim_start_matches(['\n', '\r']);
        if starts_line || current.is_none() {
            if let Some(line) = current.take() {
                lines.push(line);
            }
            current = Some(LyricLine {
                start_ms: i64::from(*stamp),
                text: String::new(),
            });
        }
        let ends_line = piece.ends_with(['\n', '\r']);
        if let Some(line) = current.as_mut() {
            line.text.push_str(piece.trim_end_matches(['\n', '\r']));
        }
        if ends_line && let Some(line) = current.take() {
            lines.push(line);
        }
    }
    if let Some(line) = current {
        lines.push(line);
    }
    for line in &mut lines {
        line.text = line.text.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sylt_pieces_join_into_lines() {
        let content = vec![
            (1000, "Hel".to_owned()),
            (1200, "lo ".to_owned()),
            (1400, "world".to_owned()),
            (3000, "\nSecond ".to_owned()),
            (3300, "line\n".to_owned()),
            (5000, "Third".to_owned()),
        ];
        let lines = sylt_lines(&content);
        assert_eq!(
            lines,
            vec![
                LyricLine {
                    start_ms: 1000,
                    text: "Hello world".into()
                },
                LyricLine {
                    start_ms: 3000,
                    text: "Second line".into()
                },
                LyricLine {
                    start_ms: 5000,
                    text: "Third".into()
                },
            ]
        );
        // One entry per line, as most taggers write it.
        let per_line = sylt_lines(&[(0, "a".into()), (1000, "b".into())]);
        assert_eq!(per_line.len(), 2, "entries without breaks are a line each");
    }

    #[test]
    fn tag_text_is_synced_or_plain() {
        let mut found = EmbeddedLyrics::default();
        take_text(&mut found, "[00:01.00]One\n[00:02.00]Two");
        take_text(&mut found, "Plain words\r\nhere");
        assert_eq!(
            found.synced.as_deref(),
            Some("[00:01.00]One\n[00:02.00]Two\n")
        );
        assert_eq!(found.plain.as_deref(), Some("Plain words\nhere"));
    }

    #[test]
    fn reads_vorbis_lyrics_from_a_flac_file_on_disk() {
        // A minimal FLAC: marker, STREAMINFO, and a VORBIS_COMMENT with LYRICS.
        let mut bytes = b"fLaC".to_vec();
        let mut streaminfo = vec![0u8; 34];
        streaminfo[0..2].copy_from_slice(&4096u16.to_be_bytes());
        streaminfo[2..4].copy_from_slice(&4096u16.to_be_bytes());
        // 44100 Hz, 2 channels, 16 bits, 0 samples.
        let packed: u64 = (44_100u64 << 44) | (1u64 << 41) | (15u64 << 36);
        streaminfo[10..18].copy_from_slice(&packed.to_be_bytes());
        bytes.push(0x00);
        bytes.extend_from_slice(&(streaminfo.len() as u32).to_be_bytes()[1..]);
        bytes.extend_from_slice(&streaminfo);
        let mut comment = Vec::new();
        let vendor = b"test";
        comment.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        comment.extend_from_slice(vendor);
        let fields = ["TITLE=Song", "LYRICS=[00:01.00]One\n[00:02.50]Two"];
        comment.extend_from_slice(&(fields.len() as u32).to_le_bytes());
        for field in fields {
            comment.extend_from_slice(&(field.len() as u32).to_le_bytes());
            comment.extend_from_slice(field.as_bytes());
        }
        bytes.push(0x80 | 0x04);
        bytes.extend_from_slice(&(comment.len() as u32).to_be_bytes()[1..]);
        bytes.extend_from_slice(&comment);
        let found = read_blocking(std::io::Cursor::new(bytes)).unwrap().unwrap();
        assert_eq!(
            found.synced.as_deref(),
            Some("[00:01.00]One\n[00:02.50]Two\n")
        );
        assert_eq!(found.plain, None);
    }
}
