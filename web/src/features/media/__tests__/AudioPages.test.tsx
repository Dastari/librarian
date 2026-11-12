import type { MockLink } from "@apollo/client/testing";
import { useApolloClient } from "@apollo/client/react";
import { act, configure, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  AlbumDetailDocument, AlbumTracksPageDocument, AudiobookChaptersPageDocument, AudiobookDetailDocument,
  AudiobookPartsDocument, EntityChapterUpdateDocument, EntityPlaybackProgressDeleteManyDocument, EntityTrackUpdateDocument, EntityTrackUpdateManyDocument, PlaybackProgressForFilesDocument,
  type AlbumDetailQuery, type AudiobookDetailQuery, type PlaybackProgressFieldsFragment,
} from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";
import { playerStore } from "@/features/player/store";

import { AlbumPage } from "../AlbumPage";
import { AudiobookPage } from "../AudiobookPage";

let admin = true;
let width = 1600;
const visibleEnds = new Set<IntersectionObserverCallback>();
const playAudio = vi.fn();
const enqueue = vi.fn();
vi.mock("@/features/player/usePlayer", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/features/player/usePlayer")>();
  return { ...actual, usePlayer: () => ({ ...actual.usePlayer(), playAudio, enqueue }) };
});
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => admin, useSession: () => ({ user: { id: "audio-user", role: admin ? "admin" : "member" } }) }));
vi.mock("@/hooks/useContentStatuses", async (original) => ({ ...(await original<typeof import("@/hooks/useContentStatuses")>()), useContentStatuses: () => new Map() }));
vi.mock("@/features/downloads/ReleaseSearchDialog", () => ({ ReleaseSearchDialog: () => null }));

type Track = NonNullable<AlbumDetailQuery["album"]>["tracks"]["edges"][number]["node"] & { __typename: "Track" };
type Chapter = NonNullable<AudiobookDetailQuery["audiobook"]>["chapters"]["edges"][number]["node"] & { __typename: "Chapter" };
const stamp = "2026-10-24T00:00:00Z";
const track = (n: number, overrides: Partial<Track> = {}): Track => ({
  __typename: "Track", id: `track-${n}`, albumId: "album", libraryId: "music", title: `Track ${n}`, trackNumber: n, discNumber: 1,
  musicbrainzId: null, isrc: null, durationSecs: 180, explicit: false, artistName: "Artist", artistId: "artist",
  wanted: false, ignored: false, mediaFileId: `track-file-${n}`, createdAt: stamp, updatedAt: stamp,
  // A non-null file id is playable even when the file relation is unavailable.
  mediaFile: null, ...overrides,
});
const chapter = (n: number, overrides: Partial<Chapter> = {}): Chapter => ({
  __typename: "Chapter", id: `chapter-${n}`, audiobookId: "book", title: `Part ${n}`, chapterNumber: n, startTimeSecs: 0, fileOffsetSecs: null,
  endTimeSecs: null, durationSecs: 300, wanted: false, ignored: false, mediaFileId: `chapter-file-${n}`,
  createdAt: stamp, updatedAt: stamp, mediaFile: null, ...overrides,
});
const connection = <T,>(rows: T[], totalCount = rows.length, hasNextPage = false) => ({
  edges: rows.map((node) => ({ node })), pageInfo: { totalCount, hasNextPage },
});
const album = (rows: Track[], total = rows.length, hasMore = false): NonNullable<AlbumDetailQuery["album"]> & { __typename: "Album" } => ({
  __typename: "Album", id: "album", artistId: "artist", libraryId: "music", userId: "audio-user", name: "Recording", sortName: null,
  year: 2026, musicbrainzId: null, albumType: null, genres: [], label: null, country: null, releaseDate: null,
  coverUrl: null, lockedFields: null, trackCount: total, discCount: 2, totalDurationSecs: null, autoDownload: false, autoDownloadMode: "NONE",
  qualityProfileId: null, hasFiles: true, sizeBytes: null, path: null, createdAt: stamp, updatedAt: stamp,
  library: null, wantedMissing: { pageInfo: { totalCount: 0 } }, tracks: connection(rows, total, hasMore),
});
const book = (rows: Chapter[], total = rows.length, hasMore = false): NonNullable<AudiobookDetailQuery["audiobook"]> & { __typename: "Audiobook" } => ({
  __typename: "Audiobook", id: "book", lockedFields: null, libraryId: "books", userId: "audio-user", title: "Long book", sortTitle: null, authorName: "Author", seriesName: null, seriesPosition: null, chapterCheckState: null, chapterCheck: null, chapterSource: null, fileShape: null, providerChapters: null,
  narratorName: null, narrators: [], description: null, publisher: null, publishedDate: null, language: null,
  isbn: null, asin: null, audibleId: null, goodreadsId: null, totalDurationSecs: null, chapterCount: total,
  coverUrl: null, autoDownload: false, autoDownloadMode: "NONE", qualityProfileId: null, hasFiles: true,
  sizeBytes: null, path: null, createdAt: stamp, updatedAt: stamp, library: null, wantedMissing: { pageInfo: { totalCount: 0 } }, chapters: connection(rows, total, hasMore),
});
const saved = (n: number, position: number, watched = false): PlaybackProgressFieldsFragment & { __typename: "PlaybackProgress" } => ({
  __typename: "PlaybackProgress", id: `progress-${n}`, userId: "audio-user", mediaFileId: `chapter-file-${n}`, currentPosition: position,
  duration: 300, progressPercent: position / 3, isWatched: watched, watchedAt: watched ? stamp : null,
  createdAt: stamp, updatedAt: stamp,
});
const progressMock = (rows: PlaybackProgressFieldsFragment[] = [], error?: Error) => ({
  request: { query: PlaybackProgressForFilesDocument, variables: () => true },
  ...(error ? { error } : { result: (variables: { mediaFileIds: string[] }) => ({ data: { playbackProgresses: { edges: rows.filter((row) => variables.mediaFileIds.includes(row.mediaFileId!)).map((node) => ({ node })) } } }) }),
});
const albumMock = (rows: Track[], total = rows.length, more = false) => ({ request: { query: AlbumDetailDocument, variables: { id: "album" } }, result: { data: { album: album(rows, total, more) } } });
const bookMock = (rows: Chapter[], total = rows.length, more = false) => ({ request: { query: AudiobookDetailDocument, variables: { id: "book" } }, result: { data: { audiobook: book(rows, total, more) } } });
const albumPageMock = (rows: Track[], total: number, offset = 100, more = false) => ({ request: { query: AlbumTracksPageDocument, variables: { id: "album", offset } }, result: { data: { album: { __typename: "Album", id: "album", tracks: connection(rows, total, more) } } } });
const bookPageMock = (rows: Chapter[], total: number) => ({ request: { query: AudiobookChaptersPageDocument, variables: { id: "book", offset: 100 } }, result: { data: { audiobook: { __typename: "Audiobook", id: "book", chapters: connection(rows, total) } } } });
const mount = (kind: "album" | "book", mocks: MockLink.MockedResponse[]) => renderWithProviders(kind === "album" ? <AlbumPage albumId="album" /> : <AudiobookPage audiobookId="book" />, { mocks });
const table = (kind: "album" | "book") => document.querySelector<HTMLElement>(`[role="grid"][aria-label="${kind === "album" ? "Recording tracks" : "Long book chapters"}"]`)!;
const row = (kind: "album" | "book", title: string) => {
  const id = `${kind === "album" ? "track" : "chapter"}-${title.split(" ").at(-1)}`;
  const found = table(kind)?.querySelector<HTMLElement>(`[data-row-id="${id}"]`);
  expect(found).toBeInTheDocument();
  return found!;
};
const findRow = async (kind: "album" | "book", title: string) => {
  if (Number(title.split(" ").at(-1)) > 100) {
    await waitFor(() => expect(visibleEnds.size).toBeGreaterThan(0));
    act(() => visibleEnds.forEach((callback) => callback([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver)));
  }
  return waitFor(() => row(kind, title));
};
const hero = () => document.querySelector("h1")!.closest("section")!;
const notice = (message: RegExp) => waitFor(() => {
  const found = Array.from(document.querySelectorAll<HTMLElement>("[role=status]")).find((node) => message.test(node.textContent ?? ""));
  expect(found).toBeVisible();
  return found!;
});

beforeEach(() => {
  configure({ asyncUtilTimeout: 15000 });
  admin = true;
  width = 1600;
  visibleEnds.clear();
  vi.stubGlobal("IntersectionObserver", class {
    constructor(private callback: IntersectionObserverCallback) {}
    observe() { visibleEnds.add(this.callback); }
    unobserve() { visibleEnds.delete(this.callback); }
    disconnect() { visibleEnds.delete(this.callback); }
  });
  vi.clearAllMocks();
  playerStore.stopAudio();
  // Keep the real Glass table/cards/actions; jsdom only needs container measurements.
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    const measured = this.hasAttribute("data-measure") ? 110 : width;
    return { x: 0, y: 0, top: 0, left: 0, bottom: 800, right: measured, width: measured, height: 800, toJSON: () => ({}) };
  });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 32 : width; });
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 48 : 800; });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(() => width);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(800);
});
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); configure({ asyncUtilTimeout: 1000 }); });

describe("audio detail tables", { timeout: 30000 }, () => {
  it("loads successive album pages and plays a later row in disc/track order", async () => {
    const first = Array.from({ length: 100 }, (_, i) => track(i + 1, { discNumber: 2 }));
    const second = Array.from({ length: 100 }, (_, i) => track(i + 101, { discNumber: 3 }));
    const last = track(201, { discNumber: 1 });
    await mount("album", [albumMock(first, 201, true), albumPageMock(second, 201, 100, true), albumPageMock([last], 201, 200)]);
    const later = await findRow("album", "Track 201");
    const play = within(later).getByRole("button", { name: "Play" });
    expect(play).toBeVisible();
    expect(play.closest("[class*='opacity-0']")).toBeNull();
    await userEvent.click(play);
    expect(playAudio).toHaveBeenLastCalledWith(expect.arrayContaining([expect.objectContaining({ mediaFileId: "track-file-201" })]), 0);
    const queue = playAudio.mock.lastCall![0];
    expect(queue).toHaveLength(201);
    expect(queue.slice(0, 3).map((item: { entity: { id: string } }) => item.entity.id)).toEqual(["track-201", "track-1", "track-2"]);
  });

  it("treats exactly 100 tracks with no next page as a complete album", async () => {
    await mount("album", [albumMock(Array.from({ length: 100 }, (_, i) => track(i + 1)))]);
    await findRow("album", "Track 100");
    await userEvent.click(within(hero()).getByRole("button", { name: "Play" }));
    expect(playAudio.mock.lastCall![0]).toHaveLength(100);
  });

  it("keeps loaded controls readable on paging failure and retries without claiming a full queue", async () => {
    const first = Array.from({ length: 100 }, (_, i) => track(i + 1));
    await mount("album", [albumMock(first, 101, true), {
      request: { query: AlbumTracksPageDocument, variables: { id: "album", offset: 100 } }, error: new Error("Offline"), maxUsageCount: 1,
    }, albumPageMock([track(101)], 101)]);
    const error = await notice(/Could not load all tracks/);
    expect(within(hero()).getByRole("button", { name: "Play" })).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(within(row("album", "Track 1")).getByRole("button", { name: "Play" }));
    expect(playAudio.mock.lastCall![0]).toHaveLength(1);
    await userEvent.click(within(error).getByRole("button", { name: "Retry tracks" }));
    await findRow("album", "Track 101");
    await userEvent.click(within(hero()).getByRole("button", { name: "Play" }));
    expect(playAudio.mock.lastCall![0]).toHaveLength(101);
  });

  it.each(["album", "book"] as const)("hides acquisition actions for a %s member while keeping playback", async (kind) => {
    admin = false;
    await mount(kind, kind === "album" ? [albumMock([track(1), track(2, { mediaFileId: null })])] : [bookMock([chapter(1), chapter(2, { mediaFileId: null })]), progressMock()]);
    await findRow(kind, kind === "album" ? "Track 2" : "Part 2");
    expect(screen.queryByRole("button", { name: /Want|Find releases|Download settings|Remove from library/ })).not.toBeInTheDocument();
    const play = () => within(row(kind, kind === "album" ? "Track 1" : "Part 1")).getByRole("button", { name: "Play" });
    await waitFor(() => expect(play()).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(play());
    expect(playAudio).toHaveBeenCalled();
  });

  it.each(["album", "book"] as const)("preserves the focused Want button through a mutation and cache update in the %s table", async (kind) => {
    const node = kind === "album" ? track(1, { mediaFileId: null }) : chapter(1, { mediaFileId: null });
    let wanted = false;
    const detail = kind === "album" ? AlbumDetailDocument : AudiobookDetailDocument;
    const initial = { request: { query: detail, variables: { id: kind === "album" ? "album" : "book" } }, result: vi.fn(() => ({ data: kind === "album" ? { album: album([{ ...node, wanted } as Track]) } : { audiobook: book([{ ...node, wanted } as Chapter]) } })) };
    const mutation = { request: { query: kind === "album" ? EntityTrackUpdateDocument : EntityChapterUpdateDocument, variables: { id: node.id, input: { wanted: true } } }, result: vi.fn(() => {
      wanted = true;
      return { data: { [kind === "album" ? "updateTrack" : "updateChapter"]: { success: true, error: null, [kind === "album" ? "track" : "chapter"]: { ...node, wanted: true } } } };
    }) };
    await mount(kind, [initial, mutation]);
    const want = await screen.findByRole("button", { name: "Want" });
    want.focus();
    expect(want).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(screen.getByRole("button", { name: "Wanted" })).toBeVisible());
    expect(mutation.result).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(initial.result).toHaveBeenCalledTimes(2));
    expect(want.isConnected).toBe(true);
    expect(screen.getByRole("button", { name: "Wanted" })).toBe(want);
    expect(want).toHaveFocus();
  });

  it("keeps a later-page Want result and refetches the album", async () => {
    const first = Array.from({ length: 100 }, (_, i) => track(i + 1));
    const last = track(101, { mediaFileId: null });
    let wanted = false;
    const detail = { request: { query: AlbumDetailDocument, variables: { id: "album" } }, result: vi.fn(() => ({ data: { album: album(first, 101, true) } })) };
    const page = { request: { query: AlbumTracksPageDocument, variables: { id: "album", offset: 100 } }, result: () => ({ data: { album: { __typename: "Album", id: "album", tracks: connection([{ ...last, wanted }], 101) } } }) };
    const mutation = { request: { query: EntityTrackUpdateDocument, variables: { id: last.id, input: { wanted: true } } }, result: () => {
      wanted = true;
      return { data: { updateTrack: { success: true, error: null, track: { ...last, wanted } } } };
    } };
    await mount("album", [detail, page, mutation]);
    await findRow("album", "Track 101");
    await userEvent.click(within(row("album", "Track 101")).getByRole("button", { name: "Want" }));
    await waitFor(() => expect(within(row("album", "Track 101")).getByRole("button", { name: "Wanted" })).toBeVisible());
    await waitFor(() => expect(detail.result).toHaveBeenCalledTimes(2));
  });

  it("keeps individual chapters playable after a later page fails and retries the book", async () => {
    const first = Array.from({ length: 100 }, (_, i) => chapter(i + 1));
    await mount("book", [bookMock(first, 101, true), {
      request: { query: AudiobookChaptersPageDocument, variables: { id: "book", offset: 100 } }, error: new Error("Offline"), maxUsageCount: 1,
    }, bookPageMock([chapter(101)], 101), progressMock()]);
    const error = await notice(/Could not load all chapters/);
    expect(within(hero()).getByRole("button", { name: "Listen" })).toHaveAttribute("aria-disabled", "true");
    const play = () => within(row("book", "Part 1")).getByRole("button", { name: "Play" });
    await waitFor(() => expect(play()).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(play());
    expect(playAudio.mock.lastCall![0]).toHaveLength(1);
    await userEvent.click(within(error).getByRole("button", { name: "Retry chapters" }));
    await findRow("book", "Part 101");
    await waitFor(() => expect(within(hero()).getByRole("button", { name: "Listen" })).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(within(hero()).getByRole("button", { name: "Listen" }));
    expect(playAudio.mock.lastCall![0]).toHaveLength(101);
  });

  it("loads chapter 101 and its progress, then restarts a watched chapter at zero", async () => {
    const first = Array.from({ length: 100 }, (_, i) => chapter(i + 1));
    await mount("book", [bookMock(first, 101, true), bookPageMock([chapter(101)], 101), progressMock([
      ...first.map((_, i) => saved(i + 1, 300, true)), saved(101, 45),
    ])]);
    await findRow("book", "Part 101");
    const resume = await within(hero()).findByRole("button", { name: "Resume chapter 101" });
    await waitFor(() => expect(resume).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(resume);
    expect(playAudio.mock.lastCall![1]).toBe(100);
    expect(playAudio.mock.lastCall![0][100]).toMatchObject({ mediaFileId: "chapter-file-101", startPosition: 45, entity: { kind: "chapter", id: "chapter-101" }, audiobookId: "book", href: "/audiobooks/book" });
    const later = row("book", "Part 101");
    expect(within(later).getByRole("button", { name: "Resume" })).toBeVisible();
    await userEvent.click(within(row("book", "Part 1")).getByRole("button", { name: "Play again" }));
    expect(playAudio.mock.lastCall![1]).toBe(0);
    expect(playAudio.mock.lastCall![0][0]).toMatchObject({ startPosition: 0 });
  });

  it("updates chapter progress and current-player cells without replacing their controls", async () => {
    let client!: ReturnType<typeof useApolloClient>;
    function Page() {
      client = useApolloClient();
      return <AudiobookPage audiobookId="book" />;
    }
    await renderWithProviders(<Page />, { mocks: [bookMock([chapter(1), chapter(2)]), progressMock()] });
    const first = await findRow("book", "Part 1");
    const second = row("book", "Part 2");
    const play = await waitFor(() => {
      const control = within(first).getByRole("button", { name: "Play" });
      expect(control).not.toHaveAttribute("aria-disabled", "true");
      return control;
    });
    const title = within(first).getByText("Part 1");
    const nextTitle = within(second).getByText("Part 2");
    expect(title).not.toHaveClass("text-brand");

    act(() => playerStore.playQueue([1, 2].map((n) => ({
      mediaFileId: `chapter-file-${n}`, title: "Long book", entity: { kind: "chapter", id: `chapter-${n}` },
    })), 0));
    await waitFor(() => expect(title).toHaveClass("text-brand"));
    expect(within(first).getByRole("button", { name: "Pause" })).toBe(play);

    act(() => client.cache.writeQuery({
      query: PlaybackProgressForFilesDocument,
      variables: { userId: "audio-user", mediaFileIds: ["chapter-file-1", "chapter-file-2"] },
      data: { playbackProgresses: { edges: [saved(1, 45), saved(2, 300, true)].map((node) => ({ node })) } },
    }));
    await waitFor(() => expect(within(second).getByRole("button", { name: "Play again" })).toBeVisible());
    expect(play.isConnected).toBe(true);

    act(() => playerStore.jumpTo(1));
    await waitFor(() => expect(nextTitle).toHaveClass("text-brand"));
    expect(title).not.toHaveClass("text-brand");
    expect(within(first).getByRole("button", { name: "Resume" })).toBe(play);
    await userEvent.click(play);
    expect(playAudio.mock.lastCall![0][0]).toMatchObject({ mediaFileId: "chapter-file-1", startPosition: 45 });
  });

  it("labels in-progress chapter 1 as Resume chapter 1 and keeps actions in narrow cards", async () => {
    width = 390;
    await mount("book", [bookMock([chapter(1), chapter(2, { mediaFileId: null })]), progressMock([saved(1, 23)])]);
    await screen.findByRole("heading", { name: "Long book" });
    const resume = await within(hero()).findByRole("button", { name: "Resume chapter 1" });
    await waitFor(() => expect(resume).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(resume);
    expect(playAudio.mock.lastCall![0][0]).toMatchObject({ startPosition: 23 });
    expect(within(table("book")).getByRole("button", { name: "Resume" })).toBeVisible();
    expect(within(table("book")).getByRole("button", { name: "Want" })).toBeVisible();
  });

  it("disables chapter playback until failed progress can be retried", async () => {
    await mount("book", [bookMock([chapter(1)]), { ...progressMock([], new Error("Offline")), maxUsageCount: 1 }, progressMock([saved(1, 17)])]);
    const error = await notice(/Could not load playback progress/);
    const play = within(row("book", "Part 1")).getByRole("button", { name: "Play" });
    expect(play).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(play);
    expect(playAudio).not.toHaveBeenCalled();
    await userEvent.click(within(error).getByRole("button", { name: "Retry progress" }));
    const resume = await within(hero()).findByRole("button", { name: "Resume chapter 1" });
    await waitFor(() => expect(resume).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(resume);
    expect(playAudio.mock.lastCall![0][0]).toMatchObject({ startPosition: 17 });
  });
});


describe("selected audio detail actions", { timeout: 30000 }, () => {
  it("plays only selected tracks in album order and queues the same selection", async () => {
    await mount("album", [albumMock([track(1), track(2), track(3)])]);
    await findRow("album", "Track 3");
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Track 3" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Track 1" }));
    await userEvent.click(screen.getByRole("button", { name: "Actions for 2 selected tracks" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Play selected tracks" }));
    await waitFor(() => expect(playAudio).toHaveBeenCalled());
    expect(playAudio.mock.lastCall![0].map((item: { entity: { id: string } }) => item.entity.id)).toEqual(["track-1", "track-3"]);
    await userEvent.click(screen.getByRole("button", { name: "Actions for 2 selected tracks" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Add to queue" }));
    // The queue actions write to the player store directly; with nothing playing, queueing starts it.
    await waitFor(() => expect(playerStore.getSnapshot().queue).toHaveLength(2));
    expect(playerStore.getSnapshot().queue.map((item) => item.entity.id)).toEqual(["track-1", "track-3"]);
    playerStore.stopAudio();
  });

  it("updates only selected missing tracks and disables playback of mixed availability", async () => {
    const rows = [track(1), track(2, { mediaFileId: null }), track(3, { mediaFileId: null }), track(4, { mediaFileId: null })];
    const updated = vi.fn(() => ({ data: { updateTracks: { success: true, error: null, affectedCount: 2 } } }));
    await mount("album", [albumMock(rows), {
      request: { query: EntityTrackUpdateManyDocument, variables: { where: { id: { inList: ["track-2", "track-3"] }, albumId: { eq: "album" }, mediaFileId: { isNull: true } }, input: { wanted: true } } }, result: updated,
    }, albumMock(rows.map((row) => ["track-2", "track-3"].includes(row.id) ? { ...row, wanted: true } : row))]);
    await findRow("album", "Track 4");
    for (const n of [1, 2, 3]) await userEvent.click(screen.getByRole("checkbox", { name: `Select Track ${n}` }));
    await userEvent.click(screen.getByRole("button", { name: "Actions for 3 selected tracks" }));
    expect(await screen.findByRole("menuitem", { name: "Play selected tracks" })).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(screen.getByRole("menuitem", { name: "Want selected missing tracks" }));
    await waitFor(() => expect(updated).toHaveBeenCalledOnce());
    expect(playAudio).not.toHaveBeenCalled();
  });

  it("keeps selected chapter order and resumes only its first unfinished chapter", async () => {
    await mount("book", [bookMock([chapter(1), chapter(2), chapter(3)]), progressMock([saved(2, 45), saved(3, 300, true)])]);
    await findRow("book", "Part 3");
    await waitFor(() => expect(within(row("book", "Part 2")).getByRole("button", { name: "Resume" })).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Part 3" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Part 2" }));
    await userEvent.click(screen.getByRole("button", { name: "Actions for 2 selected chapters" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Play selected chapters" }));
    await waitFor(() => expect(playAudio).toHaveBeenCalled());
    expect(playAudio.mock.lastCall![0]).toEqual([
      expect.objectContaining({ mediaFileId: "chapter-file-2", startPosition: 45, audiobookId: "book" }),
      expect.objectContaining({ mediaFileId: "chapter-file-3", startPosition: 0, audiobookId: "book" }),
    ]);
  });

  it("keeps acquisition bulk actions unavailable to members", async () => {
    admin = false;
    await mount("album", [albumMock([track(1, { mediaFileId: null }), track(2, { mediaFileId: null })])]);
    await findRow("album", "Track 2");
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Track 1" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Track 2" }));
    await userEvent.click(screen.getByRole("button", { name: "Actions for 2 selected tracks" }));
    await screen.findByRole("menuitem", { name: "Copy track titles" });
    expect(screen.queryByRole("menuitem", { name: /Want selected/ })).toBeNull();
    expect(screen.getByRole("menuitem", { name: "Play selected tracks" })).toHaveAttribute("aria-disabled", "true");
  });
});

describe("marking audio unplayed", { timeout: 30000 }, () => {
  const heard = (file: string, overrides: Partial<PlaybackProgressFieldsFragment> = {}) => ({ ...saved(1, 40), id: `progress-${file}`, mediaFileId: file, ...overrides });
  const resetMock = (mediaFileIds: string[]) => {
    const called = vi.fn(() => ({ data: { deletePlaybackProgresses: { __typename: "DeletePlaybackProgressesResult", success: true, error: null } } }));
    return { called, mock: { request: { query: EntityPlaybackProgressDeleteManyDocument, variables: { where: { userId: { eq: "audio-user" }, mediaFileId: { inList: mediaFileIds } } } }, result: called } };
  };
  const menuItems = async (button: HTMLElement) => {
    await userEvent.click(button);
    return within(await screen.findByRole("menu")).getAllByRole("menuitem").map((item) => item.textContent);
  };
  const menuClosed = () => waitFor(() => expect(screen.queryByRole("menu")).not.toBeInTheDocument());
  const rowMenu = (kind: "album" | "book", title: string) => within(row(kind, title)).getByRole("button", { name: /^More actions/ });

  it("offers a track only the action it can use, and resets just that track's file", async () => {
    const tracks = [track(1), track(2), track(3)];
    const { called, mock } = resetMock(["track-file-1"]);
    await mount("album", [albumMock(tracks), progressMock([heard("track-file-1"), heard("track-file-3", { currentPosition: 0, isWatched: true })]), mock]);
    await findRow("album", "Track 3");
    await waitFor(() => expect(within(row("album", "Track 1")).getByRole("button", { name: "Resume" })).not.toHaveAttribute("aria-disabled", "true"));
    expect(await menuItems(rowMenu("album", "Track 2"))).not.toContain("Mark as unplayed");
    await userEvent.keyboard("{Escape}");
    await menuClosed();
    await userEvent.click(rowMenu("album", "Track 1"));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Mark as unplayed" }));
    await waitFor(() => expect(called).toHaveBeenCalledOnce());
  });

  it("resets the whole album from its header, and a selection of tracks", async () => {
    const tracks = [track(1), track(2), track(3)];
    const everything = resetMock(["track-file-1", "track-file-3"]);
    await mount("album", [albumMock(tracks), progressMock([heard("track-file-1"), heard("track-file-3", { currentPosition: 0, isWatched: true })]), everything.mock]);
    await findRow("album", "Track 3");
    await waitFor(() => expect(within(row("album", "Track 1")).getByRole("button", { name: "Resume" })).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(within(hero()).getByRole("button", { name: /^More actions/ }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Mark as unplayed" }));
    await waitFor(() => expect(everything.called).toHaveBeenCalledOnce());
  });

  it("resets a selection of tracks in one request", async () => {
    const tracks = [track(1), track(2), track(3)];
    const picked = resetMock(["track-file-1"]);
    await mount("album", [albumMock(tracks), progressMock([heard("track-file-1")]), picked.mock]);
    await findRow("album", "Track 3");
    await waitFor(() => expect(within(row("album", "Track 1")).getByRole("button", { name: "Resume" })).not.toHaveAttribute("aria-disabled", "true"));
    for (const n of [1, 2]) await userEvent.click(screen.getByRole("checkbox", { name: `Select Track ${n}` }));
    await userEvent.click(screen.getByRole("button", { name: "Actions for 2 selected tracks" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Mark as unplayed" }));
    await waitFor(() => expect(picked.called).toHaveBeenCalledOnce());
  });

  it("is a member's action as much as an administrator's", async () => {
    admin = false;
    const tracks = [track(1), track(2)];
    const { called, mock } = resetMock(["track-file-1"]);
    await mount("album", [albumMock(tracks), progressMock([heard("track-file-1")]), mock]);
    await findRow("album", "Track 2");
    await waitFor(() => expect(within(row("album", "Track 1")).getByRole("button", { name: "Resume" })).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(rowMenu("album", "Track 1"));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Mark as unplayed" }));
    await waitFor(() => expect(called).toHaveBeenCalledOnce());
  });

  it("offers nothing on an album with no progress", async () => {
    await mount("album", [albumMock([track(1), track(2)]), progressMock()]);
    await findRow("album", "Track 2");
    expect(await menuItems(within(hero()).getByRole("button", { name: /^More actions/ }))).not.toContain("Mark as unplayed");
  });

  it("resets a chapter that has a file of its own, and the book through every file", async () => {
    const chapters = [chapter(1), chapter(2), chapter(3)];
    const one = resetMock(["chapter-file-2"]);
    const whole = resetMock(["chapter-file-2", "chapter-file-3"]);
    await mount("book", [bookMock(chapters), progressMock([saved(2, 45), saved(3, 300, true)]), one.mock, whole.mock]);
    await findRow("book", "Part 3");
    await waitFor(() => expect(within(row("book", "Part 2")).getByRole("button", { name: "Resume" })).not.toHaveAttribute("aria-disabled", "true"));
    expect(await menuItems(rowMenu("book", "Part 1"))).not.toContain("Mark as unplayed");
    await userEvent.keyboard("{Escape}");
    await menuClosed();
    await userEvent.click(rowMenu("book", "Part 2"));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Mark as unplayed" }));
    await waitFor(() => expect(one.called).toHaveBeenCalledOnce());
    await menuClosed();
    await userEvent.click(within(hero()).getByRole("button", { name: /^More actions/ }));
    // Progress is read again after a reset, and the book's action waits for that.
    await waitFor(() => expect(screen.getByRole("menuitem", { name: "Mark as unplayed" })).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(screen.getByRole("menuitem", { name: "Mark as unplayed" }));
    await waitFor(() => expect(whole.called).toHaveBeenCalledOnce());
  });

  it("only offers the whole book when its chapters are places in one file", async () => {
    const shared = [1, 2, 3].map((n) => chapter(n, { mediaFileId: "book-file", startTimeSecs: (n - 1) * 100 }));
    const { called, mock } = resetMock(["book-file"]);
    await mount("book", [bookMock(shared), progressMock([heard("book-file", { duration: 300 })]), mock]);
    await findRow("book", "Part 3");
    await waitFor(() => expect(within(hero()).getByRole("button", { name: /^Resume chapter/ })).not.toHaveAttribute("aria-disabled", "true"));
    // One saved position can't be split between chapters, so no chapter offers its own reset.
    for (const part of ["Part 1", "Part 2", "Part 3"]) {
      expect(await menuItems(rowMenu("book", part))).not.toContain("Mark as unplayed");
      await userEvent.keyboard("{Escape}");
      await menuClosed();
    }
    await userEvent.click(within(hero()).getByRole("button", { name: /^More actions/ }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Mark as unplayed" }));
    await waitFor(() => expect(called).toHaveBeenCalledOnce());
  });

  it("resets a book cut into parts through every part, including one no chapter starts in", async () => {
    // Chapters are places in the parts; listening is saved against the part files themselves.
    const chapters = [1, 2].map((n) => chapter(n, { mediaFileId: `part-${n}`, fileOffsetSecs: 0 }));
    const parts = ["part-1", "part-2", "part-3"].map((id, index) => ({ __typename: "MediaFile", id, chapterId: chapters[Math.min(index, 1)]!.id, path: `/books/${id}.mp3`, duration: 300 }));
    const partsMock = { request: { query: AudiobookPartsDocument, variables: () => true }, result: { data: { mediaFiles: { __typename: "MediaFileConnection", edges: parts.map((node) => ({ __typename: "MediaFileEdge", node })) } } } };
    const unreferenced = heard("part-3", { currentPosition: 0, isWatched: true });
    const { called, mock } = resetMock(["part-1", "part-3"]);
    await mount("book", [{ ...bookMock(chapters), result: { data: { audiobook: { ...book(chapters), fileShape: "parts" } } } }, partsMock, progressMock([heard("part-1"), unreferenced]), mock]);
    await findRow("book", "Part 2");
    await waitFor(() => expect(within(hero()).getByRole("button", { name: /^Resume chapter|^Listen/ })).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(within(hero()).getByRole("button", { name: /^More actions/ }));
    await waitFor(() => expect(screen.getByRole("menuitem", { name: "Mark as unplayed" })).not.toHaveAttribute("aria-disabled", "true"));
    await userEvent.click(screen.getByRole("menuitem", { name: "Mark as unplayed" }));
    await waitFor(() => expect(called).toHaveBeenCalledOnce());
  });

  it("waits for the whole list before offering to reset a long book", async () => {
    const first = Array.from({ length: 100 }, (_, i) => chapter(i + 1));
    // The last chapter never arrives, so the book can't claim to reset all of it.
    await mount("book", [bookMock(first, 101, true), { request: { query: AudiobookChaptersPageDocument, variables: { id: "book", offset: 100 } }, error: new Error("Offline") }, progressMock([saved(1, 45)])]);
    await notice(/Could not load all chapters/);
    await userEvent.click(within(hero()).getByRole("button", { name: /^More actions/ }));
    const reset = await screen.findByRole("menuitem", { name: "Mark as unplayed" });
    expect(reset).toHaveAttribute("aria-disabled", "true");
  });
});
