/**
 * REST endpoints that sit beside GraphQL: artwork, media streaming and health.
 * The browser always talks to the same origin; the dev server proxies to the backend.
 */

export type ArtworkEntity = "movie" | "show" | "season" | "episode" | "album" | "artist" | "audiobook" | "collection" | "person" | "guide";
export type ArtworkKind = "poster" | "backdrop" | "banner" | "cover" | "profile" | "thumbnail";

export function artworkUrl(entity: ArtworkEntity, id: string | number, kind: ArtworkKind): string {
  return `/api/artwork/${entity}/${encodeURIComponent(String(id))}/${kind}`;
}

/**
 * Ask the artwork cache for a stored copy close to the displayed size. The server snaps the
 * width to a small set of steps and never upscales, so the result is cacheable everywhere.
 * Only local cache URLs are resized; anything else is returned unchanged.
 */
export function sizedArtwork(url: string | undefined, displayWidth: number | undefined): string | undefined {
  if (!url || !displayWidth || !url.startsWith("/api/artwork/") || url.includes("?")) return url;
  const density = typeof window === "undefined" ? 1 : Math.min(window.devicePixelRatio || 1, 2);
  return `${url}?w=${Math.round(displayWidth * density)}`;
}

/**
 * The artwork cache stores local `/api/artwork` paths. Absolute URLs are still accepted for
 * provider search results that are not in the library yet; this only normalises empty values.
 */
export function resolveArtwork(url: string | null | undefined): string | undefined {
  if (!url) return undefined;
  return url;
}

export function mediaStreamUrl(mediaFileId: string): string {
  return `/api/media/${encodeURIComponent(mediaFileId)}/stream`;
}

export function mediaInfoUrl(mediaFileId: string): string {
  return `/api/media/${encodeURIComponent(mediaFileId)}/info`;
}

export interface MediaInfo {
  id: string;
  exists: boolean;
  size_bytes: number;
  content_type: string | null;
  container: string | null;
  video_codec: string | null;
  audio_codec: string | null;
  resolution: string | null;
  width: number | null;
  height: number | null;
  duration: number | null;
  is_hdr: boolean;
  hdr_type: string | null;
  chromecast_compatible: boolean;
  needs_hls: boolean;
  transcode_decision: "none" | "remux" | "transcode";
  playback_url: string;
}

export interface PlaybackSource {
  url: string;
  isHls: boolean;
  duration: number | undefined;
  info: MediaInfo | null;
}

/** Resolve the authorised playback decision before attaching a media element. */
export async function resolvePlaybackSource(mediaFileId: string, signal?: AbortSignal, options?: { audioStreamIndex?: number }): Promise<PlaybackSource> {
  const query = options?.audioStreamIndex === undefined ? "" : `?audioStream=${encodeURIComponent(options.audioStreamIndex)}`;
  const response = await fetch(`${mediaInfoUrl(mediaFileId)}${query}`, { credentials: "include", signal });
  if (!response.ok) {
    if (response.status === 401) throw new Error("Your session ended. Sign in to continue playback.");
    if (response.status === 403 || response.status === 404) throw new Error("This media file is unavailable or you no longer have access to it.");
    throw new Error("The server could not prepare this file. Retry playback.");
  }
  const info = (await response.json()) as MediaInfo;
  if (!info.exists) throw new Error("The media file is missing from its library. Check the library connection and retry.");
  if (typeof info.playback_url !== "string" || !info.playback_url.startsWith(`/api/media/${encodeURIComponent(mediaFileId)}/`)) {
    throw new Error("The server returned an invalid playback source.");
  }
  return {
    url: info.playback_url,
    isHls: info.needs_hls === true,
    duration: typeof info.duration === "number" && info.duration > 0 ? info.duration : undefined,
    info,
  };
}

export function mediaSubtitleUrl(mediaFileId: string, subtitleId: string): string {
  return `/api/media/${encodeURIComponent(mediaFileId)}/subtitles/${encodeURIComponent(subtitleId)}.vtt`;
}
