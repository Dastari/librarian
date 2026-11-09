import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { RecommendedRelease } from "../RecommendedRelease";
import type { Recommendation } from "../recommendations";

const state = vi.hoisted(() => ({ admin: true, rows: [] as unknown[], loading: false, lookup: vi.fn(), grab: vi.fn(), grabPlan: vi.fn(), refetch: vi.fn() }));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => state.admin, useSession: () => ({ user: { id: "me", role: "admin" } }) }));
vi.mock("@/lib/graphql/live/hooks", () => ({
  useLiveQuery: (_document: unknown, options: { skip?: boolean }) => ({
    data: options.skip ? undefined : { releaseRecommendations: { edges: state.rows.map((node) => ({ node })) } },
    loading: state.loading,
    refetch: state.refetch,
  }),
}));
vi.mock("@apollo/client/react", async (original) => {
  const docs = await import("@/graphql/generated/graphql");
  return {
    ...(await original<typeof import("@apollo/client/react")>()),
    useMutation: (document: unknown) => [document === docs.LookupRecommendationsDocument ? state.lookup : document === docs.GrabRecommendationPlanDocument ? state.grabPlan : state.grab, { loading: false }],
  };
});

const row = (extra: Partial<Recommendation>): Recommendation => ({
  id: "rec-1", scope: "season_pack", status: "ready", targetLabel: "Fallout S01", title: "Fallout.S01.1080p.WEB-DL.x265-GRP", sourceKind: "indexer", sourceName: "IPTorrents",
  size: 18 * 1024 ** 3, seeders: 80, quality: "1080p WEB-DL hevc", coversCount: 8, missingCount: 10, reasons: ["1080p WEB-DL hevc fits the 'HD' profile", "Smallest at that quality (18.0 GB)"], note: null, episodeId: null, ...extra,
}) as Recommendation;

beforeEach(() => {
  vi.clearAllMocks();
  state.admin = true;
  state.rows = [];
  state.loading = false;
  state.lookup.mockResolvedValue({ data: { lookupRecommendations: { success: true, state: "queued" } } });
  state.grab.mockResolvedValue({ data: { grabRecommendation: { success: true, torrentName: "Fallout S01" } } });
  state.grabPlan.mockResolvedValue({ data: { grabRecommendationPlan: { success: true, error: null, started: 3, failed: [], torrentIds: ["t1", "t2", "t3"] } } });
});
afterEach(() => vi.restoreAllMocks());

describe("RecommendedRelease", () => {
  it("shows the best release with its size, seeders, coverage and reasons, and a Download button", async () => {
    state.rows = [row({ scope: "album", albumId: "album-x" })];
    await renderWithProviders(<RecommendedRelease target={{ albumId: "album-x" }} missing={10} />, { router: false });
    expect(await screen.findByText("Recommended")).toBeInTheDocument();
    expect(screen.getByText(/Fallout\.S01\.1080p\.WEB-DL\.x265-GRP/)).toBeInTheDocument();
    expect(screen.getByText(/18\.0 GB · 80 seeders · 1080p WEB-DL hevc · covers 8 of 10 missing · from IPTorrents/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Download Fallout\.S01/ })).toBeInTheDocument();
  });

  it("downloads through the recommendation, not through a link", async () => {
    state.rows = [row({ scope: "album", albumId: "album-y" })];
    await renderWithProviders(<RecommendedRelease target={{ albumId: "album-y" }} missing={10} />, { router: false });
    await userEvent.click(await screen.findByRole("button", { name: /Download Fallout\.S01/ }));
    await waitFor(() => expect(state.grab).toHaveBeenCalledWith({ variables: { id: "rec-1" } }));
    expect(await screen.findByText("Downloading Fallout S01")).toBeInTheDocument();
  });

  it("asks for a lookup once when the page has something missing and nothing is stored", async () => {
    await renderWithProviders(<RecommendedRelease target={{ movieId: "movie-ask" }} missing={1} />, { router: false });
    await waitFor(() => expect(state.lookup).toHaveBeenCalledTimes(1));
    expect(state.lookup).toHaveBeenCalledWith({ variables: { input: { movieId: "movie-ask" } } });
  });

  it("does not ask when nothing is missing, or for anyone but an administrator", async () => {
    await renderWithProviders(<RecommendedRelease target={{ movieId: "movie-none" }} missing={0} />, { router: false });
    state.admin = false;
    await renderWithProviders(<RecommendedRelease target={{ movieId: "movie-member" }} missing={1} />, { router: false });
    expect(state.lookup).not.toHaveBeenCalled();
    expect(screen.queryByText("Recommended")).not.toBeInTheDocument();
  });

  it("says it is looking while a lookup is queued, and says nothing quietly when none was found", async () => {
    state.rows = [row({ status: "searching", title: null })];
    const view = await renderWithProviders(<RecommendedRelease target={{ albumId: "album-a" }} missing={3} />, { router: false });
    expect(await screen.findByText("Looking for the best release…")).toBeInTheDocument();
    view.unmount();
    state.rows = [row({ status: "none", title: null, note: "no release found on 1 source(s)" })];
    await renderWithProviders(<RecommendedRelease target={{ albumId: "album-b" }} missing={3} quiet />, { router: false });
    expect(screen.queryByText(/No suitable release/)).not.toBeInTheDocument();
  });

  describe("a show's coverage plan", () => {
    const plan = row({ id: "plan-1", scope: "plan", targetLabel: "Babylon 5", title: null, size: null, seeders: null, quality: null, coversCount: 24, missingCount: 44, note: "Still missing: S02E03–E22 (no usable release found)", reasons: ["1 pack and 2 single episodes cover 24 of 44 missing at the best quality found"], planId: null });
    const pack = row({ id: "m-1", planId: "plan-1", planPosition: 0, scope: "season_pack", season: 1, coversSeasons: "[1]", targetLabel: "Babylon 5 Season 1", title: "Babylon.5.S01.1080p.BluRay.x264-GRP", coversCount: 22, missingCount: 44 });
    const first = row({ id: "m-2", planId: "plan-1", planPosition: 1, scope: "episode", season: 2, coversSeasons: "[2]", targetLabel: "Babylon 5 S02E01", title: "Babylon.5.S02E01.1080p.WEB.h264-GRP", size: 1024 ** 3, coversCount: 1, missingCount: 44 });
    const second = row({ id: "m-3", planId: "plan-1", planPosition: 2, scope: "episode", season: 2, coversSeasons: "[2]", targetLabel: "Babylon 5 S02E02", title: "Babylon.5.S02E02.1080p.WEB.h264-GRP", size: 1024 ** 3, coversCount: 1, missingCount: 44 });

    it("says how many releases cover how much, lists them in order, and says what is still missing", async () => {
      state.rows = [second, plan, first, pack];
      await renderWithProviders(<RecommendedRelease target={{ showId: "show-b5", series: true }} missing={44} />, { router: false });
      expect(await screen.findByText(/3 releases cover 24 of 44 missing/)).toBeInTheDocument();
      const titles = screen.getAllByText(/Babylon\.5\.S0/).map((node) => node.textContent ?? "");
      expect(titles[0]).toContain("Babylon.5.S01.1080p");
      expect(titles[2]).toContain("S02E02");
      expect(screen.getByText(/Season 1 ·/)).toBeInTheDocument();
      expect(screen.getByText(/adds 22/)).toBeInTheDocument();
      expect(screen.getByText("Still missing: S02E03–E22 (no usable release found)")).toBeInTheDocument();
      expect(screen.getAllByRole("button", { name: /^Download Babylon/ })).toHaveLength(3);
    });

    it("downloads the whole plan after asking, or any one release in it", async () => {
      state.rows = [plan, pack, first, second];
      await renderWithProviders(<RecommendedRelease target={{ showId: "show-b5-all", series: true }} missing={44} />, { router: false });
      await userEvent.click(await screen.findByRole("button", { name: "Download all 3 recommended releases" }));
      await userEvent.click(await screen.findByRole("button", { name: "Download all" }));
      await waitFor(() => expect(state.grabPlan).toHaveBeenCalledWith({ variables: { id: "plan-1" } }));
      expect(await screen.findByText("Downloading 3 releases")).toBeInTheDocument();
      await userEvent.click(screen.getByRole("button", { name: /Download Babylon\.5\.S02E01/ }));
      await waitFor(() => expect(state.grab).toHaveBeenCalledWith({ variables: { id: "m-2" } }));
    });

    it("shows a season only its part of the plan, and looks the whole show up once", async () => {
      state.rows = [plan, pack, first, second];
      await renderWithProviders(<RecommendedRelease target={{ showId: "show-b5-season", season: 2 }} missing={22} />, { router: false });
      expect(await screen.findByText(/S02E01\.1080p/)).toBeInTheDocument();
      expect(screen.queryByText(/S01\.1080p/)).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /Download all/ })).not.toBeInTheDocument();
      expect(state.lookup).not.toHaveBeenCalled();

      state.rows = [];
      await renderWithProviders(<RecommendedRelease target={{ showId: "show-new", season: 3 }} missing={5} />, { router: false });
      await waitFor(() => expect(state.lookup).toHaveBeenCalledWith({ variables: { input: { showId: "show-new" } } }));
    });
  });
});
