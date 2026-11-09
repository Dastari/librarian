import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { RecommendedRelease } from "../RecommendedRelease";
import type { Recommendation } from "../recommendations";

const state = vi.hoisted(() => ({ admin: true, rows: [] as unknown[], loading: false, lookup: vi.fn(), grab: vi.fn(), refetch: vi.fn() }));
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
    useMutation: (document: unknown) => [document === docs.LookupRecommendationsDocument ? state.lookup : state.grab, { loading: false }],
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
});
afterEach(() => vi.restoreAllMocks());

describe("RecommendedRelease", () => {
  it("shows the best release with its size, seeders, coverage and reasons, and a Download button", async () => {
    state.rows = [row({})];
    await renderWithProviders(<RecommendedRelease target={{ showId: "show-a", season: 1 }} missing={10} />, { router: false });
    expect(await screen.findByText("Recommended")).toBeInTheDocument();
    expect(screen.getByText(/Fallout\.S01\.1080p\.WEB-DL\.x265-GRP/)).toBeInTheDocument();
    expect(screen.getByText(/18\.0 GB · 80 seeders · 1080p WEB-DL hevc · covers 8 of 10 missing · from IPTorrents/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Download Fallout\.S01/ })).toBeInTheDocument();
  });

  it("downloads through the recommendation, not through a link", async () => {
    state.rows = [row({})];
    await renderWithProviders(<RecommendedRelease target={{ showId: "show-a", season: 1 }} missing={10} />, { router: false });
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
});
