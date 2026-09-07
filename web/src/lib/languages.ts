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
