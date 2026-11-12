import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { PlayStatisticsDocument, StatisticsPeopleDocument, StatisticsSharingDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";
import { layoutColumn, RemoteNavigation, remoteStopElements, remoteStops, stopLabel, walkRemote } from "@/test/remote";

import { StatisticsPage } from "../StatisticsPage";

const auth = vi.hoisted(() => ({ admin: false }));
vi.mock("@/lib/auth/useSession", () => ({
  useIsAdmin: () => auth.admin,
  useSession: () => ({ status: "authenticated", user: { id: "me", role: auth.admin ? "admin" : "member" } }),
}));

const bucket = (date: string, watched: number, listened: number, plays: number) => ({ __typename: "PlayStatisticsBucket", date, secondsWatched: watched, secondsListened: listened, plays });
const top = (kind: string, title: string, seconds: number, plays: number) => ({ __typename: "PlayStatisticsTopItem", key: title, kind, entityId: null, title, subtitle: null, seconds, plays });

function stats(extra: Record<string, unknown> = {}) {
  return {
    __typename: "PlayStatistics",
    userId: "me", userName: "Toby", from: "2026-10-11T00:00:00.000Z", to: "2026-11-09T12:00:00.000Z", complete: true,
    totals: { __typename: "PlayStatisticsTotals", secondsWatched: 9000, secondsListened: 1800, plays: 5, completedPlays: 3, importedPlays: 1, items: 4, activeDays: 2 },
    streak: { __typename: "PlayStatisticsStreak", current: 2, longest: 3, longestEndedOn: "2026-11-01" },
    days: [bucket("2026-11-08", 5400, 0, 2), bucket("2026-11-09", 3600, 1800, 3)],
    weeks: [bucket("2026-11-02", 9000, 1800, 5)],
    hours: Array.from({ length: 24 }, (_, hour) => (hour === 20 ? 9000 : 0)),
    topShows: [top("show", "Severance", 9000, 3)],
    topMovies: [top("movie", "Arrival", 0, 1)],
    topArtists: [top("artist", "Radiohead", 1800, 1)], topAlbums: [], topAudiobooks: [], topAuthors: [], topGenres: [top("genre", "Drama", 9000, 3)],
    devices: [{ __typename: "PlayStatisticsDevice", deviceId: "d1", name: "Living room TV", seconds: 9000, plays: 3, lastPlayedAt: "2026-11-09T11:00:00.000Z" }],
    users: [],
    origins: [{ __typename: "PlayStatisticsOrigin", origin: null, name: "This server", seconds: 10800, plays: 5 }],
    recent: [{ __typename: "PlayStatisticsPlay", id: "p1", contentType: "episode", title: "Woe's Hollow", parentTitle: "Severance", subtitle: "S2 · E4", movieId: null, episodeId: "e1", showId: "s1", trackId: null, albumId: null, artistId: null, audiobookId: null, startedAt: "2026-11-09T10:00:00.000Z", endedAt: "2026-11-09T11:00:00.000Z", secondsPlayed: 3000, completed: true, imported: false, deviceName: "Living room TV", playMethod: "direct" }],
    ...extra,
  };
}

const people = (list: Array<{ userId: string; name: string; isSelf: boolean; shared: boolean }>) => ({
  request: { query: StatisticsPeopleDocument },
  result: { data: { statisticsPeople: list.map((entry) => ({ __typename: "StatisticsPerson", ...entry })) } },
});
const statsMock = (data: unknown) => ({ request: { query: PlayStatisticsDocument, variables: () => true }, result: { data: { playStatistics: data } } });

describe("Statistics", () => {
  beforeEach(() => {
    auth.admin = false;
  });

  it("shows totals, the timeline, top lists and recent plays", async () => {
    await renderWithProviders(<StatisticsPage period="30d" onSearchChange={() => {}} />, { mocks: [people([{ userId: "me", name: "Toby", isSelf: true, shared: false }]), statsMock(stats())] });
    expect(await screen.findByText("Time per day")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Top shows" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Top genres" })).toBeInTheDocument();
    expect(screen.getByText(/4 different titles · 3 finished · active on 2 days · longest streak 3 days · 1 imported/)).toBeInTheDocument();
    expect(screen.getByText("Severance: Woe's Hollow")).toBeInTheDocument();
    // Your own statistics can be shared; nobody else's can.
    expect(screen.getByRole("button", { name: /Share/ })).toBeInTheDocument();
  });

  it("says when nothing was played", async () => {
    const empty = stats({ totals: { __typename: "PlayStatisticsTotals", secondsWatched: 0, secondsListened: 0, plays: 0, completedPlays: 0, importedPlays: 0, items: 0, activeDays: 0 } });
    await renderWithProviders(<StatisticsPage period="7d" onSearchChange={() => {}} />, { mocks: [people([]), statsMock(empty)] });
    expect(await screen.findByText("Nothing played in this period")).toBeInTheDocument();
  });

  it("opens someone's shared statistics read-only", async () => {
    await renderWithProviders(<StatisticsPage period="30d" person="amy" onSearchChange={() => {}} />, {
      mocks: [people([{ userId: "me", name: "Toby", isSelf: true, shared: false }, { userId: "amy", name: "Amy", isSelf: false, shared: true }]), statsMock(stats({ userId: "amy", userName: "Amy" }))],
    });
    expect(await screen.findByText("Amy shares these with you. They are read-only.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Share/ })).not.toBeInTheDocument();
  });

  it("shares with chosen people", async () => {
    const sharing = {
      request: { query: StatisticsSharingDocument, variables: { userId: "me" } },
      result: { data: { statisticsShareCandidates: [{ __typename: "StatisticsPerson", userId: "amy", name: "Amy", isSelf: false, shared: false }], statsShares: { __typename: "StatsShareConnection", edges: [] } } },
    };
    await renderWithProviders(<StatisticsPage period="30d" onSearchChange={() => {}} />, { mocks: [people([{ userId: "me", name: "Toby", isSelf: true, shared: false }]), statsMock(stats()), sharing] });
    await userEvent.click(await screen.findByRole("button", { name: /Share/ }));
    expect(await screen.findByText("Share your statistics")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Some people" }));
    expect(await screen.findByRole("checkbox", { name: "Amy" })).toBeInTheDocument();
  });
});

describe("Server statistics (Settings › Statistics)", () => {
  beforeEach(() => {
    auth.admin = true;
  });
  const everyonePeople = people([{ userId: "me", name: "Toby", isSelf: true, shared: false }, { userId: "amy", name: "Amy", isSelf: false, shared: false }]);

  it("shows everyone by default, per person, and no origin choice with one server", async () => {
    const data = stats({ userId: null, userName: null, users: [{ __typename: "PlayStatisticsUser", userId: "amy", name: "Amy", seconds: 9000, plays: 3, secondsWatched: 9000, secondsListened: 0, completedPlays: 2, lastPlayedAt: "2026-11-09T11:00:00.000Z" }] });
    await renderWithProviders(<StatisticsPage scope="server" period="30d" onSearchChange={() => {}} />, { mocks: [everyonePeople, statsMock(data)] });
    expect(await screen.findByRole("region", { name: "People" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Share/ })).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Where played")).not.toBeInTheDocument();
  });

  it("offers an origin choice once plays come from another server", async () => {
    const data = stats({ userId: null, userName: null, origins: [
      { __typename: "PlayStatisticsOrigin", origin: null, name: "This server", seconds: 9000, plays: 4 },
      { __typename: "PlayStatisticsOrigin", origin: "srv-2", name: "Cabin NAS", seconds: 1800, plays: 1 },
    ] });
    await renderWithProviders(<StatisticsPage scope="server" period="30d" onSearchChange={() => {}} />, { mocks: [everyonePeople, statsMock(data)] });
    expect(await screen.findByLabelText("Where played")).toBeInTheDocument();
  });

  it("lists only you and people sharing with you in your own profile, even for an administrator", async () => {
    await renderWithProviders(<StatisticsPage period="30d" onSearchChange={() => {}} />, { mocks: [everyonePeople, statsMock(stats())] });
    await screen.findByText("Time per day");
    // Amy does not share with Toby: there is no one to choose between.
    expect(screen.queryByRole("combobox", { name: "Whose statistics" })).not.toBeInTheDocument();
  });
});

describe("Statistics on a TV remote", () => {
  it("stops only on its controls: the period, then Share; charts, totals and lists are for reading", async () => {
    window.localStorage.setItem("librarian.inputMode", "tv");
    await renderWithProviders(
      <><RemoteNavigation /><main id="main"><StatisticsPage period="30d" onSearchChange={() => {}} /></main></>,
      { shell: true, mocks: [people([{ userId: "me", name: "Toby", isSelf: true, shared: false }]), statsMock(stats())] },
    );
    await screen.findByText("Time per day");
    layoutColumn(document.body);
    expect(remoteStops(document.body)).toEqual(["7 days", "30 days", "90 days", "Year", "Share"]);
    const period = remoteStopElements(document.body).find((stop) => stopLabel(stop) === "30 days")!;
    expect(walkRemote(period)).toEqual(["30 days", "Share"]);
  });
});
