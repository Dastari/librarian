//! LRC lyrics: `[mm:ss.xx]` before each line.
//!
//! Every synced source (an `.lrc` sidecar, an embedded LRC in a `LYRICS` tag, an ID3 `SYLT` frame,
//! LRCLIB's `syncedLyrics`) is normalised to one form before it is stored: one `[mm:ss.cc]text`
//! line per timestamp, sorted, with the file's `[offset:]` applied and enhanced-LRC word times
//! removed. The player then only needs to split lines.

use std::fmt::Write as _;

/// One line of synced lyrics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricLine {
    pub start_ms: i64,
    pub text: String,
}

/// What an LRC file holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lrc {
    pub lines: Vec<LyricLine>,
    /// `[la:]`, when the file names its language.
    pub language: Option<String>,
}

fn parse_stamp(raw: &str) -> Option<i64> {
    let (minutes, rest) = raw.split_once(':')?;
    let minutes = minutes.trim().parse::<i64>().ok()?;
    let (seconds, fraction) = match rest.split_once(['.', ':']) {
        Some((seconds, fraction)) => (seconds, fraction),
        None => (rest, ""),
    };
    let seconds = seconds.trim().parse::<i64>().ok()?;
    if !(0..60).contains(&seconds) || minutes < 0 {
        return None;
    }
    let fraction_ms = if fraction.is_empty() {
        0
    } else {
        let digits = &fraction[..fraction.len().min(3)];
        if !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        digits.parse::<i64>().ok()? * 10_i64.pow(3 - digits.len() as u32)
    };
    Some((minutes * 60 + seconds) * 1000 + fraction_ms)
}

/// Remove enhanced-LRC word times (`<00:12.34>`).
fn strip_word_times(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        match rest[at..].find('>') {
            Some(close) if parse_stamp(&rest[at + 1..at + close]).is_some() => {
                rest = &rest[at + close + 1..];
            }
            _ => {
                out.push('<');
                rest = &rest[at + 1..];
            }
        }
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse LRC text. Lines without a time are ignored; a file with no timed line has no lines.
pub fn parse(text: &str) -> Lrc {
    let mut lrc = Lrc::default();
    let mut offset_ms = 0_i64;
    let mut timed: Vec<LyricLine> = Vec::new();
    for raw in text.trim_start_matches('\u{feff}').lines() {
        let mut rest = raw.trim();
        let mut stamps = Vec::new();
        while let Some(inner) = rest.strip_prefix('[') {
            let Some(close) = inner.find(']') else { break };
            let tag = &inner[..close];
            rest = inner[close + 1..].trim_start();
            if let Some(stamp) = parse_stamp(tag) {
                stamps.push(stamp);
                continue;
            }
            if let Some((name, value)) = tag.split_once(':') {
                match name.trim().to_ascii_lowercase().as_str() {
                    "offset" => offset_ms = value.trim().parse::<i64>().unwrap_or(0),
                    "la" | "lang" | "language" if !value.trim().is_empty() => {
                        lrc.language = Some(value.trim().to_owned())
                    }
                    _ => {}
                }
            }
        }
        if stamps.is_empty() {
            continue;
        }
        let text = strip_word_times(rest);
        for stamp in stamps {
            timed.push(LyricLine {
                start_ms: stamp,
                text: text.clone(),
            });
        }
    }
    // A positive offset brings the lyrics in sooner.
    for line in &mut timed {
        line.start_ms = (line.start_ms - offset_ms).max(0);
    }
    timed.sort_by_key(|line| line.start_ms);
    lrc.lines = timed;
    lrc
}

/// Whether text is synced lyrics: at least two timed lines.
pub fn is_synced(text: &str) -> bool {
    parse(text).lines.len() >= 2
}

/// The stored form: `[mm:ss.cc]text`, one line each, sorted.
pub fn to_lrc(lines: &[LyricLine]) -> String {
    let mut out = String::new();
    for line in lines {
        let ms = line.start_ms.max(0);
        let _ = writeln!(
            out,
            "[{:02}:{:02}.{:02}]{}",
            ms / 60_000,
            (ms / 1000) % 60,
            (ms % 1000) / 10,
            line.text.trim()
        );
    }
    out
}

/// Normalise any synced lyrics text to the stored form, or `None` when it is not synced.
pub fn normalise(text: &str) -> Option<String> {
    let lrc = parse(text);
    (lrc.lines.len() >= 2).then(|| to_lrc(&lrc.lines))
}

/// The words without times, for plain display: one line per lyric line, repeats kept.
pub fn plain_from_lines(lines: &[LyricLine]) -> String {
    lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

/// Plain lyrics as stored: trimmed, line endings normalised, no run of more than one blank line.
pub fn normalise_plain(text: &str) -> Option<String> {
    let text = text
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut out = Vec::new();
    let mut blank = false;
    for line in text.lines().map(str::trim_end) {
        if line.trim().is_empty() {
            if !blank && !out.is_empty() {
                out.push(String::new());
            }
            blank = true;
            continue;
        }
        blank = false;
        out.push(line.to_owned());
    }
    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    (!out.is_empty()).then(|| out.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lrc_lines_offsets_and_repeats() {
        let text = "\u{feff}[ar:Someone]\n[ti:Song]\n[la:en]\n[offset:+500]\n[00:12.00]First line\n[00:15.30][01:02.5]Chorus <00:15.80>with <00:16.10>words\n\n[00:20.123]  Third   line  \nno time here\n[bad]x\n";
        let lrc = parse(text);
        assert_eq!(lrc.language.as_deref(), Some("en"));
        assert_eq!(
            lrc.lines,
            vec![
                LyricLine {
                    start_ms: 11_500,
                    text: "First line".into()
                },
                LyricLine {
                    start_ms: 14_800,
                    text: "Chorus with words".into()
                },
                LyricLine {
                    start_ms: 19_623,
                    text: "Third line".into()
                },
                LyricLine {
                    start_ms: 62_000,
                    text: "Chorus with words".into()
                },
            ]
        );
        assert_eq!(
            to_lrc(&lrc.lines),
            "[00:11.50]First line\n[00:14.80]Chorus with words\n[00:19.62]Third line\n[01:02.00]Chorus with words\n"
        );
        assert_eq!(plain_from_lines(&lrc.lines).lines().count(), 4);
    }

    #[test]
    fn plain_text_is_not_synced() {
        assert!(!is_synced("Just words\nand more words"));
        assert!(normalise("Just words").is_none());
        assert!(is_synced("[00:01.00]a\n[00:02.00]b"));
        // Blank timed lines (instrumental breaks) are kept as lines.
        let lrc = parse("[00:01.00]a\n[00:05.00]\n[00:09.00]b");
        assert_eq!(lrc.lines[1].text, "");
    }

    #[test]
    fn plain_lyrics_are_tidied() {
        assert_eq!(
            normalise_plain("\r\n\r\nOne\r\nTwo  \r\n\r\n\r\n\r\nThree\r\n\r\n").as_deref(),
            Some("One\nTwo\n\nThree")
        );
        assert_eq!(normalise_plain("  \n\n "), None);
    }

    #[test]
    fn three_digit_fractions_and_negative_offset() {
        let lrc = parse("[offset:-250]\n[00:01.005]x\n[00:02]y");
        assert_eq!(lrc.lines[0].start_ms, 1255);
        assert_eq!(lrc.lines[1].start_ms, 2250);
    }
}
