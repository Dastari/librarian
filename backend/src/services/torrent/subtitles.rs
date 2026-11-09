//! Subtitle files bundled in a torrent (Q112): which video each one belongs to, and its language.
//!
//! Releases carry subtitles in a few common layouts:
//!
//! - next to the video, sharing its name: `Show.S01E01.mkv` + `Show.S01E01.en.srt`;
//! - in a `Subs/` folder beside a single video: `Movie/Subs/2_English.srt`;
//! - in a folder per episode in a season pack: `Show.S01/Subs/Show.S01E01/3_English.srt`;
//! - VobSub pairs, `.idx` + `.sub`, which only work together.
//!
//! [`link_subtitles`] fills [`ContentFile::belongs_to`] (the index of the video) and
//! [`ContentFile::language`] so a selection of videos can take their subtitles along, and a
//! recommendation can say "includes English subtitles". Pure code: no I/O. Importing subtitles into
//! a library is not done here.

use std::collections::HashMap;

use super::inspect::{ContentFile, FileKind};

/// Subtitle file extensions: text (`srt`, `ass`, `ssa`, `vtt`) and image (`idx`/`sub`, `sup`).
pub const SUBTITLE_EXTENSIONS: &[&str] = &["srt", "ass", "ssa", "vtt", "sub", "idx", "sup"];

/// Languages by the codes and names releases use, to the name shown.
const LANGUAGES: &[(&[&str], &str)] = &[
    (&["en", "eng", "english"], "English"),
    (&["fr", "fre", "fra", "french", "francais"], "French"),
    (&["de", "ger", "deu", "german", "deutsch"], "German"),
    (
        &["es", "spa", "spanish", "espanol", "castellano"],
        "Spanish",
    ),
    (&["it", "ita", "italian", "italiano"], "Italian"),
    (
        &["pt", "por", "portuguese", "portugues", "brazilian", "ptbr"],
        "Portuguese",
    ),
    (&["nl", "dut", "nld", "dutch", "nederlands"], "Dutch"),
    (&["sv", "swe", "swedish", "svenska"], "Swedish"),
    (&["no", "nor", "nob", "norwegian", "norsk"], "Norwegian"),
    (&["da", "dan", "danish", "dansk"], "Danish"),
    (&["fi", "fin", "finnish", "suomi"], "Finnish"),
    (&["pl", "pol", "polish", "polski"], "Polish"),
    (&["ru", "rus", "russian"], "Russian"),
    (&["ja", "jpn", "japanese"], "Japanese"),
    (&["ko", "kor", "korean"], "Korean"),
    (
        &["zh", "chi", "zho", "chinese", "chs", "cht", "mandarin"],
        "Chinese",
    ),
    (&["ar", "ara", "arabic"], "Arabic"),
    (&["he", "heb", "hebrew"], "Hebrew"),
    (&["tr", "tur", "turkish"], "Turkish"),
    (&["el", "gre", "ell", "greek"], "Greek"),
    (&["cs", "cze", "ces", "czech"], "Czech"),
    (&["hu", "hun", "hungarian"], "Hungarian"),
    (&["ro", "rum", "ron", "romanian"], "Romanian"),
    (&["hin", "hindi"], "Hindi"),
    (&["th", "tha", "thai"], "Thai"),
    (&["vi", "vie", "vietnamese"], "Vietnamese"),
    (&["id", "ind", "indonesian"], "Indonesian"),
    (&["uk", "ukr", "ukrainian"], "Ukrainian"),
];

fn extension(path: &str) -> String {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

/// Whether a path is a subtitle file.
pub fn is_subtitle(path: &str) -> bool {
    SUBTITLE_EXTENSIONS.contains(&extension(path).as_str())
}

fn stem_of(path: &str) -> &str {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

fn folders(path: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = path.split(['/', '\\']).collect();
    parts.pop();
    parts
}

/// The language a subtitle's name gives: `2_English`, `Show.S01E01.en`, `English.forced`,
/// `Subs/eng/...`. Short codes count only as the last word of the name (or a folder named just the
/// code), so `Show.It.Follows` is not Italian. `None` when nothing says.
pub fn subtitle_language(path: &str) -> Option<String> {
    let lookup = |word: &str, long_only: bool| -> Option<&'static str> {
        let word = word.to_ascii_lowercase();
        if long_only && word.len() <= 3 {
            return None;
        }
        LANGUAGES
            .iter()
            .find(|(names, _)| names.contains(&word.as_str()))
            .map(|(_, name)| *name)
    };
    let stem = stem_of(path);
    let words: Vec<&str> = stem
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    // Trailing tags that are not the language: `.forced`, `.sdh`, `.cc`, `.default`.
    let ignorable = |word: &str| {
        matches!(
            word.to_ascii_lowercase().as_str(),
            "forced" | "sdh" | "cc" | "hi" | "default" | "full" | "subs" | "sub"
        ) || word.chars().all(|c| c.is_ascii_digit())
    };
    let meaningful: Vec<&str> = words
        .iter()
        .rev()
        .copied()
        .skip_while(|word| ignorable(word))
        .collect();
    if let Some(last) = meaningful.first()
        && let Some(language) = lookup(last, false)
    {
        return Some(language.to_string());
    }
    // `2_English`, `English (SDH)`: a full language name anywhere in the name.
    if let Some(language) = words.iter().find_map(|word| lookup(word, true)) {
        return Some(language.to_string());
    }
    // A folder named for the language: `Subs/English/1.srt`, `Subs/eng/1.srt`.
    folders(path)
        .iter()
        .rev()
        .take(2)
        .find_map(|folder| lookup(folder, false))
        .map(str::to_string)
}

/// Whether a folder is a subtitles folder (`Subs`, `Subtitles`, `Sub`).
fn is_subs_folder(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "subs" | "sub" | "subtitles" | "subtitle" | "subpack"
    )
}

/// The video a subtitle belongs to, as an index into `videos`, by (in order) a shared name, a
/// `Subs/<video stem>/` folder, the same episode, or a `Subs/` folder beside a single video.
pub fn owner_video(subtitle: &ContentFile, videos: &[&ContentFile]) -> Option<usize> {
    let sub_stem = stem_of(&subtitle.path).to_ascii_lowercase();
    // `Video.en.srt`, `Video.srt`: the longest video stem the subtitle's name starts with.
    let by_name = videos
        .iter()
        .enumerate()
        .filter(|(_, video)| {
            let stem = video.stem().to_ascii_lowercase();
            sub_stem == stem
                || sub_stem
                    .strip_prefix(stem.as_str())
                    .is_some_and(|rest| rest.starts_with(['.', '_', '-', ' ', '[', '(']))
        })
        .max_by_key(|(_, video)| video.stem().len())
        .map(|(index, _)| index);
    if by_name.is_some() {
        return by_name;
    }
    let sub_folders = folders(&subtitle.path);
    // `Subs/<video stem>/2_English.srt`.
    if let Some(parent) = sub_folders.last() {
        let parent = parent.to_ascii_lowercase();
        if let Some(index) = videos
            .iter()
            .position(|video| video.stem().to_ascii_lowercase() == parent)
        {
            return Some(index);
        }
    }
    // The same episode: `Subs/S01E03.English.srt`.
    if !subtitle.episodes.is_empty() {
        let same: Vec<usize> = videos
            .iter()
            .enumerate()
            .filter(|(_, video)| {
                (subtitle.season.is_none() || video.season == subtitle.season)
                    && subtitle
                        .episodes
                        .iter()
                        .any(|episode| video.episodes.contains(episode))
            })
            .map(|(index, _)| index)
            .collect();
        if same.len() == 1 {
            return Some(same[0]);
        }
    }
    // `Movie/Subs/2_English.srt`: a subtitles folder whose parent holds exactly one video.
    if let Some(position) = sub_folders
        .iter()
        .rposition(|folder| is_subs_folder(folder))
    {
        let base = &sub_folders[..position];
        let beside: Vec<usize> = videos
            .iter()
            .enumerate()
            .filter(|(_, video)| folders(&video.path).as_slice() == base)
            .map(|(index, _)| index)
            .collect();
        if beside.len() == 1 {
            return Some(beside[0]);
        }
    }
    // A torrent with one video: every subtitle is its.
    (videos.len() == 1).then_some(0)
}

/// Fill in each subtitle's language and the torrent file index of the video it belongs to. Safe to
/// run again (old cached readings are linked when they are loaded).
pub fn link_subtitles(files: &mut [ContentFile]) {
    let videos: Vec<ContentFile> = files
        .iter()
        .filter(|file| file.kind == FileKind::Video)
        .cloned()
        .collect();
    let video_refs: Vec<&ContentFile> = videos.iter().collect();
    for file in files.iter_mut() {
        if file.kind != FileKind::Subtitle {
            continue;
        }
        if file.language.is_none() {
            file.language = subtitle_language(&file.path);
        }
        file.belongs_to = owner_video(file, &video_refs).map(|position| video_refs[position].index);
    }
    // A VobSub pair goes together: an `.idx` without an owner takes its `.sub`'s, and back.
    let mut by_stem: HashMap<String, Option<usize>> = HashMap::new();
    for file in files.iter() {
        let ext = extension(&file.path);
        if file.kind == FileKind::Subtitle && (ext == "idx" || ext == "sub") {
            let key = pair_key(&file.path);
            let entry = by_stem.entry(key).or_insert(None);
            if entry.is_none() {
                *entry = file.belongs_to;
            }
        }
    }
    for file in files.iter_mut() {
        let ext = extension(&file.path);
        if file.kind == FileKind::Subtitle
            && (ext == "idx" || ext == "sub")
            && file.belongs_to.is_none()
        {
            file.belongs_to = by_stem.get(&pair_key(&file.path)).copied().flatten();
        }
    }
}

/// `folder/stem` without the extension, lower case: what an `.idx` and its `.sub` share.
pub fn pair_key(path: &str) -> String {
    path.rsplit_once('.')
        .map_or(path, |(head, _)| head)
        .to_ascii_lowercase()
}

/// The subtitle languages of the chosen videos (all videos when `chosen` is `None`), each once, in
/// a stable order with English first. Subtitles whose language is not known count as "unknown".
pub fn languages_for(files: &[ContentFile], chosen: Option<&[usize]>) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for file in files.iter().filter(|file| file.kind == FileKind::Subtitle) {
        let counts = match (chosen, file.belongs_to) {
            (None, _) => true,
            (Some(chosen), Some(owner)) => chosen.contains(&owner),
            (Some(_), None) => false,
        };
        if !counts {
            continue;
        }
        let language = file
            .language
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        if !found.contains(&language) {
            found.push(language);
        }
    }
    found.sort_by_key(|language| match language.as_str() {
        "English" => (0, String::new()),
        "unknown" => (2, String::new()),
        other => (1, other.to_string()),
    });
    found
}

/// "Includes English subtitles", "Includes English, French and 3 more subtitles", or `None`.
pub fn describe_languages(languages: &[String]) -> Option<String> {
    let named: Vec<&String> = languages
        .iter()
        .filter(|language| language.as_str() != "unknown")
        .collect();
    if named.is_empty() {
        return (!languages.is_empty()).then(|| "Includes subtitles".to_string());
    }
    let shown = match named.len() {
        1 => named[0].clone(),
        2 => format!("{} and {}", named[0], named[1]),
        3 => format!("{}, {} and {}", named[0], named[1], named[2]),
        more => format!("{}, {} and {} more", named[0], named[1], more - 2),
    };
    Some(format!("Includes {shown} subtitles"))
}

#[cfg(test)]
#[path = "subtitles_tests.rs"]
mod tests;
