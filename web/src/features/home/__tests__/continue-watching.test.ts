import { describe, expect, it } from "vitest";
import type { ResumeShelfQuery } from "@/graphql/generated/graphql";
import { resumeItems } from "../useContinueWatching";

type Row = ResumeShelfQuery["resumeShelf"]["items"][number];
type File = NonNullable<Row["mediaFile"]>;
export const checkpoint = (file: Partial<File> = {}, overrides: Partial<Row> = {}): Row => ({
  id: "checkpoint", userId: "me", mediaFileId: "file", currentPosition: 60, duration: null,
  progressPercent: 50, isWatched: false, watchedAt: null, createdAt: "2026-10-24", updatedAt: "2026-10-24",
  mediaFile: { id: "file", duration: 120, movie: null, episode: null, track: null, chapter: null, ...file },
  ...overrides,
});

describe("authoritative resume shelf composition", () => {
  it("preserves server order and movie artwork/position without a bounded client regroup", () => {
    const rows = [
      checkpoint({ movie: { id: "movie-b", title: "Second film", year: 2026 } }, { id: "b", updatedAt: "1" }),
      checkpoint({ movie: { id: "movie-a", title: "First film", year: null } }, { id: "a", updatedAt: "9999999" }),
    ];
    const result = resumeItems(rows);
    expect(result.map((item) => item.key)).toEqual(["b", "a"]);
    expect(result[0]).toMatchObject({ title: "Second film", subtitle: "2026", href: "/movies/movie-b", position: 60, duration: 120, progress: 0.5 });
    expect(result[0]?.image).toContain("movie-b");
  });
  it("composes show, album and book destinations from generated nested entities", () => {
    const items = resumeItems([
      checkpoint({ episode: { id: "ep", showId: "show", season: 2, episode: 5, title: "Episode title", show: { id: "show", name: "Show", posterUrl: null, backdropUrl: "show-art" } } }),
      checkpoint({ track: { id: "track", albumId: "album", title: "Song", artistName: "Artist", album: { id: "album", name: "Recording", coverUrl: "album-art" } } }),
      checkpoint({ chapter: { id: "chapter", audiobookId: "book", title: "Part one", chapterNumber: 1, audiobook: { id: "book", title: "Book", coverUrl: "book-art", authorName: "Author" } } }),
    ]);
    expect(items.map((item) => item.href)).toEqual(["/shows/show", "/albums/album", "/audiobooks/book"]);
    expect(items.map((item) => item.entity)).toEqual([
      { kind: "episode", id: "ep" }, { kind: "track", id: "track" }, { kind: "chapter", id: "chapter" },
    ]);
    expect(items[2]?.audiobookId).toBe("book");
    // Artwork always comes from the local cache, which downloads the provider image once.
    expect(items.map((item) => item.image)).toEqual(["/api/artwork/show/show/backdrop", "/api/artwork/album/album/cover", "/api/artwork/audiobook/book/cover"]);
    expect(items[0]?.subtitle).toContain("S02 · E05");
    expect(items[1]?.subtitle).toBe("Recording");
    expect(items[2]?.title).toBe("Book");
  });
  it("does not fabricate a display title when the authorized file/target vanished", () => {
    expect(resumeItems([checkpoint({}, { mediaFile: null })])).toEqual([]);
    expect(resumeItems([checkpoint()])).toEqual([]);
  });
});
