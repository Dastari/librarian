//! Which subtitle files belong to which video, and what their names say about them.
//!
//! A sidecar is found where players look for one:
//! - beside the video, named after it: `Movie (2020).srt`, `Movie (2020).en.forced.srt`,
//!   `Show.S01E01.eng.sdh.srt`, `Movie (2020)_English.srt`;
//! - in a `Subs`/`Subtitles` folder beside the video, named after it, or named only by language
//!   (`Subs/2_English.srt`) when that folder's parent holds a single video;
//! - in a folder named after the video inside `Subs` (`Subs/Show.S01E01/3_English.srt`), as TV
//!   season packs ship them.
//!
//! When two videos' names both prefix a subtitle (`Movie.mkv` and `Movie.Extended.mkv`), the
//! longer name wins. A VobSub pair (`.idx` with its `.sub`) is one bitmap subtitle, named by the
//! `.idx`; a `.sub` alone is MicroDVD text.
//!
//! Everything here works on path strings, so local paths and `smb://` addresses behave alike.

use std::collections::{HashMap, HashSet};

use crate::services::library_storage::StoragePath;

use super::language;

/// Extensions a sidecar subtitle can have.
pub const SIDECAR_EXTENSIONS: &[&str] = &["srt", "vtt", "ass", "ssa", "sub", "idx"];

/// Folders that hold a release's subtitles.
const SUBS_FOLDERS: &[&str] = &["subs", "subtitles", "sub", "subtitle"];

pub fn is_sidecar_extension(extension: &str) -> bool {
    SIDECAR_EXTENSIONS
        .iter()
        .any(|known| extension.eq_ignore_ascii_case(known))
}

pub fn is_subs_folder(name: &str) -> bool {
    SUBS_FOLDERS
        .iter()
        .any(|known| name.eq_ignore_ascii_case(known))
}

/// What a sidecar's name says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NameFacts {
    /// ISO 639-2/B, as stored for embedded streams.
    pub language: Option<&'static str>,
    pub forced: bool,
    pub hearing_impaired: bool,
    pub default: bool,
    /// Words that are neither a language nor a flag: `Commentary`, `Signs & Songs`.
    pub title: Option<String>,
}

/// One sidecar subtitle of a video.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidecarFile {
    /// The subtitle's own path; the `.idx` of a VobSub pair.
    pub path: String,
    /// The codec stored on the row, as ffprobe names it.
    pub codec: &'static str,
    /// A picture format, which needs burning in.
    pub bitmap: bool,
    pub facts: NameFacts,
}

/// Read the part of a sidecar's name after the video's (`en.forced`, `English_SDH`, `2_eng`).
pub fn parse_name_tokens(rest: &str) -> NameFacts {
    let mut facts = NameFacts::default();
    let mut title = Vec::new();
    let tokens = rest
        .split(['.', '_', ' ', '(', ')', '[', ']', ','])
        .flat_map(split_hyphen)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    for token in &tokens {
        let lower = token.to_lowercase();
        match lower.as_str() {
            "forced" | "foreign" | "forcedonly" => facts.forced = true,
            "sdh" | "hi" | "cc" | "hoh" | "hearing" | "impaired" | "deaf" => {
                facts.hearing_impaired = true
            }
            "default" => facts.default = true,
            "full" | "subs" | "subtitles" | "sub" | "srt" => {}
            _ if lower.chars().all(|c| c.is_ascii_digit()) => {}
            _ => {
                if facts.language.is_none()
                    && let Some(code) = language::from_file_token(&lower)
                {
                    facts.language = Some(code);
                } else {
                    title.push(token.to_string());
                }
            }
        }
    }
    // A name of nothing but an ambiguous code (`Movie.it.srt`) still says Italian.
    if facts.language.is_none()
        && title.len() == 1
        && let Some(code) = language::normalize(&title[0])
    {
        facts.language = Some(code);
        title.clear();
    }
    if !title.is_empty() {
        facts.title = Some(title.join(" "));
    }
    facts
}

/// `pt-BR` stays one token; `English-SDH` is two.
fn split_hyphen(token: &str) -> Vec<&str> {
    if language::normalize(token).is_some() {
        return vec![token];
    }
    token.split('-').collect()
}

fn split_name(name: &str) -> (&str, String) {
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem, extension.to_ascii_lowercase()),
        _ => (name, String::new()),
    }
}

/// The text after `video_stem` in `stem` when `stem` is the video's name or the video's name and
/// a separator, compared without case.
fn after_video_name<'a>(stem: &'a str, video_stem: &str) -> Option<&'a str> {
    if stem.len() < video_stem.len() || !stem.is_char_boundary(video_stem.len()) {
        return None;
    }
    let (head, rest) = stem.split_at(video_stem.len());
    if !head.eq_ignore_ascii_case(video_stem) {
        return None;
    }
    if rest.is_empty() {
        return Some(rest);
    }
    let first = rest.chars().next()?;
    matches!(first, '.' | '_' | '-' | ' ').then(|| &rest[first.len_utf8()..])
}

/// Videos by the folder they are in, for placing subtitles.
pub struct VideoIndex {
    by_folder: HashMap<String, Vec<(String, String)>>,
}

impl VideoIndex {
    /// `videos` are full paths of video files.
    pub fn new<I, S>(videos: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut by_folder: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for video in videos {
            let path = StoragePath::new(video.as_ref());
            let (Some(folder), Some(name)) = (path.parent(), path.file_name()) else {
                continue;
            };
            let (stem, _) = split_name(&name);
            by_folder
                .entry(folder.into_string())
                .or_default()
                .push((video.as_ref().to_owned(), stem.to_owned()));
        }
        Self { by_folder }
    }

    fn in_folder(&self, folder: &StoragePath) -> &[(String, String)] {
        self.by_folder
            .get(folder.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// The video whose name prefixes `stem` the longest, and what follows it.
    fn by_prefix<'a>(&'a self, folder: &StoragePath, stem: &'a str) -> Option<(&'a str, &'a str)> {
        self.in_folder(folder)
            .iter()
            .filter_map(|(video, video_stem)| {
                after_video_name(stem, video_stem)
                    .map(|rest| (video.as_str(), video_stem.len(), rest))
            })
            .max_by_key(|(_, length, _)| *length)
            .map(|(video, _, rest)| (video, rest))
    }

    /// Which video a subtitle at `path` belongs to, and the part of its name that describes it.
    fn place<'a>(&'a self, path: &StoragePath, stem: &'a str) -> Option<(&'a str, &'a str)> {
        let folder = path.parent()?;
        if let Some(found) = self.by_prefix(&folder, stem) {
            return Some(found);
        }
        let folder_name = folder.file_name()?;
        if is_subs_folder(&folder_name) {
            let anchor = folder.parent()?;
            if let Some(found) = self.by_prefix(&anchor, stem) {
                return Some(found);
            }
            return match self.in_folder(&anchor) {
                [(video, _)] => Some((video.as_str(), stem)),
                _ => None,
            };
        }
        // Subs/<video name>/<language>.srt
        let subs = folder.parent()?;
        if is_subs_folder(&subs.file_name()?) {
            let anchor = subs.parent()?;
            return self
                .in_folder(&anchor)
                .iter()
                .find(|(_, video_stem)| video_stem.eq_ignore_ascii_case(&folder_name))
                .map(|(video, _)| (video.as_str(), stem));
        }
        None
    }
}

/// Every sidecar among `subtitles` with the video it belongs to. Subtitles no video claims are
/// left out.
pub fn assign<S: AsRef<str>>(index: &VideoIndex, subtitles: &[S]) -> Vec<(String, SidecarFile)> {
    let present: HashSet<String> = subtitles
        .iter()
        .map(|path| path.as_ref().to_lowercase())
        .collect();
    let mut found = Vec::new();
    for raw in subtitles {
        let raw = raw.as_ref();
        let path = StoragePath::new(raw);
        let Some(name) = path.file_name() else {
            continue;
        };
        let (stem, extension) = split_name(&name);
        if !is_sidecar_extension(&extension) {
            continue;
        }
        let sibling = |other: &str| {
            let lower = raw.to_lowercase();
            let base = &lower[..lower.len() - extension.len()];
            present.contains(&format!("{base}{other}"))
        };
        let (codec, bitmap) = match extension.as_str() {
            "srt" => ("subrip", false),
            "vtt" => ("webvtt", false),
            "ass" => ("ass", false),
            "ssa" => ("ssa", false),
            // The pair is listed once, by its index file.
            "sub" if sibling("idx") => continue,
            "sub" => ("microdvd", false),
            "idx" if sibling("sub") => ("dvd_subtitle", true),
            _ => continue,
        };
        let Some((video, rest)) = index.place(&path, stem) else {
            continue;
        };
        found.push((
            video.to_owned(),
            SidecarFile {
                path: raw.to_owned(),
                codec,
                bitmap,
                facts: parse_name_tokens(rest),
            },
        ));
    }
    found.sort_by(|a, b| (&a.0, &a.1.path).cmp(&(&b.0, &b.1.path)));
    found
}

/// The name a sidecar takes beside a video: `<video stem>[.<lang>][.forced][.sdh].<ext>`.
/// Imports and organizing rename sidecars with this, so a moved video keeps its subtitles.
pub fn sidecar_name(
    video_stem: &str,
    facts: &NameFacts,
    extension: &str,
    extra: Option<&str>,
) -> String {
    let mut name = video_stem.to_owned();
    if let Some(code) = facts.language {
        name.push('.');
        name.push_str(language::iso639_1(code).unwrap_or(code));
    }
    if let Some(extra) = extra.filter(|extra| !extra.is_empty()) {
        name.push('.');
        name.push_str(extra);
    }
    if facts.forced {
        name.push_str(".forced");
    }
    if facts.hearing_impaired {
        name.push_str(".sdh");
    }
    name.push('.');
    name.push_str(&extension.to_ascii_lowercase());
    name
}

/// The name a sidecar takes when its video is renamed from `old_stem` to `new_stem`: what
/// followed the old name is kept (`Film.en.forced.srt` → `Film (2020).en.forced.srt`); a
/// sidecar not named after its video (`Subs/2_English.srt`) is named from what it says.
pub fn carried_name(old_stem: &str, new_stem: &str, name: &str, facts: &NameFacts) -> String {
    if let Some(rest) = after_video_name(name, old_stem)
        && rest.contains('.')
    {
        let separator = &name[old_stem.len()..old_stem.len() + 1];
        return format!("{new_stem}{separator}{rest}");
    }
    let (_, extension) = split_name(name);
    sidecar_name(new_stem, facts, &extension, facts.title.as_deref())
}

/// The names a video's sidecars take beside it once it is renamed from `old_stem` to
/// `new_stem`, in the order given: [`carried_name`], with `.2`, `.3` for a name already taken
/// by an earlier one (`Subs/English.srt` and `Subs/2_English.srt` are both English). The same
/// input always gives the same names, so importing a torrent again finds them in place and adds
/// nothing.
pub fn carry_names(old_stem: &str, new_stem: &str, files: &[SidecarFile]) -> Vec<String> {
    let mut taken = HashSet::new();
    files
        .iter()
        .map(|file| {
            let name = StoragePath::new(&file.path).file_name().unwrap_or_default();
            let base = carried_name(old_stem, new_stem, &name, &file.facts);
            let mut candidate = base.clone();
            let mut n = 2;
            while !taken.insert(candidate.to_lowercase()) {
                candidate = numbered(&base, n);
                n += 1;
            }
            candidate
        })
        .collect()
}

/// `Film.en.srt` as `Film.en.2.srt`, for a second file that would take the same name.
pub fn numbered(name: &str, n: usize) -> String {
    match name.rsplit_once('.') {
        Some((stem, extension)) => format!("{stem}.{n}.{extension}"),
        None => format!("{name}.{n}"),
    }
}

/// A file name's stem, without its last extension.
pub fn file_stem(name: &str) -> &str {
    split_name(name).0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(rest: &str) -> NameFacts {
        parse_name_tokens(rest)
    }

    #[test]
    fn names_give_language_and_flags() {
        assert_eq!(facts("en").language, Some("eng"));
        let forced = facts("en.forced");
        assert_eq!((forced.language, forced.forced), (Some("eng"), true));
        let sdh = facts("eng.sdh");
        assert_eq!((sdh.language, sdh.hearing_impaired), (Some("eng"), true));
        let hi = facts("English.HI");
        assert_eq!((hi.language, hi.hearing_impaired), (Some("eng"), true));
        let numbered = facts("2_English");
        assert_eq!(
            (numbered.language, numbered.title.clone()),
            (Some("eng"), None)
        );
        assert_eq!(facts("pt-BR").language, Some("por"));
        assert_eq!(facts("English-SDH").hearing_impaired, true);
        let commentary = facts("en.Director Commentary");
        assert_eq!(commentary.title.as_deref(), Some("Director Commentary"));
        assert_eq!(facts("it").language, Some("ita"));
        assert_eq!(facts("").language, None);
        let default = facts("de.default.forced");
        assert!(default.default && default.forced);
        assert_eq!(default.language, Some("ger"));
    }

    #[test]
    fn local_sidecars_beside_and_in_subs_folders() {
        let videos = [
            "/media/Movies/Film (2020)/Film (2020).mkv",
            "/media/Movies/Film (2020)/Film (2020) - Extended.mkv",
            "/media/TV/Show/Season 01/Show.S01E01.mkv",
            "/media/TV/Show/Season 01/Show.S01E02.mkv",
        ];
        let subtitles = [
            "/media/Movies/Film (2020)/Film (2020).en.srt",
            "/media/Movies/Film (2020)/Film (2020).en.forced.srt",
            "/media/Movies/Film (2020)/Film (2020) - Extended.fr.ass",
            "/media/Movies/Film (2020)/Film (2020).idx",
            "/media/Movies/Film (2020)/Film (2020).sub",
            "/media/Movies/Film (2020)/Other.srt",
            "/media/TV/Show/Season 01/Show.S01E01.eng.sdh.srt",
            "/media/TV/Show/Season 01/Subs/Show.S01E02/2_English.srt",
            "/media/TV/Show/Season 01/Subs/Show.S01E02/3_French.srt",
            "/media/TV/Show/Season 01/Subs/Lonely.srt",
        ];
        let index = VideoIndex::new(videos);
        let found = assign(&index, &subtitles);
        let describe = found
            .iter()
            .map(|(video, sidecar)| {
                (
                    video.rsplit('/').next().unwrap().to_owned(),
                    sidecar.path.rsplit('/').next().unwrap().to_owned(),
                    sidecar.codec,
                    sidecar.facts.language,
                    sidecar.facts.forced,
                    sidecar.facts.hearing_impaired,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            describe,
            vec![
                (
                    "Film (2020) - Extended.mkv".into(),
                    "Film (2020) - Extended.fr.ass".into(),
                    "ass",
                    Some("fre"),
                    false,
                    false
                ),
                (
                    "Film (2020).mkv".into(),
                    "Film (2020).en.forced.srt".into(),
                    "subrip",
                    Some("eng"),
                    true,
                    false
                ),
                (
                    "Film (2020).mkv".into(),
                    "Film (2020).en.srt".into(),
                    "subrip",
                    Some("eng"),
                    false,
                    false
                ),
                (
                    "Film (2020).mkv".into(),
                    "Film (2020).idx".into(),
                    "dvd_subtitle",
                    None,
                    false,
                    false
                ),
                (
                    "Show.S01E01.mkv".into(),
                    "Show.S01E01.eng.sdh.srt".into(),
                    "subrip",
                    Some("eng"),
                    false,
                    true
                ),
                (
                    "Show.S01E02.mkv".into(),
                    "2_English.srt".into(),
                    "subrip",
                    Some("eng"),
                    false,
                    false
                ),
                (
                    "Show.S01E02.mkv".into(),
                    "3_French.srt".into(),
                    "subrip",
                    Some("fre"),
                    false,
                    false
                ),
            ]
        );
        assert!(
            found
                .iter()
                .find(|(_, s)| s.path.ends_with(".idx"))
                .unwrap()
                .1
                .bitmap
        );
    }

    #[test]
    fn subs_folder_of_a_single_movie_needs_no_names() {
        let index = VideoIndex::new(["/m/Film/Film.2020.1080p.mkv"]);
        let found = assign(
            &index,
            &[
                "/m/Film/Subs/English.srt",
                "/m/Film/Subs/Spanish.sdh.srt",
                "/m/Film/Film.2020.1080p.sub",
            ],
        );
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].1.codec, "microdvd");
        assert_eq!(found[1].1.facts.language, Some("eng"));
        assert_eq!(found[2].1.facts.language, Some("spa"));
        assert!(found[2].1.facts.hearing_impaired);
    }

    #[test]
    fn smb_sidecars_place_the_same_way() {
        let videos = [
            "smb://nas/media/TV/Show/Season 1/Show - S01E03.mkv",
            "smb://nas/media/Movies/Film/Film.mkv",
        ];
        let subtitles = [
            "smb://nas/media/TV/Show/Season 1/Show - S01E03.en.srt",
            "smb://nas/media/TV/Show/Season 1/Show - S01E03.de.forced.vtt",
            "smb://nas/media/Movies/Film/Subs/English.srt",
            "smb://nas/media/Movies/Film/film.EN.SRT",
        ];
        let index = VideoIndex::new(videos);
        let found = assign(&index, &subtitles);
        assert_eq!(found.len(), 4, "{found:?}");
        let show = found
            .iter()
            .filter(|(video, _)| video.contains("S01E03"))
            .collect::<Vec<_>>();
        assert_eq!(show.len(), 2);
        assert_eq!(show[0].1.facts.language, Some("ger"));
        assert!(show[0].1.facts.forced);
        assert_eq!(show[0].1.codec, "webvtt");
        // Case-insensitive: `film.EN.SRT` belongs to `Film.mkv`.
        assert!(
            found
                .iter()
                .any(|(video, s)| video.ends_with("Film.mkv") && s.path.ends_with("film.EN.SRT"))
        );
    }

    #[test]
    fn a_title_that_merely_starts_the_same_is_not_a_match() {
        let index = VideoIndex::new(["/m/Alien.mkv"]);
        assert!(assign(&index, &["/m/Aliens.en.srt"]).is_empty());
        assert_eq!(assign(&index, &["/m/Alien.srt"]).len(), 1);
    }

    #[test]
    fn carried_names_keep_what_followed_the_video() {
        let english = NameFacts {
            language: Some("eng"),
            ..Default::default()
        };
        assert_eq!(
            carried_name("film", "Film (2020)", "film.en.forced.srt", &english),
            "Film (2020).en.forced.srt"
        );
        assert_eq!(
            carried_name("film", "Film (2020)", "film.srt", &NameFacts::default()),
            "Film (2020).srt"
        );
        assert_eq!(
            carried_name("film", "Film (2020)", "2_English.srt", &english),
            "Film (2020).en.srt"
        );
        assert_eq!(
            carried_name(
                "Show.S01E01.720p",
                "Show - S01E01",
                "Show.S01E01.720p.idx",
                &NameFacts::default()
            ),
            "Show - S01E01.idx"
        );
        assert_eq!(numbered("Film.en.srt", 2), "Film.en.2.srt");
    }

    #[test]
    fn torrent_subtitles_take_the_library_names() {
        let index = VideoIndex::new(["/dl/Film.2020.1080p/Film.2020.1080p.mkv"]);
        let found = assign(
            &index,
            &[
                "/dl/Film.2020.1080p/Film.2020.1080p.en.srt",
                "/dl/Film.2020.1080p/Film.2020.1080p.en.forced.srt",
                "/dl/Film.2020.1080p/Film.2020.1080p.idx",
                "/dl/Film.2020.1080p/Film.2020.1080p.sub",
                "/dl/Film.2020.1080p/Film.2020.1080p.fr.ssa",
                "/dl/Film.2020.1080p/Subs/English.srt",
                "/dl/Film.2020.1080p/Subs/2_English.srt",
                "/dl/Film.2020.1080p/Subs/3_Spanish.srt",
                "/dl/Film.2020.1080p/Subs/4_English_SDH.srt",
            ],
        );
        let files = found.into_iter().map(|(_, file)| file).collect::<Vec<_>>();
        let names = carry_names("Film.2020.1080p", "Film (2020)", &files);
        assert_eq!(
            names,
            vec![
                "Film (2020).en.forced.srt",
                "Film (2020).en.srt",
                "Film (2020).fr.ssa",
                "Film (2020).idx",
                "Film (2020).en.2.srt",
                "Film (2020).es.srt",
                "Film (2020).en.sdh.srt",
                "Film (2020).en.3.srt",
            ]
        );
        // Importing again gives the same names, which are then already in place.
        assert_eq!(carry_names("Film.2020.1080p", "Film (2020)", &files), names);

        let pack = VideoIndex::new([
            "/dl/Show.S01/Show.S01E02.mkv",
            "/dl/Show.S01/Show.S01E01.mkv",
        ]);
        let episode = assign(
            &pack,
            &[
                "/dl/Show.S01/Subs/Show.S01E02/2_English.srt",
                "/dl/Show.S01/Subs/Show.S01E02/3_French.srt",
            ],
        );
        let files = episode
            .into_iter()
            .map(|(_, file)| file)
            .collect::<Vec<_>>();
        assert_eq!(
            carry_names("Show.S01E02", "Show - S01E02 - Title", &files),
            vec![
                "Show - S01E02 - Title.en.srt",
                "Show - S01E02 - Title.fr.srt"
            ]
        );
    }

    #[test]
    fn sidecar_names_follow_the_video() {
        let facts = NameFacts {
            language: Some("eng"),
            forced: true,
            hearing_impaired: false,
            default: false,
            title: None,
        };
        assert_eq!(
            sidecar_name("Film (2020)", &facts, "SRT", None),
            "Film (2020).en.forced.srt"
        );
        let plain = NameFacts::default();
        assert_eq!(
            sidecar_name("Show - S01E01", &plain, "ass", Some("2")),
            "Show - S01E01.2.ass"
        );
    }
}
