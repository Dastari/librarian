import { API_BASE_URL } from "./baseUrl";

export function getMediaStreamUrl(mediaFileId: string): string {
  return `${API_BASE_URL}/api/media/${mediaFileId}/stream`;
}

export interface MediaPlaybackSource {
  url: string;
  /** Full file duration; an in-progress HLS playlist only describes encoded segments. */
  duration?: number;
}

export async function resolveMediaPlaybackSource(
  mediaFileId: string,
): Promise<MediaPlaybackSource> {
  const fallback = { url: getMediaStreamUrl(mediaFileId) };
  try {
    const response = await fetch(
      `${API_BASE_URL}/api/media/${mediaFileId}/info`,
      { credentials: "include" },
    );
    if (!response.ok) return fallback;
    const info = await response.json();
    return {
      url:
        info?.needs_hls === true && typeof info.playback_url === "string"
          ? `${API_BASE_URL}${info.playback_url}`
          : fallback.url,
      duration:
        typeof info?.duration === "number" &&
        Number.isFinite(info.duration) &&
        info.duration > 0
          ? info.duration
          : undefined,
    };
  } catch {
    return fallback;
  }
}

export async function resolveMediaPlaybackUrl(
  mediaFileId: string,
): Promise<string> {
  return (await resolveMediaPlaybackSource(mediaFileId)).url;
}
