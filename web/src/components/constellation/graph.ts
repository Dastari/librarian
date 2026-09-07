/**
 * The sign-in constellation dataset: well-known films and series joined to their best-known
 * cast, with each one's Wikipedia page image (a poster or a portrait). Generated from Wikidata
 * and Wikipedia by `scripts/generate-film-graph.mjs`; loaded lazily so it only ships to the
 * sign-in screens.
 */
export interface FilmGraph {
  generatedAt: string;
  source: string;
  /** [name, year, kind, imagePath] where kind is 0 for film and 1 for series. */
  titles: [string, number, 0 | 1, string][];
  /** [name, imagePath] */
  people: [string, string][];
  /** [titleIndex, personIndex] */
  edges: [number, number][];
}

export interface Adjacency {
  graph: FilmGraph;
  /** Person indices for each title. */
  cast: number[][];
  /** Title indices for each person. */
  credits: number[][];
}

const UPLOAD_PREFIX = "https://upload.wikimedia.org/wikipedia/";

/**
 * Full thumbnail URL for a stored image, or null when the page had no usable image. Paths are
 * stored as `wiki:hash:width:file` (see the generator); anything else is a verbatim path.
 */
export function imageUrl(path: string): string | null {
  if (!path) return null;
  const parts = path.split(":");
  if (parts.length === 4) {
    const [wiki, hash, width, file] = parts as [string, string, string, string];
    return `${UPLOAD_PREFIX}${wiki}/thumb/${hash}/${file}/${width}px-${file}`;
  }
  return `${UPLOAD_PREFIX}${path}`;
}

let cached: Promise<Adjacency> | null = null;

export function loadFilmGraph(): Promise<Adjacency> {
  cached ??= import("@/assets/film-graph.json").then((module) => {
    const graph = (module.default ?? module) as unknown as FilmGraph;
    const cast: number[][] = graph.titles.map(() => []);
    const credits: number[][] = graph.people.map(() => []);
    for (const [title, person] of graph.edges) {
      cast[title]!.push(person);
      credits[person]!.push(title);
    }
    return { graph, cast, credits };
  });
  return cached;
}
