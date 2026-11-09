import type { NowPlayingQuery } from "@/graphql/generated/graphql";
import { albumCover, audiobookCover, moviePoster, showPoster } from "@/lib/artwork";
import { parseTimestamp } from "@/lib/format";

export type NowPlayingSession = NowPlayingQuery["playbackSessions"]["edges"][number]["node"];

/**
 * A player reports every 5–15 seconds (paused ones too), so a session silent for this long has
 * gone: a closed laptop or a tab that crashed without saying goodbye.
 */
export const STALE_AFTER_MS = 3 * 60_000;

export function isLive(session: Pick<NowPlayingSession, "endedAt" | "lastUpdatedAt">, now = Date.now()): boolean {
  if (session.endedAt) return false;
  const updated = parseTimestamp(session.lastUpdatedAt)?.getTime();
  return updated !== undefined && now - updated <= STALE_AFTER_MS;
}

export const METHOD_LABELS: Record<string, string> = {
  direct: "Direct play",
  hls: "Direct stream",
  transcode: "Transcoding",
  cast: "Casting",
};

export function methodLabel(method: string | null | undefined): string {
  return (method && METHOD_LABELS[method]) || "Playing";
}

export type PlayState = { id: string; label: string; tone: "success" | "neutral" | "warning" | "accent" };

export function playState(session: Pick<NowPlayingSession, "state" | "isPlaying" | "stopRequestedAt">): PlayState {
  if (session.stopRequestedAt) return { id: "stopping", label: "Stopping", tone: "warning" };
  const state = session.state ?? (session.isPlaying ? "playing" : "paused");
  if (state === "playing") return { id: "playing", label: "Playing", tone: "success" };
  if (state === "buffering") return { id: "buffering", label: "Buffering", tone: "accent" };
  return { id: "paused", label: "Paused", tone: "neutral" };
}

/** "1080p · 8.2 Mbps" */
export function qualityLabel(session: Pick<NowPlayingSession, "quality" | "bitrate">): string {
  const rate = session.bitrate ? (session.bitrate >= 1_000_000 ? `${(session.bitrate / 1_000_000).toFixed(1)} Mbps` : `${Math.round(session.bitrate / 1000)} kbps`) : "";
  return [session.quality ?? "", rate].filter(Boolean).join(" · ");
}

/** What a session is of: its title, the line under it, its picture and its page. */
export function describeSession(session: NowPlayingSession): { title: string; subtitle: string; artwork?: string; square: boolean; href?: string } {
  if (session.movie) {
    return { title: session.movie.title, subtitle: session.movie.year ? String(session.movie.year) : "Movie", artwork: moviePoster(session.movie.id), square: false, href: `/movies/${session.movie.id}` };
  }
  if (session.episode) {
    const episode = session.episode;
    const show = episode.show;
    return {
      title: episode.title || `Episode ${episode.episode}`,
      subtitle: [show?.name, `S${episode.season} · E${episode.episode}`].filter(Boolean).join(" · "),
      artwork: show ? showPoster(show) : undefined,
      square: false,
      href: show ? `/shows/${show.id}` : undefined,
    };
  }
  if (session.track) {
    const track = session.track;
    return {
      title: track.title,
      subtitle: [track.artistName, track.album?.name].filter(Boolean).join(" · "),
      artwork: track.album ? albumCover(track.album) : undefined,
      square: true,
      href: track.album ? `/albums/${track.album.id}` : undefined,
    };
  }
  if (session.audiobook) {
    return { title: session.audiobook.title, subtitle: session.audiobook.authorName ?? "Audiobook", artwork: audiobookCover(session.audiobook), square: true, href: `/audiobooks/${session.audiobook.id}` };
  }
  return { title: "Something no longer in the library", subtitle: "", square: false };
}

/** The fraction played, 0–1, when the length is known. */
export function sessionProgress(session: Pick<NowPlayingSession, "currentPosition" | "duration">): number | null {
  if (!session.duration || session.duration <= 0) return null;
  return Math.max(0, Math.min(1, session.currentPosition / session.duration));
}
