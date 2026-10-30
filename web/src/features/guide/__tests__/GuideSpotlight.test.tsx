import { screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { GuideEpisodesDocument, ScheduleCountriesDocument, ScheduleWeekDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { GuideSpotlight } from "../GuideSpotlight";

const TODAY = "2026-10-30";
const HORIZON = "2027-12-04";

const show = (id: string, name: string, network: string) => ({ __typename: "Show" as const, id, name, network, posterUrl: `/p/${id}.jpg`, backdropUrl: `/b/${id}.jpg`, autoDownload: true });
const percy = show("percy", "Percy Jackson and the Olympians", "Disney+");
const silo = show("silo", "Silo", "Apple TV");

const episode = (id: string, owner: ReturnType<typeof show>, season: number, number: number, airDate: string, title = `Episode ${number}`) => ({
  __typename: "EpisodeEdge" as const,
  node: { __typename: "Episode" as const, id, showId: owner.id, season, episode: number, title, airDate, airStamp: `${airDate}T12:00:00+00:00`, mediaFileId: null, show: owner },
});

const listing = (id: number, name: string, season: number, number: number, genres = ["Drama"]) => ({
  __typename: "ScheduleCacheEdge" as const,
  node: {
    __typename: "ScheduleCache" as const, id: `row-${id}`, tvmazeEpisodeId: id, episodeName: "Episode", season, episodeNumber: number, episodeType: "regular",
    airDate: TODAY, airTime: null, airStamp: null, runtime: 50, episodeImageUrl: null, summary: null, tvmazeShowId: id, showName: name,
    showNetwork: "Apple TV", showPosterUrl: null, showGenres: genres, countryCode: "US", createdAt: "", updatedAt: "",
  },
});

const mocks = (episodes: unknown[], listings: unknown[]) => [
  { request: { query: GuideEpisodesDocument, variables: { from: TODAY, to: HORIZON } }, result: { data: { episodes: { __typename: "EpisodeConnection", edges: episodes } } } },
  {
    request: { query: ScheduleCountriesDocument, variables: {} },
    result: { data: { scheduleSyncStates: { __typename: "ScheduleSyncStateConnection", edges: [{ __typename: "ScheduleSyncStateEdge", node: { __typename: "ScheduleSyncState", id: "us", countryCode: "US", lastSyncedAt: null, syncError: null } }] } } },
  },
  {
    request: { query: ScheduleWeekDocument, variables: { from: TODAY, to: TODAY, country: "US", offset: 0 } },
    result: { data: { scheduleCaches: { __typename: "ScheduleCacheConnection", edges: listings, pageInfo: { __typename: "PageInfo", totalCount: listings.length } } } },
  },
];

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  vi.setSystemTime(new Date(`${TODAY}T12:00:00`));
});
afterEach(() => vi.useRealTimers());

const mount = (episodes: unknown[], listings: unknown[] = []) =>
  renderWithProviders(<GuideSpotlight />, { mocks: mocks(episodes, listings) as never, routes: ["/shows/$showId", "/search", "/guide"] });

describe("GuideSpotlight", () => {
  it("spotlights the next show with a countdown and its release schedule", async () => {
    await mount([
      episode("p1", percy, 3, 1, "2026-11-20", "My Rescue Operation Goes Very Wrong"),
      episode("p2", percy, 3, 2, "2026-11-20", "An Old Dead Friend Comes to Visit"),
      episode("p3", percy, 3, 3, "2026-11-27", "TBA"),
      episode("s1", silo, 4, 1, "2027-07-09"),
    ]);
    expect(await screen.findByRole("heading", { name: percy.name })).toBeInTheDocument();
    expect(screen.getByText(/Season 3 premieres/)).toHaveTextContent("2 episodes weekly");
    const countdown = screen.getByLabelText("Countdown");
    expect(countdown).toHaveTextContent("21");
    expect(countdown).toHaveTextContent("3episodes");
    const schedule = screen.getByRole("list", { name: /season 3 schedule/ });
    expect(within(schedule).getAllByRole("listitem")).toHaveLength(2);
    expect(schedule).toHaveTextContent("My Rescue Operation Goes Very Wrong");
    expect(schedule).toHaveTextContent("Title to be announced");
    expect(screen.getByText("Auto-download on")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /Silo · Season 4/ })).toHaveTextContent("in 8 months");
    expect(screen.getByRole("link", { name: /Full guide/ })).toHaveAttribute("href", "/guide");
  });

  it("lists tonight's new listings without noise or library shows", async () => {
    await mount([episode("s1", silo, 4, 1, "2027-07-09")], [
      listing(1, "Nocturne", 1, 1),
      listing(2, "Morning Joe", 2026, 215, []),
      listing(3, "Silo", 4, 1),
      listing(4, "Tulsa King", 4, 3),
    ]);
    const row = await screen.findByLabelText("New tonight");
    expect(row).toHaveTextContent("Nocturne");
    expect(row).toHaveTextContent("New series");
    expect(row).toHaveTextContent("Tulsa King");
    expect(row).not.toHaveTextContent("Morning Joe");
    expect(row).not.toHaveTextContent(/^Silo/);
  });

  it("says when no episodes are listed for your shows", async () => {
    await mount([]);
    expect(await screen.findByText("No upcoming episodes are listed for your shows yet.")).toBeInTheDocument();
  });
});
