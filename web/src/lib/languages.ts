/**
 * The languages offered in quality profiles. ISO 639-1 codes, because that is what release
 * names and the backend's parser use. Deliberately short: the thirty languages that actually
 * show up in scene and web releases.
 */

export interface Language {
  code: string;
  name: string;
}

export const LANGUAGES: Language[] = [
  { code: "en", name: "English" },
  { code: "es", name: "Spanish" },
  { code: "fr", name: "French" },
  { code: "de", name: "German" },
  { code: "it", name: "Italian" },
  { code: "pt", name: "Portuguese" },
  { code: "nl", name: "Dutch" },
  { code: "sv", name: "Swedish" },
  { code: "no", name: "Norwegian" },
  { code: "da", name: "Danish" },
  { code: "fi", name: "Finnish" },
  { code: "is", name: "Icelandic" },
  { code: "pl", name: "Polish" },
  { code: "cs", name: "Czech" },
  { code: "sk", name: "Slovak" },
  { code: "hu", name: "Hungarian" },
  { code: "ro", name: "Romanian" },
  { code: "bg", name: "Bulgarian" },
  { code: "el", name: "Greek" },
  { code: "ru", name: "Russian" },
  { code: "uk", name: "Ukrainian" },
  { code: "tr", name: "Turkish" },
  { code: "he", name: "Hebrew" },
  { code: "ar", name: "Arabic" },
  { code: "hi", name: "Hindi" },
  { code: "ja", name: "Japanese" },
  { code: "ko", name: "Korean" },
  { code: "zh", name: "Chinese" },
  { code: "th", name: "Thai" },
  { code: "vi", name: "Vietnamese" },
  { code: "id", name: "Indonesian" },
];

const BY_CODE = new Map(LANGUAGES.map((language) => [language.code, language]));

/** "en" -> "English"; unknown codes are shown uppercased so nothing disappears. */
export function languageName(code: string): string {
  return BY_CODE.get(code.toLowerCase())?.name ?? code.toUpperCase();
}

/**
 * Codes as subtitle rows store them (ISO 639-2/B, as ffprobe reports: `eng`, `fre`, `ger`) as BCP
 * 47 tags and names a person reads. Unknown codes pass through.
 */
const TO_BCP47: Record<string, string> = {
  eng: "en", fre: "fr", fra: "fr", ger: "de", deu: "de", spa: "es", ita: "it", por: "pt", dut: "nl", nld: "nl",
  swe: "sv", nor: "no", nob: "nb", nno: "nn", dan: "da", fin: "fi", ice: "is", isl: "is", pol: "pl", cze: "cs", ces: "cs",
  slo: "sk", slk: "sk", slv: "sl", hun: "hu", rum: "ro", ron: "ro", bul: "bg", hrv: "hr", srp: "sr", bos: "bs",
  mac: "mk", mkd: "mk", gre: "el", ell: "el", tur: "tr", rus: "ru", ukr: "uk", est: "et", lav: "lv", lit: "lt",
  ara: "ar", heb: "he", per: "fa", fas: "fa", hin: "hi", ben: "bn", tam: "ta", tel: "te", urd: "ur", tha: "th",
  vie: "vi", ind: "id", may: "ms", msa: "ms", fil: "fil", tgl: "tl", chi: "zh", zho: "zh", jpn: "ja", kor: "ko",
  cat: "ca", baq: "eu", eus: "eu", glg: "gl", wel: "cy", cym: "cy", gle: "ga", alb: "sq", sqi: "sq", arm: "hy",
  hye: "hy", geo: "ka", kat: "ka", afr: "af", swa: "sw", lat: "la", epo: "eo",
};

/** A stored code as a BCP 47 tag (`eng` → `en`; `pt-BR` stays). */
export function toBcp47(code: string | null | undefined): string | undefined {
  if (!code) return undefined;
  const trimmed = code.trim();
  const lower = trimmed.toLowerCase();
  if (!lower || lower === "und") return undefined;
  if (TO_BCP47[lower]) return TO_BCP47[lower];
  if (/^[a-z]{2,3}(-[a-z0-9]{2,8})*$/i.test(trimmed)) return trimmed;
  return undefined;
}

/** The same language, whatever form each code is in. */
export function sameLanguage(a: string | null | undefined, b: string | null | undefined): boolean {
  const left = toBcp47(a)?.split("-")[0]?.toLowerCase();
  const right = toBcp47(b)?.split("-")[0]?.toLowerCase();
  return Boolean(left) && left === right;
}

let displayNames: Intl.DisplayNames | null | undefined;

/** `eng` → `English`, in the browser's language when it can name it; `undefined` for no code. */
export function displayLanguage(code: string | null | undefined): string | undefined {
  const tag = toBcp47(code);
  if (!tag) return code?.trim() || undefined;
  try {
    displayNames ??= new Intl.DisplayNames(undefined, { type: "language" });
    return displayNames.of(tag) ?? tag;
  } catch {
    displayNames = null;
    return languageName(tag.split("-")[0]!);
  }
}
