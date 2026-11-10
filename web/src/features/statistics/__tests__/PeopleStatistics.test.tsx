import { screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { EntityUserListDocument, PlayStatisticsDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { PeopleStatistics, peopleRows } from "../PeopleStatistics";

vi.mock("@/lib/auth/useSession", () => ({
  useIsAdmin: () => true,
  useSession: () => ({ status: "authenticated", user: { id: "me", role: "admin" } }),
}));

const usage = (userId: string, name: string, watched: number) => ({ __typename: "PlayStatisticsUser", userId, name, seconds: watched, plays: 2, secondsWatched: watched, secondsListened: 0, completedPlays: 1, lastPlayedAt: "2026-11-09T11:00:00.000Z" });
const user = (id: string, displayName: string) => ({ __typename: "User", id, username: id, email: null, role: "member", displayName, avatarUrl: null, isActive: true, lastLoginAt: null, createdAt: "1700000000", updatedAt: "1700000000", phone: null, genreInterests: null, mustChangePassword: false, twoFactorEnabledAt: null, recoveryCodesRemaining: 0, passwordChangedAt: null });

// Tables measure themselves; give jsdom a laptop-sized box.
beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(() => ({ x: 0, y: 0, top: 0, left: 0, bottom: 800, right: 1200, width: 1200, height: 800, toJSON: () => ({}) }));
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 32 : 1200; });
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 48 : 800; });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1200);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(800);
});
afterEach(() => { vi.restoreAllMocks(); });

describe("Settings › Statistics › People", () => {
  it("joins each person's time with their account", () => {
    const rows = peopleRows([usage("amy", "Amy", 3600), usage("gone", "Removed user", 60)] as never, [user("amy", "Amy") as never]);
    expect(rows[0]!.account?.displayName).toBe("Amy");
    expect(rows[1]!.account).toBeNull();
  });

  it("lists everyone who played, with their time", async () => {
    const stats = {
      __typename: "PlayStatistics", userId: null, userName: null, from: "", to: "", complete: true,
      totals: { __typename: "PlayStatisticsTotals", secondsWatched: 7200, secondsListened: 0, plays: 2, completedPlays: 1, importedPlays: 0, items: 1, activeDays: 1 },
      streak: { __typename: "PlayStatisticsStreak", current: 1, longest: 1, longestEndedOn: null }, days: [], weeks: [], hours: [],
      topShows: [], topMovies: [], topArtists: [], topAlbums: [], topAudiobooks: [], topAuthors: [], topGenres: [], devices: [],
      users: [usage("amy", "Amy", 7200)], origins: [], recent: [],
    };
    await renderWithProviders(<PeopleStatistics period="30d" />, {
      mocks: [
        { request: { query: PlayStatisticsDocument, variables: () => true }, result: { data: { playStatistics: stats } } },
        { request: { query: EntityUserListDocument, variables: () => true }, result: { data: { users: { __typename: "UserConnection", edges: [{ __typename: "UserEdge", node: user("amy", "Amy") }], pageInfo: { __typename: "PageInfo", hasNextPage: false, hasPreviousPage: false, totalCount: 1 } } } } },
      ],
    });
    expect((await screen.findAllByText("2h")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("Amy").length).toBeGreaterThan(0);
  });
});
