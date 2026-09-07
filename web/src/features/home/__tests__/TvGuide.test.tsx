import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { NextLibraryEpisodesDocument, ScheduleCountriesDocument, ScheduleWeekDocument, UpcomingLibraryEpisodesDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { TvGuide } from "../TvGuide";

const FROM = "2026-09-07";
const TO = "2026-09-13";

const show = (id: string, name: string, network: string | null) => ({
  __typename: "Show" as const,
  id,
  name,
  posterUrl: `https://cdn/${id}.jpg`,
  network,
  libraryId: "lib-1",
});

const libraryEpisode = (
  id: string,
  overrides: Partial<{ season: number; episode: number; title: string | null; airDate: string; airStamp: string | null; mediaFileId: string | null; show: ReturnType<typeof show> }>,
) => ({
  __typename: "EpisodeEdge" as const,
  node: {
    __typename: "Episode" as const,
    id,
    showId: "show-1",
    season: 2,
    episode: 5,
    title: "Welcome to the Rebellion",
    airDate: "2026-09-07",
    airStamp: "2026-09-07T20:00:00Z",
    runtime: 45,
    mediaFileId: null,
    wanted: true,
    show: show("show-1", "Andor", "Disney+"),
    ...overrides,
  },
});

const scheduleRow = (id: string, overrides: Partial<Record<string, unknown>> = {}) => ({
  __typename: "ScheduleCacheEdge" as const,
  node: {
    __typename: "ScheduleCache" as const,
    id,
    tvmazeEpisodeId: Number(id.replace(/\D/g, "")),
    episodeName: "Pilot",
    season: 1,
    episodeNumber: 1,
    episodeType: "regular",
    airDate: "2026-09-07",
    airTime: "21:00",
    airStamp: "2026-09-07T21:00:00Z",
    runtime: 30,
    episodeImageUrl: null,
    summary: null,
    tvmazeShowId: 10,
    showName: "The Bear",
    showNetwork: "FX",
    showPosterUrl: "https://cdn/bear.jpg",
    showGenres: ["Drama"],
    countryCode: "US",
    createdAt: "2026-09-01T00:00:00Z",
    updatedAt: "2026-09-01T00:00:00Z",
    ...overrides,
  },
});

const libraryMock = (edges: unknown[]) => ({
  request: { query: UpcomingLibraryEpisodesDocument, variables: { from: FROM, to: TO } },
  result: { data: { episodes: { __typename: "EpisodeConnection", edges } } },
});

const laterMock = (edges: unknown[]) => ({
  request: { query: NextLibraryEpisodesDocument, variables: { from: TO } },
  result: { data: { episodes: { __typename: "EpisodeConnection", edges } } },
});

const countriesMock = (codes: string[]) => ({
  request: { query: ScheduleCountriesDocument, variables: {} },
  result: {
    data: {
      scheduleSyncStates: {
        __typename: "ScheduleSyncStateConnection",
        edges: codes.map((code) => ({
          __typename: "ScheduleSyncStateEdge",
          node: { __typename: "ScheduleSyncState", id: `sync-${code}`, countryCode: code, lastSyncedAt: "2026-09-06T00:00:00Z", syncError: null },
        })),
      },
    },
  },
});

const weekMock = (country: string, edges: unknown[]) => ({
  request: { query: ScheduleWeekDocument, variables: { from: FROM, to: TO, country, offset: 0 } },
  result: {
    data: {
      scheduleCaches: { __typename: "ScheduleCacheConnection", edges, pageInfo: { __typename: "PageInfo", totalCount: edges.length } },
    },
  },
});

const laterEpisode = {
  __typename: "EpisodeEdge" as const,
  node: {
    __typename: "Episode" as const,
    id: "later-1",
    showId: "show-9",
    season: 3,
    episode: 1,
    title: "Cold Harbor",
    airDate: "2026-10-01",
    airStamp: "2026-10-01T20:00:00Z",
    wanted: true,
    show: { __typename: "Show" as const, id: "show-9", name: "Severance", posterUrl: null, network: "Apple TV+" },
  },
};

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  vi.setSystemTime(new Date("2026-09-07T12:00:00Z"));
});

afterEach(() => vi.useRealTimers());

const mount = (mocks: unknown[]) => renderWithProviders(<TvGuide />, { mocks: mocks as never, routes: ["/shows/$showId", "/search"] });

const dayTab = (name: RegExp) => screen.getByRole("tab", { name });

describe("TvGuide", () => {
  it("lays out a week and counts the airings on each day", async () => {
    await mount([
      libraryMock([
        libraryEpisode("ep-1", {}),
        libraryEpisode("ep-2", { episode: 6, airDate: "2026-09-08", airStamp: null, title: "Who Are You?" }),
        libraryEpisode("ep-3", { episode: 7, airDate: "2026-09-08", airStamp: "2026-09-08T20:00:00Z", title: "Welcome" }),
      ]),
      laterMock([]),
    ]);
    const strip = await screen.findByRole("tablist", { name: "Day" });
    const tabs = within(strip).getAllByRole("tab");
    expect(tabs).toHaveLength(7);
    expect(tabs[0]).toHaveTextContent("Today");
    expect(tabs[1]).toHaveTextContent("Tomorrow");
    expect(tabs[2]).toHaveTextContent("Wednesday");
    await vi.waitFor(() => expect(tabs[0]).toHaveTextContent("· 1"));
    expect(tabs[1]).toHaveTextContent("· 2");
    expect(tabs[6]).not.toHaveTextContent("·");
    expect(tabs[0]).toHaveAttribute("aria-selected", "true");
  });

  it("shows today's programmes with their air time and library state", async () => {
    await mount([
      libraryMock([libraryEpisode("ep-1", { mediaFileId: "file-1" }), libraryEpisode("ep-2", { episode: 6, airDate: "2026-09-08" })]),
      laterMock([]),
    ]);
    const item = await screen.findByRole("button", { name: /Andor/ });
    expect(item).toHaveTextContent("S02E05");
    expect(item).toHaveTextContent("Welcome to the Rebellion");
    expect(item).toHaveTextContent(/8:00\s?PM/);
    expect(item).toHaveTextContent("Disney+");
    expect(item.querySelector('[title="Downloaded"]')).toBeTruthy();
    // Only the selected day is listed.
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
  });

  it("marks an episode that is in the library but not downloaded", async () => {
    await mount([libraryMock([libraryEpisode("ep-1", {})]), laterMock([])]);
    const item = await screen.findByRole("button", { name: /Andor/ });
    expect(item.querySelector('[title="In your library"]')).toBeTruthy();
  });

  it("says when the air time is still to be announced", async () => {
    await mount([libraryMock([libraryEpisode("ep-1", { airStamp: null })]), laterMock([])]);
    expect(await screen.findByText(/Time to be announced/)).toBeInTheDocument();
  });

  it("switches days and reports an empty one", async () => {
    await mount([libraryMock([libraryEpisode("ep-1", {})]), laterMock([])]);
    await screen.findByRole("button", { name: /Andor/ });
    await userEvent.click(dayTab(/Tomorrow/));
    expect(screen.queryByRole("button", { name: /Andor/ })).toBeNull();
    expect(screen.getByText(/Nothing on tomorrow/)).toBeInTheDocument();
  });

  it("numbers a special without an episode number", async () => {
    await mount([libraryMock([libraryEpisode("ep-1", { episode: 0 })]), laterMock([])]);
    expect(await screen.findByText(/S02 Special/)).toBeInTheDocument();
  });

  it("lists what is further ahead", async () => {
    await mount([libraryMock([]), laterMock([laterEpisode])]);
    expect(await screen.findByText("Further ahead")).toBeInTheDocument();
    const link = screen.getByRole("link", { name: /Severance/ });
    expect(link).toHaveAttribute("href", "/shows/show-9");
    expect(link).toHaveTextContent("S03E01");
  });

  it("offers an empty state for a week with nothing in it", async () => {
    await mount([libraryMock([]), laterMock([])]);
    expect(await screen.findByText("Nothing from your shows airs this week")).toBeInTheDocument();
  });

  it("opens a library show and searches for one that is not in a library", async () => {
    const { router } = await mount([
      libraryMock([libraryEpisode("ep-1", {})]),
      laterMock([]),
      countriesMock(["US"]),
      weekMock("US", [scheduleRow("sc-1")]),
    ]);
    await userEvent.click(await screen.findByRole("button", { name: /Andor/ }));
    await vi.waitFor(() => expect(router!.state.location.pathname).toBe("/shows/show-1"));
  });
});

describe("TvGuide across every show", () => {
  const allShows = async () => {
    const result = await mount([
      libraryMock([libraryEpisode("ep-1", {})]),
      laterMock([]),
      countriesMock(["US", "GB"]),
      weekMock("US", [scheduleRow("sc-1"), scheduleRow("sc-2", { showName: "Andor", tvmazeEpisodeId: 2 })]),
      weekMock("GB", [scheduleRow("sc-3", { showName: "Doctor Who", showNetwork: "BBC One", tvmazeEpisodeId: 3, countryCode: "GB" })]),
    ]);
    await screen.findByRole("button", { name: /Andor/ });
    await userEvent.click(screen.getByRole("radio", { name: "All shows" }));
    return result;
  };

  it("adds the schedule cache and drops shows already in the library", async () => {
    await allShows();
    expect(await screen.findByRole("button", { name: /The Bear/ })).toBeInTheDocument();
    // "Andor" appears once: the library row, not the duplicate schedule row.
    expect(screen.getAllByRole("button", { name: /Andor/ })).toHaveLength(1);
    expect(screen.getByText(/Everything airing this week · US/)).toBeInTheDocument();
  });

  it("offers the synced countries and reloads the week when one is picked", async () => {
    await allShows();
    await screen.findByRole("button", { name: /The Bear/ });
    const picker = screen.getByRole("radiogroup", { name: "Country" });
    expect(within(picker).getAllByRole("radio").map((radio) => radio.textContent)).toEqual(["US", "GB"]);
    await userEvent.click(within(picker).getByRole("radio", { name: "GB" }));
    expect(await screen.findByRole("button", { name: /Doctor Who/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /The Bear/ })).toBeNull();
  });

  it("marks a discovered show as addable", async () => {
    await allShows();
    const bear = await screen.findByRole("button", { name: /The Bear/ });
    expect(bear.querySelector("svg.tabler-icon-plus")).toBeTruthy();
    expect(screen.getByRole("button", { name: /Andor/ }).querySelector("svg.tabler-icon-plus")).toBeNull();
  });
});
