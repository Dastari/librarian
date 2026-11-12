//! Language names as subtitle files, ffprobe and OpenSubtitles write them.
//!
//! Rows store the ISO 639-2/B code ffprobe reports for embedded streams (`eng`, `fre`, `ger`), so
//! a sidecar's `Movie.en.srt`, `Movie.English.srt` and an embedded `eng` stream all compare equal.
//! OpenSubtitles speaks ISO 639-1 (`en`, `pt-BR`), which [`opensubtitles_code`] gives.

/// (639-2/B, 639-2/T, 639-1, English name, other names a file might use)
const LANGUAGES: &[(&str, &str, &str, &str, &[&str])] = &[
    (
        "eng",
        "eng",
        "en",
        "English",
        &["anglais", "inglés", "ingles"],
    ),
    (
        "fre",
        "fra",
        "fr",
        "French",
        &["français", "francais", "vf", "vff"],
    ),
    ("ger", "deu", "de", "German", &["deutsch"]),
    (
        "spa",
        "spa",
        "es",
        "Spanish",
        &["español", "espanol", "castellano", "latino"],
    ),
    ("ita", "ita", "it", "Italian", &["italiano"]),
    (
        "por",
        "por",
        "pt",
        "Portuguese",
        &[
            "português",
            "portugues",
            "brazilian",
            "pob",
            "ptbr",
            "pt-br",
        ],
    ),
    ("dut", "nld", "nl", "Dutch", &["nederlands", "flemish"]),
    ("swe", "swe", "sv", "Swedish", &["svenska"]),
    (
        "nor",
        "nor",
        "no",
        "Norwegian",
        &["norsk", "nob", "nno", "nb", "nn"],
    ),
    ("dan", "dan", "da", "Danish", &["dansk"]),
    ("fin", "fin", "fi", "Finnish", &["suomi"]),
    ("ice", "isl", "is", "Icelandic", &["íslenska"]),
    ("pol", "pol", "pl", "Polish", &["polski"]),
    ("cze", "ces", "cs", "Czech", &["čeština", "cestina"]),
    ("slo", "slk", "sk", "Slovak", &["slovenčina"]),
    ("slv", "slv", "sl", "Slovenian", &["slovene", "slovenščina"]),
    ("hun", "hun", "hu", "Hungarian", &["magyar"]),
    ("rum", "ron", "ro", "Romanian", &["română", "romana"]),
    ("bul", "bul", "bg", "Bulgarian", &[]),
    ("hrv", "hrv", "hr", "Croatian", &["hrvatski"]),
    ("srp", "srp", "sr", "Serbian", &["srpski"]),
    ("bos", "bos", "bs", "Bosnian", &[]),
    ("mac", "mkd", "mk", "Macedonian", &[]),
    ("gre", "ell", "el", "Greek", &["ελληνικά"]),
    ("tur", "tur", "tr", "Turkish", &["türkçe", "turkce"]),
    ("rus", "rus", "ru", "Russian", &["русский"]),
    ("ukr", "ukr", "uk", "Ukrainian", &[]),
    ("est", "est", "et", "Estonian", &[]),
    ("lav", "lav", "lv", "Latvian", &[]),
    ("lit", "lit", "lt", "Lithuanian", &[]),
    ("ara", "ara", "ar", "Arabic", &[]),
    ("heb", "heb", "he", "Hebrew", &["iw"]),
    ("per", "fas", "fa", "Persian", &["farsi"]),
    ("hin", "hin", "hi", "Hindi", &[]),
    ("ben", "ben", "bn", "Bengali", &[]),
    ("tam", "tam", "ta", "Tamil", &[]),
    ("tel", "tel", "te", "Telugu", &[]),
    ("urd", "urd", "ur", "Urdu", &[]),
    ("tha", "tha", "th", "Thai", &[]),
    ("vie", "vie", "vi", "Vietnamese", &[]),
    ("ind", "ind", "id", "Indonesian", &["bahasa"]),
    ("may", "msa", "ms", "Malay", &[]),
    ("fil", "fil", "tl", "Filipino", &["tagalog", "tgl"]),
    (
        "chi",
        "zho",
        "zh",
        "Chinese",
        &[
            "chs",
            "cht",
            "zhs",
            "zht",
            "mandarin",
            "cantonese",
            "zh-cn",
            "zh-tw",
            "zh-hans",
            "zh-hant",
            "简体",
            "繁體",
        ],
    ),
    ("jpn", "jpn", "ja", "Japanese", &["jp", "日本語"]),
    ("kor", "kor", "ko", "Korean", &["kr", "한국어"]),
    ("cat", "cat", "ca", "Catalan", &["català"]),
    ("baq", "eus", "eu", "Basque", &["euskara"]),
    ("glg", "glg", "gl", "Galician", &[]),
    ("wel", "cym", "cy", "Welsh", &[]),
    ("gle", "gle", "ga", "Irish", &[]),
    ("alb", "sqi", "sq", "Albanian", &[]),
    ("arm", "hye", "hy", "Armenian", &[]),
    ("geo", "kat", "ka", "Georgian", &[]),
    ("afr", "afr", "af", "Afrikaans", &[]),
    ("swa", "swa", "sw", "Swahili", &[]),
    ("lat", "lat", "la", "Latin", &[]),
    ("epo", "epo", "eo", "Esperanto", &[]),
];

/// Codes that are also everyday words in file names and must not be read as a language unless
/// nothing else in the name is: `Movie.it.srt` is Italian, but `Movie.Is.Here` is a title.
const AMBIGUOUS_TWO_LETTER: &[&str] = &[
    "is", "it", "no", "be", "to", "an", "as", "at", "by", "he", "hi", "me", "my", "or", "so", "us",
    "we",
];

fn fold(token: &str) -> String {
    token.trim().to_lowercase().replace('_', "-")
}

/// The ISO 639-2/B code for a code or name, or `None` when the token is not a language.
pub fn normalize(token: &str) -> Option<&'static str> {
    let folded = fold(token);
    if folded.is_empty() {
        return None;
    }
    // `pt-BR`, `en-US`, `zh-Hans`: the base language, unless the whole tag is a known alias.
    for (b, t, one, name, others) in LANGUAGES {
        if folded == *b
            || folded == *t
            || folded == *one
            || folded == name.to_lowercase()
            || others.iter().any(|other| folded == *other)
        {
            return Some(b);
        }
    }
    if let Some((base, _region)) = folded.split_once('-')
        && base.len() <= 3
    {
        return normalize(base);
    }
    None
}

/// Like [`normalize`], but only for tokens a file name uses for a language when it means one:
/// two-letter codes that are common words are refused.
pub fn from_file_token(token: &str) -> Option<&'static str> {
    let folded = fold(token);
    if folded.len() == 2 && AMBIGUOUS_TWO_LETTER.contains(&folded.as_str()) {
        return None;
    }
    normalize(&folded)
}

/// The English name of a stored code (`eng` → `English`), or the code as it is.
pub fn display_name(code: &str) -> String {
    match normalize(code) {
        Some(b) => LANGUAGES
            .iter()
            .find(|entry| entry.0 == b)
            .map(|entry| entry.3.to_owned())
            .unwrap_or_else(|| code.to_owned()),
        None => code.to_owned(),
    }
}

/// The ISO 639-1 code (`en`), which OpenSubtitles and LRCLIB use.
pub fn iso639_1(code: &str) -> Option<&'static str> {
    let b = normalize(code)?;
    LANGUAGES
        .iter()
        .find(|entry| entry.0 == b)
        .map(|entry| entry.2)
}

/// The language codes OpenSubtitles takes for a stored code: ISO 639-1, with Portuguese and
/// Chinese asked for in both of their written forms.
pub fn opensubtitles_codes(code: &str) -> Vec<&'static str> {
    match normalize(code) {
        Some("por") => vec!["pt-PT", "pt-BR"],
        Some("chi") => vec!["zh-CN", "zh-TW"],
        Some(b) => LANGUAGES
            .iter()
            .find(|entry| entry.0 == b)
            .map(|entry| vec![entry.2])
            .unwrap_or_default(),
        None => Vec::new(),
    }
}

/// Parse a preference list (`"en, fr"`, `["eng","fre"]`) into stored codes, in order, once each.
pub fn parse_preference_list(raw: &str) -> Vec<&'static str> {
    let trimmed = raw.trim();
    let items: Vec<String> = if trimmed.starts_with('[') {
        serde_json::from_str::<Vec<String>>(trimmed).unwrap_or_default()
    } else {
        trimmed
            .trim_matches('"')
            .split([',', ';', ' '])
            .map(str::to_owned)
            .collect()
    };
    let mut codes = Vec::new();
    for item in items {
        if let Some(code) = normalize(&item)
            && !codes.contains(&code)
        {
            codes.push(code);
        }
    }
    codes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_names_agree() {
        assert_eq!(normalize("en"), Some("eng"));
        assert_eq!(normalize("ENG"), Some("eng"));
        assert_eq!(normalize("English"), Some("eng"));
        assert_eq!(normalize("fra"), Some("fre"));
        assert_eq!(normalize("Français"), Some("fre"));
        assert_eq!(normalize("pt-BR"), Some("por"));
        assert_eq!(normalize("pt_br"), Some("por"));
        assert_eq!(normalize("en-US"), Some("eng"));
        assert_eq!(normalize("zh-Hant"), Some("chi"));
        assert_eq!(normalize("forced"), None);
        assert_eq!(normalize("sdh"), None);
    }

    #[test]
    fn ambiguous_words_are_not_languages_in_names() {
        assert_eq!(from_file_token("it"), None);
        assert_eq!(from_file_token("no"), None);
        assert_eq!(from_file_token("ita"), Some("ita"));
        assert_eq!(from_file_token("nl"), Some("dut"));
    }

    #[test]
    fn provider_codes_and_preferences() {
        assert_eq!(opensubtitles_codes("eng"), vec!["en"]);
        assert_eq!(opensubtitles_codes("por"), vec!["pt-PT", "pt-BR"]);
        assert_eq!(iso639_1("ger"), Some("de"));
        assert_eq!(display_name("dut"), "Dutch");
        assert_eq!(parse_preference_list("en, fr , en,xx"), vec!["eng", "fre"]);
        assert_eq!(
            parse_preference_list("[\"spa\",\"English\"]"),
            vec!["spa", "eng"]
        );
        assert_eq!(parse_preference_list("\"de\""), vec!["ger"]);
    }
}
