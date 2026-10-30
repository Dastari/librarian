import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { useMemo } from "react";

import type { PlayItem } from "@/features/player/store";

import { ResumeShelfDocument, type ResumeShelfQuery } from "@/graphql/generated/graphql";
import { albumCover, audiobookCover, movieBackdrop, showBackdrop } from "@/lib/artwork";
import { useSession } from "@/lib/auth/useSession";
import { isPresent } from "@/lib/utils";

export type ResumeKind = "movie" | "episode" | "track" | "chapter";

export interface ResumeItem {
  key: string;
  kind: ResumeKind;
  mediaFileId: string;
  entity: PlayItem["entity"];
  audiobookId?: string;
  title: string;
  subtitle?: string;
  image: string | undefined;
  progress: number;
  position: number;
  duration: number | undefined;
  /** Route to the owning entity's detail page. */
  href: string;
  updatedAt: string;
}

/** Format authorized generated entity nesting; identity/order/completion are
 * decided by the server's latest checkpoint per logical title, not a local cap. */
export function resumeItems(rows: ResumeShelfQuery["resumeShelf"]["items"]): ResumeItem[] {
  return rows.map((row): ResumeItem | null => {
    const file = row.mediaFile;
    if (!file || !row.mediaFileId) return null;
    const base = {
      key: row.id, mediaFileId: row.mediaFileId,
      progress: row.progressPercent / 100, position: row.currentPosition,
      duration: row.duration ?? file.duration ?? undefined, updatedAt: row.updatedAt,
    };
    if (file.movie) {
      const movie = file.movie;
      return { ...base, kind: "movie", entity: { kind: "movie", id: movie.id }, title: movie.title, subtitle: movie.year ? String(movie.year) : undefined, image: movieBackdrop(movie.id), href: `/movies/${movie.id}` };
    }
    if (file.episode) {
      const episode = file.episode;
      const code = `S${String(episode.season).padStart(2, "0")} · E${String(episode.episode).padStart(2, "0")}`;
      return { ...base, kind: "episode", entity: { kind: "episode", id: episode.id }, title: episode.show?.name ?? episode.title ?? "Episode", subtitle: [code, episode.title].filter(Boolean).join("  "), image: episode.show ? showBackdrop(episode.show) : undefined, href: `/shows/${episode.showId}` };
    }
    if (file.track) {
      const track = file.track;
      return { ...base, kind: "track", entity: { kind: "track", id: track.id }, title: track.title, subtitle: track.album?.name ?? track.artistName ?? undefined, image: track.album ? albumCover(track.album) : undefined, href: `/albums/${track.albumId}` };
    }
    if (file.chapter) {
      const chapter = file.chapter;
      return { ...base, kind: "chapter", entity: { kind: "chapter", id: chapter.id }, audiobookId: chapter.audiobookId, title: chapter.audiobook?.title ?? chapter.title ?? "Audiobook", subtitle: chapter.title ?? `Chapter ${chapter.chapterNumber}`, image: chapter.audiobook ? audiobookCover(chapter.audiobook) : undefined, href: `/audiobooks/${chapter.audiobookId}` };
    }
    return null;
  }).filter(isPresent);
}

export function useContinueWatching(options?: { limit?: number; libraryId?: string }) {
  const { user } = useSession();
  const query = useQuery(ResumeShelfDocument, { variables: { limit: options?.limit ?? 20, libraryId: options?.libraryId }, skip: !user });
  const shelf = query.data?.resumeShelf;
  const items = useMemo(() => resumeItems(shelf?.items ?? []), [shelf]);
  const missing = Boolean(shelf && items.length !== shelf.items.length);
  return {
    items,
    loading: query.loading,
    error: query.error ?? (shelf?.error ? new Error(shelf.error) : missing ? new Error("A resume item changed while loading. Retry to reconcile the shelf.") : undefined),
    complete: Boolean(shelf?.complete && !shelf.error && !query.error && !missing),
    hasMore: shelf?.hasMore ?? null,
    scannedCount: shelf?.scannedCount ?? 0,
    refetch: query.refetch,
  };
}
