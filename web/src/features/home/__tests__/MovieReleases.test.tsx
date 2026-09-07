import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AddMovieDocument, LibrariesOverviewDocument, LibraryMoviesByTmdbIdsDocument, MovieReleaseGuideDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { MovieReleases } from "../MovieReleases";

const release = (providerId: number, title: string, releaseDate: string | null) => ({
  __typename: "ProviderMovieResult" as const,
  provider: "tmdb",
  providerId,
  title,
  originalTitle: title,
  year: releaseDate ? Number(releaseDate.slice(0, 4)) : null,
  releaseDate,
  overview: "",
  posterUrl: `https://cdn/${providerId}.jpg`,
  backdropUrl: null,
  imdbId: null,
  voteAverage: 7.5,
  popularity: 100,
});

const guideMock = (kind: "NOW_PLAYING" | "UPCOMING", releases: unknown[]) => ({
  request: { query: MovieReleaseGuideDocument, variables: { kind, region: "US", page: 1 } },
  result: { data: { movieReleases: releases } },
});

const guideError = (kind: "NOW_PLAYING" | "UPCOMING") => ({
  request: { query: MovieReleaseGuideDocument, variables: { kind, region: "US", page: 1 } },
  error: new Error("TMDB is unreachable"),
});

const ownedMock = (ids: number[], owned: Array<{ id: string; tmdbId: number; hasFile: boolean; wanted: boolean }>) => ({
  request: { query: LibraryMoviesByTmdbIdsDocument, variables: { ids } },
  result: {
    data: {
      movies: {
        __typename: "MovieConnection",
        edges: owned.map((movie) => ({ __typename: "MovieEdge", node: { __typename: "Movie", ...movie } })),
      },
    },
  },
});

const library = (id: string, name: string, libraryType: string) => ({
  __typename: "LibraryEdge" as const,
  node: {
    __typename: "Library" as const,
    id,
    userId: "u1",
    name,
    path: `/media/${id}`,
    libraryType,
    icon: null,
    color: null,
    autoScan: false,
    autoOrganize: false,
    namingPattern: null,
    scanIntervalMinutes: 60,
    watchForChanges: false,
    scanning: false,
    lastScannedAt: null,
    qualityProfileId: null,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    movies: { __typename: "MovieConnection", edges: [], pageInfo: { __typename: "PageInfo", totalCount: 0 } },
    shows: { __typename: "ShowConnection", edges: [], pageInfo: { __typename: "PageInfo", totalCount: 0 } },
    albums: { __typename: "AlbumConnection", edges: [], pageInfo: { __typename: "PageInfo", totalCount: 0 } },
    audiobooks: { __typename: "AudiobookConnection", edges: [], pageInfo: { __typename: "PageInfo", totalCount: 0 } },
    mediaFiles: { __typename: "MediaFileConnection", pageInfo: { __typename: "PageInfo", totalCount: 0 } },
  },
});

const librariesMock = (edges: unknown[]) => ({
  request: { query: LibrariesOverviewDocument, variables: {} },
  result: { data: { libraries: { __typename: "LibraryConnection", edges } } },
});

const nowPlaying = [release(1, "Dune Part Three", "2026-09-01"), release(2, "The Batman 2", "2026-09-05")];

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  vi.setSystemTime(new Date("2026-09-07T12:00:00Z"));
});

afterEach(() => vi.useRealTimers());

const mount = (mocks: unknown[]) => renderWithProviders(<MovieReleases />, { mocks: mocks as never, routes: ["/movies/$movieId", "/search"] });

/** Card titles in row order. Each card is a direct child of the row and leads with its title. */
const titlesIn = (list: HTMLElement) => Array.from(list.children).map((card) => card.querySelector("p")?.textContent);
/** The badge is the first span inside a card's artwork frame. */
const badgesIn = (list: HTMLElement) => Array.from(list.children).map((card) => card.querySelector("span")?.textContent);

describe("MovieReleases", () => {
  it("puts the newest release first in the cinema row", async () => {
    await mount([
      guideMock("NOW_PLAYING", nowPlaying),
      guideMock("UPCOMING", []),
      ownedMock([2, 1], []),
      librariesMock([library("lib-1", "Films", "movies")]),
    ]);
    const cinema = await screen.findByRole("list", { name: "Recently released movies" });
    expect(titlesIn(cinema)).toEqual(["The Batman 2", "Dune Part Three"]);
  });

  it("orders upcoming releases by date", async () => {
    await mount([
      guideMock("NOW_PLAYING", []),
      guideMock("UPCOMING", [release(11, "Far Off", "2026-12-01"), release(13, "Tomorrow", "2026-09-08"), release(14, "Today", "2026-09-07")]),
      ownedMock([11, 13, 14], []),
      librariesMock([library("lib-1", "Films", "movies")]),
    ]);
    const upcoming = await screen.findByRole("list", { name: "Upcoming movie releases" });
    expect(titlesIn(upcoming)).toEqual(["Today", "Tomorrow", "Far Off"]);
  });

  /*
   * BUG: re-releases are meant to come last. The ranking prefixes their date with "~" and sorts
   * with `localeCompare`, which orders punctuation before digits, so they land first instead.
   * A plain `<` comparison (or a numeric rank) would do what the comment intends.
   */
  it("currently sorts re-releases to the front of the upcoming row", async () => {
    await mount([
      guideMock("NOW_PLAYING", []),
      guideMock("UPCOMING", [release(11, "Far Off", "2026-12-01"), release(12, "Re-run", "2001-05-04"), release(14, "Today", "2026-09-07")]),
      ownedMock([11, 12, 14], []),
      librariesMock([library("lib-1", "Films", "movies")]),
    ]);
    const upcoming = await screen.findByRole("list", { name: "Upcoming movie releases" });
    expect(titlesIn(upcoming)).toEqual(["Re-run", "Today", "Far Off"]);
  });

  it("badges each upcoming release with how far away it is", async () => {
    await mount([
      guideMock("NOW_PLAYING", []),
      guideMock("UPCOMING", [release(14, "Today", "2026-09-07"), release(13, "Tomorrow", "2026-09-08"), release(15, "Later", "2026-09-14"), release(12, "Re-run", "2001-05-04")]),
      ownedMock([12, 14, 13, 15], []),
      librariesMock([library("lib-1", "Films", "movies")]),
    ]);
    const upcoming = await screen.findByRole("list", { name: "Upcoming movie releases" });
    expect(badgesIn(upcoming)).toEqual(["Re-release", "Today", "Tomorrow", "In 7 days"]);
  });

  it("drops an upcoming entry that is already in the cinema row", async () => {
    await mount([
      guideMock("NOW_PLAYING", [release(1, "Dune Part Three", "2026-09-01")]),
      guideMock("UPCOMING", [release(1, "Dune Part Three", "2026-09-01"), release(2, "Something Else", "2026-10-01")]),
      ownedMock([1, 2], []),
      librariesMock([library("lib-1", "Films", "movies")]),
    ]);
    const upcoming = await screen.findByRole("list", { name: "Upcoming movie releases" });
    expect(titlesIn(upcoming)).toEqual(["Something Else"]);
  });

  it("links titles already in a library and labels their state", async () => {
    await mount([
      guideMock("NOW_PLAYING", nowPlaying),
      guideMock("UPCOMING", []),
      ownedMock([2, 1], [
        { id: "mv-1", tmdbId: 1, hasFile: true, wanted: false },
        { id: "mv-2", tmdbId: 2, hasFile: false, wanted: true },
      ]),
      librariesMock([library("lib-1", "Films", "movies")]),
    ]);
    const downloaded = await screen.findByRole("link", { name: /Dune Part Three/ });
    expect(downloaded).toHaveAttribute("href", "/movies/mv-1");
    expect(downloaded).toHaveTextContent("Downloaded");
    expect(screen.getByRole("link", { name: /The Batman 2/ })).toHaveTextContent("Wanted");
    // Nothing to add: an owned title has no add button.
    expect(screen.queryByRole("button", { name: /Add .* to library/ })).toBeNull();
  });

  it("labels a monitored-but-not-wanted title as in the library", async () => {
    await mount([
      guideMock("NOW_PLAYING", [release(1, "Dune Part Three", "2026-09-01")]),
      guideMock("UPCOMING", []),
      ownedMock([1], [{ id: "mv-1", tmdbId: 1, hasFile: false, wanted: false }]),
      librariesMock([library("lib-1", "Films", "movies")]),
    ]);
    expect(await screen.findByRole("link", { name: /Dune Part Three/ })).toHaveTextContent("In library");
  });

  it("adds a title to the only movie library", async () => {
    const addMovie = vi.fn(() => ({ data: { addMovie: { __typename: "AddMovieResult", success: true, error: null, movie: null } } }));
    await mount([
      guideMock("NOW_PLAYING", [release(1, "Dune Part Three", "2026-09-01")]),
      guideMock("UPCOMING", []),
      ownedMock([1], []),
      librariesMock([library("lib-1", "Films", "movies"), library("lib-2", "Concerts", "music")]),
      {
        request: { query: AddMovieDocument, variables: { libraryId: "lib-1", input: { tmdbId: 1, monitored: true } } },
        result: addMovie,
      },
    ]);
    await userEvent.click(await screen.findByRole("button", { name: "Add Dune Part Three to library" }));
    await vi.waitFor(() => expect(addMovie).toHaveBeenCalled());
    // A single movie library needs no picker; the row says where the film is playing instead.
    expect(screen.queryByRole("button", { name: "Add to library" })).toBeNull();
    expect(screen.getByText("In cinemas · US")).toBeInTheDocument();
  });

  it("offers a library picker when more than one movie library exists", async () => {
    await mount([
      guideMock("NOW_PLAYING", [release(1, "Dune Part Three", "2026-09-01")]),
      guideMock("UPCOMING", []),
      ownedMock([1], []),
      librariesMock([library("lib-1", "Films", "movies"), library("lib-3", "Kids films", "movies")]),
    ]);
    expect(await screen.findByRole("button", { name: /Add to library/ })).toBeInTheDocument();
  });

  it("shows an inline error with a retry when TMDB fails", async () => {
    await mount([
      guideError("NOW_PLAYING"),
      guideMock("UPCOMING", []),
      librariesMock([library("lib-1", "Films", "movies")]),
    ]);
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Could not load recently released movies");
    expect(alert).toHaveTextContent("TMDB is unreachable");
    expect(within(alert).getByRole("button", { name: /Try again/ })).toBeInTheDocument();
  });

  it("explains an empty region", async () => {
    await mount([guideMock("NOW_PLAYING", []), guideMock("UPCOMING", []), librariesMock([])]);
    expect(await screen.findByText("No current releases")).toBeInTheDocument();
    expect(await screen.findByText("Nothing scheduled")).toBeInTheDocument();
  });
});
