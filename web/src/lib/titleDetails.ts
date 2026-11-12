/**
 * The structured fields of a `TitleDetail` are JSON text in shapes the backend documents
 * (docs/design.md Q130). These read them defensively: a field that is missing, malformed or
 * from a newer shape gives an empty list rather than an error.
 */

export interface ContentRating {
  country: string;
  rating: string;
  kind?: string | null;
}

export interface AlternativeTitle {
  title: string;
  country?: string | null;
  kind?: string | null;
}

export interface VideoLink {
  site: string;
  key: string;
  url?: string | null;
  name: string;
  kind?: string | null;
  official: boolean;
  language?: string | null;
  publishedAt?: string | null;
}

function parse(text: string | null | undefined): unknown {
  if (!text) return null;
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return null;
  }
}

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const text = (value: unknown): string | null => (typeof value === "string" && value.trim() ? value : null);

function list<T>(source: string | null | undefined, read: (item: Record<string, unknown>) => T | null): T[] {
  const value = parse(source);
  if (!Array.isArray(value)) return [];
  return value.flatMap((item) => {
    if (!isRecord(item)) return [];
    const read_ = read(item);
    return read_ ? [read_] : [];
  });
}

export function contentRatings(source: string | null | undefined): ContentRating[] {
  return list(source, (item) => {
    const country = text(item.country);
    const rating = text(item.rating);
    return country && rating ? { country, rating, kind: text(item.kind) } : null;
  });
}

export function alternativeTitles(source: string | null | undefined): AlternativeTitle[] {
  return list(source, (item) => {
    const title = text(item.title);
    return title ? { title, country: text(item.country), kind: text(item.kind) } : null;
  });
}

/** Only links that open somewhere: a video without a known site URL is left out. */
export function videos(source: string | null | undefined): VideoLink[] {
  return list(source, (item) => {
    const site = text(item.site);
    const key = text(item.key);
    const url = text(item.url);
    if (!site || !key || !url || !/^https:\/\//.test(url)) return null;
    return { site, key, url, name: text(item.name) ?? site, kind: text(item.kind), official: item.official === true, language: text(item.language), publishedAt: text(item.publishedAt) };
  });
}

export function externalIds(source: string | null | undefined): Record<string, string> {
  const value = parse(source);
  if (!isRecord(value)) return {};
  return Object.fromEntries(Object.entries(value).flatMap(([site, id]) => (typeof id === "string" && id ? [[site, id]] : typeof id === "number" ? [[site, String(id)]] : [])));
}

/** The rating to show: the viewer's country when listed, then the US, then the first. */
export function preferredRating(ratings: ContentRating[], country = browserCountry()): ContentRating | undefined {
  return ratings.find((rating) => rating.country === country) ?? ratings.find((rating) => rating.country === "US") ?? ratings[0];
}

function browserCountry(): string | undefined {
  if (typeof navigator === "undefined") return undefined;
  const region = navigator.language?.split("-")[1];
  return region?.toUpperCase();
}

/** The first official trailer, else any trailer, else nothing. */
export function mainTrailer(list: VideoLink[]): VideoLink | undefined {
  const trailers = list.filter((video) => video.kind === "Trailer");
  return trailers.find((video) => video.official) ?? trailers[0];
}
