import { render, screen } from "@testing-library/react";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import { describe, expect, it, vi } from "vitest";

import type { PersonalEntryFragment, PlaylistCoverEntryFragment } from "@/graphql/generated/graphql";

import { PlaylistIdentity, playlistState } from "../PlaylistIdentity";
import { entryCover, playlistCovers } from "../playlistArt";
import { playlistDuration } from "../playlistTotals";
import { entryPresentation, entryTarget, targetFields } from "../types";
import { personalEntry, movieFields, trackFields, playlist } from "./personalHarness";

const cover = (overrides: Partial<PlaylistCoverEntryFragment>): PlaylistCoverEntryFragment => ({
  id: "e", movieId: null, showId: null, albumId: null, audiobookId: null, trackId: null, episodeId: null, chapterId: null, track: null, episode: null, chapter: null, ...overrides,
});
const node = (overrides: Partial<PlaylistCoverEntryFragment>) => ({ node: cover(overrides) });

describe("a playlist's identity", () => {
  it("says what a list is: smart, pinned, empty or how many it holds", () => {
    expect(playlistState(playlist({ kind: "SMART" }))).toMatchObject({ id: "smart", tone: "info" });
    expect(playlistState(playlist({}, 3), { pinned: true })).toMatchObject({ id: "pinned", label: "On Home" });
    expect(playlistState(playlist({}, 0))).toMatchObject({ id: "empty", tone: "neutral" });
    expect(playlistState(playlist({}, 1))).toMatchObject({ id: "ready", label: "1 item", tone: "success" });
    expect(playlistState(playlist({ kind: "WATCH_LATER" }, 5))).toMatchObject({ id: "watch-later", label: "5 items" });
  });

  it("shows its name, state and facts at the header tier, and the state at the compact ones too", () => {
    const list = playlist({}, 12);
    const { rerender } = render(<TooltipProvider><ActionProvider navigate={vi.fn()}><PlaylistIdentity tier="header" heading="h2" list={list} duration="3h 20m" /></ActionProvider></TooltipProvider>);
    expect(screen.getByRole("heading", { name: /Evening films/ })).toBeTruthy();
    expect(screen.getByText("Playlist · 12 items · 3h 20m")).toBeTruthy();
    for (const tier of ["row", "compact", "chip", "icon"] as const) {
      rerender(<TooltipProvider><ActionProvider navigate={vi.fn()}><PlaylistIdentity tier={tier} list={list} /></ActionProvider></TooltipProvider>);
      // Every size names the playlist and carries its state for assistive technology.
      expect(screen.getAllByLabelText(/Evening films|12 items/).length).toBeGreaterThan(0);
    }
  });
});

describe("a playlist's picture", () => {
  it("takes each kind's own artwork, a track's from its album, an episode's from its show", () => {
    expect(entryCover(cover({ movieId: "m" }))).toBe("/api/artwork/movie/m/poster");
    expect(entryCover(cover({ albumId: "a" }))).toBe("/api/artwork/album/a/cover");
    expect(entryCover(cover({ trackId: "t", track: { albumId: "al" } }))).toBe("/api/artwork/album/al/cover");
    expect(entryCover(cover({ episodeId: "e", episode: { showId: "s" } }))).toBe("/api/artwork/show/s/poster");
    expect(entryCover(cover({ chapterId: "c", chapter: { audiobookId: "b" } }))).toBe("/api/artwork/audiobook/b/cover");
    expect(entryCover(cover({}))).toBeUndefined();
  });

  it("makes one tile of several entries from the same album and stops at four", () => {
    const entries = [node({ trackId: "1", track: { albumId: "a" } }), node({ trackId: "2", track: { albumId: "a" } }), node({ movieId: "m" }), node({ albumId: "b" }), node({ albumId: "c" }), node({ albumId: "d" })].map((edge) => edge.node);
    expect(playlistCovers(entries)).toEqual(["/api/artwork/album/a/cover", "/api/artwork/movie/m/poster", "/api/artwork/album/b/cover", "/api/artwork/album/c/cover"]);
    expect(playlistCovers(undefined)).toEqual([]);
  });
});

describe("what a playlist runs for", () => {
  const entries = (...rows: Record<string, unknown>[]) => rows.map((row, index) => personalEntry(index, { movieId: null, ...row }));
  it("adds each entry's own length, minutes for films and episodes, seconds for tracks and chapters", () => {
    const total = playlistDuration(entries({ movie: movieFields("m", "Film", { runtime: 90 }) }, { trackId: "t", track: trackFields("t", "Song", { durationSecs: 200 }) }, { chapter: { durationSecs: 600 } }, { episode: { runtime: 40 } }));
    expect(total).toEqual({ seconds: 5400 + 200 + 600 + 2400, partial: false });
  });
  it("counts an album's and an audiobook's totals, and reports a show, which has none, as at least", () => {
    expect(playlistDuration(entries({ album: { totalDurationSecs: 3000 } }, { audiobook: { totalDurationSecs: 7200 } }))).toEqual({ seconds: 10200, partial: false });
    expect(playlistDuration(entries({ album: { totalDurationSecs: 3000 } }, { show: { id: "s" } }))).toEqual({ seconds: 3000, partial: true });
  });
});

describe("what an entry points at", () => {
  it("finds the one target of an entry, for any of the seven kinds", () => {
    expect(entryTarget(personalEntry(0, { movieId: null, episodeId: "e" }))).toEqual({ kind: "episode", id: "e" });
    expect(entryTarget(personalEntry(0, { movieId: null, chapterId: "c" }))).toEqual({ kind: "chapter", id: "c" });
    expect(entryTarget(personalEntry(0, { movieId: null }))).toBeNull();
  });
  it("writes a target as exactly one id with the others cleared", () => {
    expect(targetFields({ kind: "chapter", id: "c" })).toEqual({ movieId: null, showId: null, albumId: null, audiobookId: null, trackId: null, episodeId: null, chapterId: "c" });
  });
  it("names episodes and chapters for shelves and rows", () => {
    const entry = personalEntry(0, { movieId: null, episode: { id: "e", showId: "s", season: 2, episode: 5, title: "The one", show: { id: "s", name: "Series" } } }) as PersonalEntryFragment;
    expect(entryPresentation(entry)).toMatchObject({ title: "The one", subtitle: "Series · S02E05", href: "/shows/s" });
    const chapter = personalEntry(0, { movieId: null, chapter: { id: "c", audiobookId: "b", chapterNumber: 3, title: null, audiobook: { id: "b", title: "Book" } } }) as PersonalEntryFragment;
    expect(entryPresentation(chapter)).toMatchObject({ title: "Chapter 3", subtitle: "Book", href: "/audiobooks/b" });
  });
});
