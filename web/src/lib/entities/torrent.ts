import type { EntityRef } from "glass-ux/ai";
import type { BadgeTone, StatusKind } from "glass-ux/badge";

import { entityPath } from "./kinds";

/** The linked records a torrent row carries (see the Downloads document). */
export interface TorrentLinks {
  season?: number | null;
  library?: { id: string; name: string } | null;
  movie?: { id: string; title: string; year?: number | null } | null;
  show?: { id: string; name: string } | null;
  episode?: { id: string; showId: string; season: number; episode: number; title?: string | null } | null;
  album?: { id: string; name: string } | null;
  audiobook?: { id: string; title: string } | null;
}

const STATE: Record<string, { label: string; status: StatusKind }> = {
  downloading: { label: "Downloading", status: "installing" },
  checking: { label: "Checking", status: "installing" },
  initializing: { label: "Starting", status: "connecting" },
  queued: { label: "Queued", status: "offline" },
  seeding: { label: "Seeding", status: "online" },
  completed: { label: "Completed", status: "online" },
  paused: { label: "Paused", status: "maintenance" },
  error: { label: "Error", status: "error" },
};

/** The client state as the glyph's status dot and its name. */
export function torrentStatus(state: string | null | undefined): { label: string; status: StatusKind } {
  return STATE[(state ?? "").toLowerCase()] ?? { label: state || "Unknown", status: "offline" };
}

/** Filter presets over the stored state (the sync loop keeps it current). */
export const TORRENT_STATE_PRESETS = {
  downloading: ["downloading", "checking", "initializing", "queued"],
  seeding: ["seeding"],
  paused: ["paused"],
  error: ["error"],
} as const;

export const TORRENT_STATES = Object.keys(STATE);

const IMPORT: Record<string, { label: string; tone: BadgeTone }> = {
  completed: { label: "Imported", tone: "success" },
  failed: { label: "Import failed", tone: "danger" },
  unmatched: { label: "No match", tone: "warning" },
};

/** Post-processing outcome. Null means not attempted yet (still downloading, or waiting). */
export function torrentImport(status: string | null | undefined): { label: string; tone: BadgeTone } {
  return status ? IMPORT[status] ?? { label: status, tone: "neutral" } : { label: "Waiting", tone: "neutral" };
}

export const TORRENT_IMPORT_STATES = Object.keys(IMPORT);

/**
 * What the torrent was grabbed for, as an entity reference for a chip: the episode (or the
 * season for a pack), the movie, album or audiobook. Null when nothing is linked yet.
 */
export function torrentTarget(torrent: TorrentLinks): (EntityRef & { href?: string }) | null {
  if (torrent.episode) {
    const code = `S${String(torrent.episode.season).padStart(2, "0")}E${String(torrent.episode.episode).padStart(2, "0")}`;
    const label = torrent.show ? `${torrent.show.name} · ${code}` : code;
    return { kind: "episode", id: torrent.episode.id, label, meta: torrent.episode.title ?? undefined, href: entityPath("episode", torrent.episode.id, torrent.episode.showId) };
  }
  if (torrent.show) {
    const label = torrent.season != null ? `${torrent.show.name} · Season ${torrent.season}` : torrent.show.name;
    return { kind: "show", id: torrent.show.id, label, href: entityPath("show", torrent.show.id) };
  }
  if (torrent.movie) return { kind: "movie", id: torrent.movie.id, label: torrent.movie.title, meta: torrent.movie.year ? String(torrent.movie.year) : undefined, href: entityPath("movie", torrent.movie.id) };
  if (torrent.album) return { kind: "album", id: torrent.album.id, label: torrent.album.name, href: entityPath("album", torrent.album.id) };
  if (torrent.audiobook) return { kind: "audiobook", id: torrent.audiobook.id, label: torrent.audiobook.title, href: entityPath("audiobook", torrent.audiobook.id) };
  return null;
}

/** Share ratio against the tracker's minimum (1.0 when it sets none). */
export type RatioLevel = "short" | "close" | "met";
export function ratioLevel(ratio: number, minimum: number | null | undefined): RatioLevel {
  const target = minimum && minimum > 0 ? minimum : 1;
  return ratio >= target ? "met" : ratio >= target * 0.5 ? "close" : "short";
}
