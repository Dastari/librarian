/**
 * REST endpoints that sit beside GraphQL: artwork, media streaming and health.
 * The browser always talks to the same origin; the dev server proxies to the backend.
 */

export type ArtworkEntity = "movie" | "show" | "episode" | "album" | "artist" | "audiobook" | "collection" | "person";
export type ArtworkKind = "poster" | "backdrop" | "banner" | "cover" | "profile" | "thumbnail";

export function artworkUrl(entity: ArtworkEntity, id: string | number, kind: ArtworkKind): string {
  return `/api/artwork/${entity}/${encodeURIComponent(String(id))}/${kind}`;
}

/**
 * Metadata providers return absolute URLs while the artwork cache stores local `/api/artwork`
 * paths. Both are usable as-is; this only normalises empty values.
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

/** Asks the backend how this file should play; falls back to direct streaming on failure. */
export async function resolvePlaybackSource(mediaFileId: string, signal?: AbortSignal): Promise<PlaybackSource> {
  const fallback: PlaybackSource = { url: mediaStreamUrl(mediaFileId), isHls: false, duration: undefined, info: null };
  try {
    const response = await fetch(mediaInfoUrl(mediaFileId), { credentials: "include", signal });
    if (!response.ok) return fallback;
    const info = (await response.json()) as MediaInfo;
    const isHls = info.needs_hls === true && typeof info.playback_url === "string";
    return {
      url: isHls ? info.playback_url : fallback.url,
      isHls,
      duration: typeof info.duration === "number" && info.duration > 0 ? info.duration : undefined,
      info,
    };
  } catch {
    return fallback;
  }
}
