//! Text subtitles to WebVTT, without ffmpeg.
//!
//! Sidecar and downloaded subtitles are small text files, so they are read through the storage
//! layer and converted here: SubRip (`.srt`), Advanced SubStation (`.ass`/`.ssa`, styling
//! dropped apart from italic, bold and underline), MicroDVD (`.sub` without an `.idx`) and WebVTT
//! itself. Embedded text streams still come out of ffmpeg as WebVTT and pass through
//! [`parse_vtt`] so the same timing offset applies to every kind.
//!
//! Cue text is reduced to what WebVTT and every browser render alike: `<i>`, `<b>` and `<u>`,
//! balanced, with everything else escaped. A subtitle file is untrusted input, so nothing in it
//! can add markup, classes or voices of its own.

use std::fmt::Write as _;

/// One timed piece of text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cue {
    pub start_ms: i64,
    pub end_ms: i64,
    /// WebVTT cue text: lines joined by `\n`, with only `<i>`, `<b>` and `<u>` markup.
    pub text: String,
    /// WebVTT cue settings kept from the source (`line:0` for a cue placed at the top).
    pub settings: Option<String>,
}

/// The text formats that can be converted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFormat {
    Srt,
    Ass,
    Vtt,
    MicroDvd,
}

impl TextFormat {
    /// The format a sidecar's extension names. `.sub` is MicroDVD here; a `.sub` with an `.idx`
    /// beside it is VobSub, a picture format, and never reaches this module.
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_ascii_lowercase().as_str() {
            "srt" => Some(Self::Srt),
            "ass" | "ssa" => Some(Self::Ass),
            "vtt" => Some(Self::Vtt),
            "sub" => Some(Self::MicroDvd),
            _ => None,
        }
    }

    /// The codec name stored on a `Subtitle` row, as ffprobe names the same format.
    pub fn codec(self) -> &'static str {
        match self {
            Self::Srt => "subrip",
            Self::Ass => "ass",
            Self::Vtt => "webvtt",
            Self::MicroDvd => "microdvd",
        }
    }

    /// The format of a stored codec name.
    pub fn from_codec(codec: &str) -> Option<Self> {
        match codec.to_ascii_lowercase().as_str() {
            "subrip" | "srt" => Some(Self::Srt),
            "ass" | "ssa" => Some(Self::Ass),
            "webvtt" | "vtt" => Some(Self::Vtt),
            "microdvd" => Some(Self::MicroDvd),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("the file has no subtitles in it")]
    Empty,
}

/// Frames per second assumed for a MicroDVD file that does not say.
pub const DEFAULT_MICRODVD_FPS: f64 = 23.976;

/// Parse a text subtitle file. `fps` is only used by MicroDVD, whose times are frame numbers.
pub fn parse(format: TextFormat, text: &str, fps: Option<f64>) -> Result<Vec<Cue>, ParseError> {
    let mut cues = match format {
        TextFormat::Srt => parse_srt(text),
        TextFormat::Ass => parse_ass(text),
        TextFormat::Vtt => parse_vtt(text),
        TextFormat::MicroDvd => parse_microdvd(text, fps),
    };
    cues.retain(|cue| cue.end_ms > cue.start_ms && !cue.text.trim().is_empty());
    if cues.is_empty() {
        return Err(ParseError::Empty);
    }
    cues.sort_by_key(|cue| (cue.start_ms, cue.end_ms));
    Ok(cues)
}

/// Decode a subtitle file's bytes. UTF-8 (with or without a byte order mark) and UTF-16 with one
/// are read as they are; anything else is guessed, since old SubRip files are usually in the
/// author's Windows code page.
pub fn decode_text(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return encoding_rs::UTF_16LE.decode(bytes).0.into_owned();
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return encoding_rs::UTF_16BE.decode(bytes).0.into_owned();
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }
    let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
    detector.feed(bytes, true);
    let encoding = detector.guess(None, chardetng::Utf8Detection::Allow);
    encoding.decode(bytes).0.into_owned()
}

/// Write cues as a WebVTT file, every time moved by `offset_ms` (positive is later). Cues that
/// end before the start of the file are dropped; one that starts before it starts at zero.
pub fn to_vtt(cues: &[Cue], offset_ms: i64) -> String {
    let mut out = String::with_capacity(cues.len() * 64 + 16);
    out.push_str("WEBVTT\n");
    for cue in cues {
        let start = cue.start_ms.saturating_add(offset_ms).max(0);
        let end = cue.end_ms.saturating_add(offset_ms);
        if end <= start {
            continue;
        }
        out.push('\n');
        let _ = write!(out, "{} --> {}", vtt_time(start), vtt_time(end));
        if let Some(settings) = cue.settings.as_deref().filter(|s| !s.is_empty()) {
            out.push(' ');
            out.push_str(settings);
        }
        out.push('\n');
        // A blank line would end the cue early.
        for line in cue.text.lines().filter(|line| !line.trim().is_empty()) {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn vtt_time(ms: i64) -> String {
    let ms = ms.max(0);
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}

fn normalise_newlines(text: &str) -> String {
    text.trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

/// `01:02:03,456`, `01:02:03.456`, `02:03.456`, `1:02:03.4`: hours optional, any fraction.
fn parse_clock(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    let (clock, fraction) = match raw.rfind([',', '.']) {
        Some(at) => (&raw[..at], &raw[at + 1..]),
        None => (raw, ""),
    };
    let parts = clock
        .split(':')
        .map(|part| part.trim().parse::<i64>().ok())
        .collect::<Option<Vec<_>>>()?;
    let (hours, minutes, seconds) = match parts.as_slice() {
        [h, m, s] => (*h, *m, *s),
        [m, s] => (0, *m, *s),
        _ => return None,
    };
    if minutes < 0 || seconds < 0 || hours < 0 {
        return None;
    }
    let fraction_ms = if fraction.is_empty() {
        0
    } else {
        if !fraction.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let digits = &fraction[..fraction.len().min(3)];
        let value = digits.parse::<i64>().ok()?;
        value * 10_i64.pow(3 - digits.len() as u32)
    };
    Some(((hours * 60 + minutes) * 60 + seconds) * 1000 + fraction_ms)
}

/// A timing line: `start --> end` and whatever follows it.
fn parse_timing(line: &str) -> Option<(i64, i64, &str)> {
    let (start, rest) = line.split_once("-->")?;
    let rest = rest.trim_start();
    let (end, tail) = match rest.find(char::is_whitespace) {
        Some(at) => (&rest[..at], rest[at..].trim()),
        None => (rest, ""),
    };
    Some((parse_clock(start)?, parse_clock(end)?, tail))
}

// --- SubRip -------------------------------------------------------------------------------

/// SubRip: numbered blocks of a timing line and text. Numbers are optional and ignored, as is
/// SubRip's extended position (`X1:… Y1:…`) after the times.
pub fn parse_srt(text: &str) -> Vec<Cue> {
    let text = normalise_newlines(text);
    let mut cues = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let Some((start_ms, end_ms, _)) = parse_timing(line) else {
            continue;
        };
        let mut body = Vec::new();
        while let Some(next) = lines.peek() {
            if next.trim().is_empty() {
                break;
            }
            // A file with no blank line between blocks: the next number and timing start a cue.
            if parse_timing(next).is_some() {
                break;
            }
            body.push(*next);
            lines.next();
        }
        // A trailing block number belongs to the next cue.
        if body.len() > 1
            && body
                .last()
                .is_some_and(|last| last.trim().chars().all(|c| c.is_ascii_digit()))
            && lines
                .peek()
                .is_some_and(|next| parse_timing(next).is_some())
        {
            body.pop();
        }
        let raw = body.join("\n");
        let settings = position_setting(&raw);
        cues.push(Cue {
            start_ms,
            end_ms,
            text: sanitize_markup(&raw),
            settings,
        });
    }
    cues
}

/// ASS-style alignment codes carried in SubRip (`{\an8}`) or ASS text: 7, 8 and 9 are the top.
fn position_setting(raw: &str) -> Option<String> {
    let at = raw.find("\\an")?;
    match raw[at + 3..].chars().next() {
        Some('7' | '8' | '9') => Some("line:0".to_owned()),
        _ => None,
    }
}

/// Keep `<i>`, `<b>` and `<u>` (balanced), drop any other tag and `{…}` override block, escape
/// the rest.
fn sanitize_markup(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut open: Vec<char> = Vec::new();
    let mut chars = raw.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            '<' => {
                if let Some(close) = raw[at..].find('>') {
                    let tag = &raw[at + 1..at + close];
                    if looks_like_tag(tag) {
                        apply_tag(tag, &mut out, &mut open);
                        // Skip to after the '>'.
                        while let Some((next_at, _)) = chars.peek() {
                            if *next_at > at + close {
                                break;
                            }
                            chars.next();
                        }
                        continue;
                    }
                }
                out.push_str("&lt;");
            }
            '{' => {
                // `{\i1}` and friends; a brace that does not hold an override is text.
                if let Some(close) = raw[at..].find('}') {
                    let block = &raw[at + 1..at + close];
                    if block.starts_with('\\') {
                        apply_overrides(block, &mut out, &mut open);
                        while let Some((next_at, _)) = chars.peek() {
                            if *next_at > at + close {
                                break;
                            }
                            chars.next();
                        }
                        continue;
                    }
                }
                out.push('{');
            }
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '\\' => match chars.peek().map(|(_, next)| *next) {
                // ASS line breaks and hard spaces, also seen in SubRip made from ASS.
                Some('N' | 'n') => {
                    chars.next();
                    out.push('\n');
                }
                Some('h') => {
                    chars.next();
                    out.push(' ');
                }
                _ => out.push('\\'),
            },
            '\t' => out.push(' '),
            _ => out.push(c),
        }
    }
    while let Some(tag) = open.pop() {
        let _ = write!(out, "</{tag}>");
    }
    out.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

fn looks_like_tag(tag: &str) -> bool {
    let name = tag.trim_start_matches('/');
    !name.is_empty()
        && name.len() <= 200
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && !tag.contains('<')
}

fn apply_tag(tag: &str, out: &mut String, open: &mut Vec<char>) {
    let closing = tag.starts_with('/');
    let name = tag
        .trim_start_matches('/')
        .split(|c: char| c.is_whitespace() || c == '/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if name == "br" {
        out.push('\n');
        return;
    }
    let Some(kind) = (match name.as_str() {
        "i" | "em" => Some('i'),
        "b" | "strong" => Some('b'),
        "u" => Some('u'),
        _ => None,
    }) else {
        return;
    };
    set_style(kind, !closing, out, open);
}

fn set_style(kind: char, on: bool, out: &mut String, open: &mut Vec<char>) {
    if on {
        if !open.contains(&kind) {
            let _ = write!(out, "<{kind}>");
            open.push(kind);
        }
        return;
    }
    let Some(position) = open.iter().rposition(|tag| *tag == kind) else {
        return;
    };
    // Close what was opened inside it, then reopen it, so the markup stays nested.
    let inner = open.split_off(position + 1);
    for tag in inner.iter().rev() {
        let _ = write!(out, "</{tag}>");
    }
    let _ = write!(out, "</{kind}>");
    open.pop();
    for tag in inner {
        let _ = write!(out, "<{tag}>");
        open.push(tag);
    }
}

/// An ASS override block: `\i1`, `\b0`, `\u1` become markup; `\r` resets; the rest (colours,
/// fonts, positions, karaoke) is dropped.
fn apply_overrides(block: &str, out: &mut String, open: &mut Vec<char>) {
    for code in block.split('\\').filter(|code| !code.is_empty()) {
        let mut chars = code.chars();
        let Some(first) = chars.next() else { continue };
        let rest: String = chars.collect();
        match (first, rest.as_str()) {
            ('i' | 'b' | 'u', value) if value.chars().all(|c| c.is_ascii_digit()) => {
                // `\b700` is a weight: bold when it is heavier than normal.
                let on = match value {
                    "" | "0" => false,
                    "1" => true,
                    weight => weight.parse::<u32>().is_ok_and(|w| w > 400),
                };
                set_style(first, on, out, open);
            }
            ('r', _) => {
                while let Some(tag) = open.pop() {
                    let _ = write!(out, "</{tag}>");
                }
            }
            _ => {}
        }
    }
}

// --- Advanced SubStation ------------------------------------------------------------------

/// ASS and SSA: the `[Events]` section's `Dialogue:` lines, read by its `Format:` line. Drawings
/// (`\p1`) are left out; everything else about styling is dropped.
pub fn parse_ass(text: &str) -> Vec<Cue> {
    let text = normalise_newlines(text);
    let mut in_events = false;
    let mut format: Vec<String> = Vec::new();
    let mut cues = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_events = line.eq_ignore_ascii_case("[events]");
            continue;
        }
        if !in_events {
            continue;
        }
        if let Some(fields) = strip_prefix_ci(line, "format:") {
            format = fields
                .split(',')
                .map(|field| field.trim().to_ascii_lowercase())
                .collect();
            continue;
        }
        let Some(values) = strip_prefix_ci(line, "dialogue:") else {
            continue;
        };
        let columns = if format.is_empty() {
            // The v4+ default, for a file that leaves the Format line out.
            [
                "layer", "start", "end", "style", "name", "marginl", "marginr", "marginv",
                "effect", "text",
            ]
            .map(str::to_owned)
            .to_vec()
        } else {
            format.clone()
        };
        let fields = values
            .trim_start()
            .splitn(columns.len(), ',')
            .collect::<Vec<_>>();
        if fields.len() < columns.len() {
            continue;
        }
        let field = |name: &str| {
            columns
                .iter()
                .position(|column| column == name)
                .map(|index| fields[index])
        };
        let (Some(start), Some(end), Some(raw)) = (field("start"), field("end"), field("text"))
        else {
            continue;
        };
        let (Some(start_ms), Some(end_ms)) = (parse_clock(start), parse_clock(end)) else {
            continue;
        };
        if is_drawing(raw) {
            continue;
        }
        cues.push(Cue {
            start_ms,
            end_ms,
            text: sanitize_markup(raw),
            settings: position_setting(raw),
        });
    }
    cues
}

fn strip_prefix_ci<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    (line.len() >= prefix.len() && line[..prefix.len()].eq_ignore_ascii_case(prefix))
        .then(|| &line[prefix.len()..])
}

fn is_drawing(raw: &str) -> bool {
    let mut rest = raw;
    while let Some(at) = rest.find("\\p") {
        let after = &rest[at + 2..];
        if after.starts_with(|c: char| c.is_ascii_digit() && c != '0') {
            return true;
        }
        rest = after;
    }
    false
}

// --- WebVTT ---------------------------------------------------------------------------------

/// WebVTT: cue blocks, skipping the header, `NOTE`, `STYLE` and `REGION` blocks. Cue text is
/// reduced like any other source's; position settings are kept.
pub fn parse_vtt(text: &str) -> Vec<Cue> {
    let text = normalise_newlines(text);
    let mut cues = Vec::new();
    for block in text.split("\n\n") {
        let lines = block
            .lines()
            .skip_while(|line| line.trim().is_empty())
            .collect::<Vec<_>>();
        let Some(timing_at) = lines.iter().position(|line| line.contains("-->")) else {
            continue;
        };
        if lines.first().is_some_and(|first| {
            let first = first.trim_start();
            first.starts_with("NOTE") || first.starts_with("STYLE") || first.starts_with("REGION")
        }) {
            continue;
        }
        let Some((start_ms, end_ms, settings)) = parse_timing(lines[timing_at]) else {
            continue;
        };
        let raw = lines[timing_at + 1..].join("\n");
        cues.push(Cue {
            start_ms,
            end_ms,
            text: sanitize_vtt_markup(&raw),
            settings: clean_vtt_settings(settings),
        });
    }
    cues
}

/// WebVTT text is markup already: keep `i`, `b`, `u` and entities, drop classes, voices, ruby
/// and timestamps.
fn sanitize_vtt_markup(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find(['<', '&']) {
        out.push_str(&rest[..at].replace('>', "&gt;"));
        rest = &rest[at..];
        if rest.starts_with('&') {
            let entity_end = rest[1..].find(';').filter(|end| {
                *end <= 10
                    && rest[1..1 + end]
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '#')
            });
            match entity_end {
                Some(end) => {
                    out.push_str(&rest[..end + 2]);
                    rest = &rest[end + 2..];
                }
                None => {
                    out.push_str("&amp;");
                    rest = &rest[1..];
                }
            }
            continue;
        }
        let Some(close) = rest.find('>') else {
            out.push_str("&lt;");
            rest = &rest[1..];
            continue;
        };
        let tag = &rest[1..close];
        let name = tag
            .trim_start_matches('/')
            .split(['.', ' '])
            .next()
            .unwrap_or("");
        if matches!(name, "i" | "b" | "u") {
            if tag.starts_with('/') {
                let _ = write!(out, "</{name}>");
            } else {
                let _ = write!(out, "<{name}>");
            }
        }
        rest = &rest[close + 1..];
    }
    out.push_str(&rest.replace('>', "&gt;"));
    balance(&out)
}

/// Close what a cue opened and never closed, and drop closers with no opener.
fn balance(markup: &str) -> String {
    let mut out = String::with_capacity(markup.len());
    let mut open: Vec<char> = Vec::new();
    let mut rest = markup;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        let Some(close) = rest.find('>') else {
            out.push_str(rest);
            rest = "";
            break;
        };
        let tag = &rest[1..close];
        let closing = tag.starts_with('/');
        if let Some(kind) = tag.trim_start_matches('/').chars().next() {
            set_style(kind, !closing, &mut out, &mut open);
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    while let Some(tag) = open.pop() {
        let _ = write!(out, "</{tag}>");
    }
    out.trim().to_owned()
}

/// Only the settings every browser knows, with values that cannot break the timing line.
fn clean_vtt_settings(settings: &str) -> Option<String> {
    let kept = settings
        .split_whitespace()
        .filter(|setting| {
            let Some((name, value)) = setting.split_once(':') else {
                return false;
            };
            matches!(name, "line" | "position" | "size" | "align" | "vertical")
                && !value.is_empty()
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '%' | '.' | '-' | ','))
        })
        .collect::<Vec<_>>();
    (!kept.is_empty()).then(|| kept.join(" "))
}

// --- MicroDVD -----------------------------------------------------------------------------

/// MicroDVD: `{start}{end}text`, in frames, `|` between lines. A first cue of `{1}{1}23.976`
/// gives the frame rate; otherwise `fps` (the video's), else 23.976.
pub fn parse_microdvd(text: &str, fps: Option<f64>) -> Vec<Cue> {
    let text = normalise_newlines(text);
    let mut rate = fps.filter(|fps| fps.is_finite() && *fps > 1.0 && *fps < 200.0);
    let mut cues = Vec::new();
    for line in text.lines() {
        let Some((start, rest)) = braced_number(line.trim()) else {
            continue;
        };
        let (end, raw) = braced_number(rest).unwrap_or((start, rest));
        if cues.is_empty()
            && start <= 1
            && end <= 1
            && let Ok(declared) = raw.trim().parse::<f64>()
            && declared > 1.0
            && declared < 200.0
        {
            rate = Some(declared);
            continue;
        }
        let fps = rate.unwrap_or(DEFAULT_MICRODVD_FPS);
        let to_ms = |frame: i64| (frame as f64 * 1000.0 / fps).round() as i64;
        let mut lines = Vec::new();
        for part in raw.split('|') {
            let (italic, part) = strip_microdvd_codes(part);
            let clean = sanitize_markup(&part);
            lines.push(if italic && !clean.is_empty() {
                format!("<i>{clean}</i>")
            } else {
                clean
            });
        }
        cues.push(Cue {
            start_ms: to_ms(start),
            end_ms: to_ms(end),
            text: lines.join("\n"),
            settings: None,
        });
    }
    cues
}

fn braced_number(text: &str) -> Option<(i64, &str)> {
    let rest = text.strip_prefix('{')?;
    let close = rest.find('}')?;
    let number = rest[..close].trim().parse::<i64>().ok()?;
    Some((number, &rest[close + 1..]))
}

/// Leading `{y:i}`-style codes: italics are kept, the rest (colours, fonts) dropped.
fn strip_microdvd_codes(part: &str) -> (bool, String) {
    let mut italic = false;
    let mut rest = part.trim_start_matches('/');
    let slash = rest.len() != part.len();
    while let Some(inner) = rest.strip_prefix('{') {
        let Some(close) = inner.find('}') else { break };
        let code = &inner[..close];
        if !code.contains(':') {
            break;
        }
        if code.to_ascii_lowercase().starts_with("y:") && code.to_ascii_lowercase().contains('i') {
            italic = true;
        }
        rest = &inner[close + 1..];
    }
    (italic || slash, rest.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srt_converts_with_markup_kept_and_escaped() {
        let srt = "\u{feff}1\r\n00:00:01,000 --> 00:00:02,500\r\n<i>Hello</i> & <font color=\"red\">world</font>\r\nline two\r\n\r\n2\r\n00:01:02,03 --> 00:01:04,000 X1:10 X2:20 Y1:5 Y2:9\r\n{\\an8}Top <b>bold\r\n\r\n";
        let cues = parse(TextFormat::Srt, srt, None).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start_ms, 1000);
        assert_eq!(cues[0].end_ms, 2500);
        assert_eq!(cues[0].text, "<i>Hello</i> &amp; world\nline two");
        assert_eq!(cues[1].start_ms, 62_030);
        assert_eq!(cues[1].text, "Top <b>bold</b>");
        assert_eq!(cues[1].settings.as_deref(), Some("line:0"));
        let vtt = to_vtt(&cues, 0);
        assert_eq!(
            vtt,
            "WEBVTT\n\n00:00:01.000 --> 00:00:02.500\n<i>Hello</i> &amp; world\nline two\n\n00:01:02.030 --> 00:01:04.000 line:0\nTop <b>bold</b>\n"
        );
    }

    #[test]
    fn srt_without_blank_lines_and_with_script_is_safe() {
        let srt = "1\n00:00:01,000 --> 00:00:02,000\nOne\n2\n00:00:03,000 --> 00:00:04,000\n<script>alert(1)</script> a<b\n";
        let cues = parse_srt(srt);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "One");
        assert_eq!(cues[1].text, "alert(1) a&lt;b");
    }

    #[test]
    fn a_cue_cannot_end_the_file_early_or_forge_a_timing() {
        let srt = "1\n00:00:01,000 --> 00:00:02,000\nOne\n\n\nTwo --> three\n";
        let cues = parse_srt(srt);
        let vtt = to_vtt(&cues, 0);
        assert!(!vtt.contains("three"), "{vtt}");
        let cue = Cue {
            start_ms: 0,
            end_ms: 1000,
            text: "a\n\nb".into(),
            settings: None,
        };
        assert_eq!(
            to_vtt(&[cue], 0),
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\na\nb\n"
        );
    }

    #[test]
    fn offset_moves_cues_and_drops_those_before_zero() {
        let cues = vec![
            Cue {
                start_ms: 500,
                end_ms: 900,
                text: "gone".into(),
                settings: None,
            },
            Cue {
                start_ms: 800,
                end_ms: 2000,
                text: "clamped".into(),
                settings: None,
            },
            Cue {
                start_ms: 3_600_000,
                end_ms: 3_601_500,
                text: "hour".into(),
                settings: None,
            },
        ];
        let vtt = to_vtt(&cues, -1000);
        assert_eq!(
            vtt,
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nclamped\n\n00:59:59.000 --> 01:00:00.500\nhour\n"
        );
        assert!(to_vtt(&cues, 250).contains("00:00:00.750 --> 00:00:01.150"));
    }

    #[test]
    fn ass_reads_events_by_format_and_drops_styling() {
        let ass = "[Script Info]\nTitle: x\n\n[V4+ Styles]\nFormat: Name, Fontname\nStyle: Default,Arial\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nComment: 0,0:00:00.00,0:00:05.00,Default,,0,0,0,,ignored\nDialogue: 0,0:00:01.50,0:00:03.20,Default,,0,0,0,,{\\i1}Hello{\\i0}, there\\Nfriend {\\c&H00FF00&}green\nDialogue: 0,0:00:04.00,0:00:05.00,Sign,,0,0,0,,{\\an8\\pos(10,10)}At the top\nDialogue: 0,0:00:06.00,0:00:07.00,Default,,0,0,0,,{\\p1}m 0 0 l 100 0 100 100{\\p0}\nDialogue: 0,0:00:02.00,0:00:02.50,Default,,0,0,0,,{\\b1}Bold, {\\i1}both{\\b0} italic\n";
        let cues = parse(TextFormat::Ass, ass, None).unwrap();
        assert_eq!(cues.len(), 3);
        assert_eq!(cues[0].start_ms, 1500);
        assert_eq!(cues[0].end_ms, 3200);
        assert_eq!(cues[0].text, "<i>Hello</i>, there\nfriend green");
        // Sorted by start: the bold line comes second.
        assert_eq!(cues[1].text, "<b>Bold, <i>both</i></b><i> italic</i>");
        assert_eq!(cues[2].text, "At the top");
        assert_eq!(cues[2].settings.as_deref(), Some("line:0"));
    }

    #[test]
    fn ssa_with_marked_column_reads_text() {
        let ssa = "[Events]\nFormat: Marked, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: Marked=0,0:00:01.00,0:00:02.00,*Default,NTP,0000,0000,0000,!Effect,Old style\n";
        let cues = parse(TextFormat::Ass, ssa, None).unwrap();
        assert_eq!(cues[0].text, "Old style");
    }

    #[test]
    fn vtt_input_keeps_positions_and_strips_classes() {
        let vtt = "WEBVTT - title\n\nNOTE a note\n\nSTYLE\n::cue { color: red }\n\nid-1\n00:01.000 --> 00:02.000 line:0 align:start foo:bar\n<c.yellow>Hi</c> <v Bob>there</v> &amp; <i>you\n\n00:00:03.000 --> 00:00:04.000\nnext &nbsp; <b>bold</b>\n";
        let cues = parse(TextFormat::Vtt, vtt, None).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Hi there &amp; <i>you</i>");
        assert_eq!(cues[0].settings.as_deref(), Some("line:0 align:start"));
        assert_eq!(cues[1].text, "next &nbsp; <b>bold</b>");
    }

    #[test]
    fn microdvd_uses_declared_frame_rate_and_line_breaks() {
        let sub = "{1}{1}25.000\n{25}{50}Hello|{y:i}world\n{75}{100}/Slashed italic\n";
        let cues = parse(TextFormat::MicroDvd, sub, None).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start_ms, 1000);
        assert_eq!(cues[0].end_ms, 2000);
        assert_eq!(cues[0].text, "Hello\n<i>world</i>");
        assert_eq!(cues[1].text, "<i>Slashed italic</i>");
        let at_video_rate = parse_microdvd("{24}{48}x", Some(24.0));
        assert_eq!(at_video_rate[0].start_ms, 1000);
    }

    #[test]
    fn empty_or_garbage_is_an_error() {
        assert_eq!(
            parse(TextFormat::Srt, "nothing here", None),
            Err(ParseError::Empty)
        );
        assert_eq!(parse(TextFormat::Ass, "", None), Err(ParseError::Empty));
    }

    #[test]
    fn decode_reads_boms_and_guesses_code_pages() {
        assert_eq!(decode_text(b"\xEF\xBB\xBFcaf\xC3\xA9"), "café");
        assert_eq!(decode_text(&[0xFF, 0xFE, b'h', 0, b'i', 0]), "hi");
        // Windows-1252 é and the curly quotes French and Spanish files are full of.
        let latin = b"1\n00:00:01,000 --> 00:00:02,000\nC\x92est l\xE0, d\xE9j\xE0 \x93vu\x94 et \xE9t\xE9 pass\xE9\n";
        let decoded = decode_text(latin);
        assert!(
            decoded.contains("C’est là, déjà “vu” et été passé"),
            "{decoded}"
        );
    }

    #[test]
    fn clock_forms() {
        assert_eq!(parse_clock("01:02:03,456"), Some(3_723_456));
        assert_eq!(parse_clock("1:02:03.4"), Some(3_723_400));
        assert_eq!(parse_clock("02:03.45"), Some(123_450));
        assert_eq!(parse_clock("0:00:01.123456"), Some(1123));
        assert_eq!(parse_clock("aa:00:01"), None);
    }
}
