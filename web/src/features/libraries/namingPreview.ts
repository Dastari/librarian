/** Made-up titles to show what a naming pattern does, by library type. */
const SAMPLES: Record<string, ReadonlyArray<Record<string, string | number>>> = {
  movies: [{ title: "Arrival", year: 2016, quality: "1080p", ext: "mkv" }],
  tv: [{ show: "Severance", season: 1, episode: 2, title: "Half Loop", year: 2022, quality: "1080p", ext: "mkv" }],
  music: [{ artist: "Foo Fighters", album: "Wasting Light", year: 2011, disc: 1, track: 3, title: "Rope", ext: "flac" }],
  audiobooks: [{ author: "Andy Weir", title: "Project Hail Mary", series: "", series_position: "", chapter: 1, chapter_title: "Chapter One", ext: "m4b" }],
  other: [{ name: "Holiday Video", ext: "mp4" }],
};

const samplesFor = (libraryType: string) => SAMPLES[libraryType.toLowerCase()] ?? SAMPLES.other!;

/** A pattern applied to a sample title, with folders split out, the way the organizer would write it. */
export function previewNamingPattern(pattern: string, libraryType: string): string {
  const sample = samplesFor(libraryType)[0]!;
  return pattern
    .replace(/\{([a-z_]+)(?::0(\d))?\}/g, (whole, token: string, width?: string) => {
      const value = sample[token];
      if (value === undefined) return token === "ext" ? "" : whole;
      return width ? String(value).padStart(Number(width), "0") : String(value);
    })
    .split("/")
    .map((part) => part.replace(/\s+/g, " ").replace(/^[\s-]+|[\s-]+$/g, ""))
    .filter(Boolean)
    .join(" / ");
}
