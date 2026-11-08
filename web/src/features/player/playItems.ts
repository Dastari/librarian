import { albumCover, audiobookCover, moviePoster, movieBackdrop, showBackdrop, showPoster } from "@/lib/artwork";

import type { PlayItem } from "./store";

/** How a piece of media becomes a queue entry, wherever it is played or queued from. */

export function trackPlayItem(row: { id: string; title: string; mediaFileId?: string | null; artistName?: string | null; albumId: string; album?: { id: string; name: string; coverUrl?: string | null } | null; durationSecs?: number | null }): PlayItem | null {
  if (!row.mediaFileId) return null;
  return {
    mediaFileId: row.mediaFileId,
    title: row.title,
    subtitle: [row.artistName, row.album?.name].filter(Boolean).join(" · ") || undefined,
    artwork: row.album ? albumCover(row.album) : undefined,
    entity: { kind: "track", id: row.id },
    albumId: row.albumId,
    href: `/albums/${row.albumId}`,
    duration: row.durationSecs ?? undefined,
  };
}

export const episodeCode = (episode: { season: number; episode: number }) => `S${String(episode.season).padStart(2, "0")} · E${String(episode.episode).padStart(2, "0")}`;

export function episodePlayItem(episode: { id: string; season: number; episode: number; title?: string | null; mediaFileId?: string | null }, show: { id: string; name: string }, startPosition?: number): PlayItem | null {
  if (!episode.mediaFileId) return null;
  return {
    mediaFileId: episode.mediaFileId,
    title: show.name,
    subtitle: [episodeCode(episode), episode.title].filter(Boolean).join("  "),
    artwork: showPoster(show),
    backdrop: showBackdrop(show),
    entity: { kind: "episode", id: episode.id },
    showId: show.id,
    href: `/shows/${show.id}`,
    startPosition,
  };
}

export function moviePlayItem(movie: { id: string; title: string; year?: number | null; mediaFileId?: string | null }, startPosition?: number): PlayItem | null {
  if (!movie.mediaFileId) return null;
  return {
    mediaFileId: movie.mediaFileId,
    title: movie.title,
    subtitle: movie.year ? String(movie.year) : undefined,
    artwork: moviePoster(movie.id),
    backdrop: movieBackdrop(movie.id),
    entity: { kind: "movie", id: movie.id },
    href: `/movies/${movie.id}`,
    startPosition,
  };
}

export interface BookChapter {
  id: string;
  chapterNumber: number;
  title?: string | null;
  startTimeSecs: number;
  mediaFileId?: string | null;
  durationSecs?: number | null;
  fileOffsetSecs?: number | null;
  mediaFile?: { duration?: number | null } | null;
}
/** A file a book was cut into, when its chapters are places in files cut without regard to them. */
export interface BookPart {
  id: string;
  chapterId?: string | null;
  duration?: number | null;
}
interface Book {
  id: string;
  title: string;
  authorName?: string | null;
  fileShape?: string | null;
}

const chapterName = (chapter: BookChapter) => chapter.title ?? `Chapter ${chapter.chapterNumber}`;

/** One file holding the whole book: it plays once, and a chapter is a place in it. */
export const isWholeFileBook = (chapters: readonly BookChapter[]) => chapters.length > 1 && Boolean(chapters[0]?.mediaFileId) && chapters.every((chapter) => chapter.mediaFileId === chapters[0]!.mediaFileId);
/** Files cut without regard to chapters: they play in order, and a chapter is a place in one of them. */
export const isPartsBook = (book: Pick<Book, "fileShape">, chapters: readonly BookChapter[]) => book.fileShape === "parts" && !isWholeFileBook(chapters);

/** The entries a book plays as: its parts, its one file, or a file per chapter. */
export function audiobookPlayItems(book: Book, chapters: readonly BookChapter[], parts: readonly BookPart[] = []): PlayItem[] {
  const wholeFile = isWholeFileBook(chapters);
  const inParts = isPartsBook(book, chapters);
  const partIndex = (id: string | null | undefined) => parts.findIndex((part) => part.id === id);
  const base = { artwork: audiobookCover(book), audiobookId: book.id, href: `/audiobooks/${book.id}` };
  if (inParts) {
    return parts.map((part, index) => {
      const here = chapters.filter((chapter) => chapter.mediaFileId === part.id);
      const before = chapters.filter((chapter) => { const holder = partIndex(chapter.mediaFileId); return holder >= 0 && holder < index; }).at(-1);
      return {
        ...base,
        mediaFileId: part.id,
        title: book.title,
        subtitle: `Part ${index + 1} of ${parts.length}`,
        entity: { kind: "chapter" as const, id: part.chapterId ?? chapters[0]!.id },
        duration: part.duration ?? undefined,
        chapters: here.map((chapter) => ({ start: chapter.fileOffsetSecs ?? 0, title: chapterName(chapter) })),
        runningChapter: before ? chapterName(before) : undefined,
      };
    });
  }
  if (wholeFile) {
    return [{
      ...base,
      mediaFileId: chapters[0]!.mediaFileId!,
      title: book.title,
      subtitle: book.authorName ?? undefined,
      entity: { kind: "chapter", id: chapters[0]!.id },
      duration: chapters[0]!.mediaFile?.duration ?? undefined,
      chapters: chapters.map((chapter) => ({ start: chapter.startTimeSecs, title: chapterName(chapter) })),
    }];
  }
  return chapters
    .filter((chapter) => chapter.mediaFileId)
    .map((chapter) => ({
      ...base,
      mediaFileId: chapter.mediaFileId!,
      title: book.title,
      subtitle: chapterName(chapter),
      entity: { kind: "chapter" as const, id: chapter.id },
      duration: chapter.durationSecs ?? undefined,
    }));
}
