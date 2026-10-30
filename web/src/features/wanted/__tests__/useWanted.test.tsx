import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { UpgradableFilesDocument, WantedListMoviesDocument } from "@/graphql/generated/graphql";

import { useWanted } from "../useWanted";

const state = vi.hoisted(() => ({
  loading: false,
  statusLoading: false,
  error: undefined as Error | undefined,
  revision: "old",
  principal: "user-1",
  refetch: vi.fn(),
}));
vi.mock("@/lib/graphql/live/hooks", () => ({ useLiveEpoch: () => "test-client" }));
vi.mock("@apollo/client/react", () => ({ useSubscription: vi.fn() }));
vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ status: "authenticated", user: { id: state.principal, role: "admin" } }) }));
vi.mock("@/hooks/useContentStatuses", () => ({
  useContentStatusState: (_type: string, ids: string[]) => ({ statuses: new Map(ids.map((id) => [id, "WANTED"])), loading: ids.length > 0 && state.statusLoading, error: undefined, refetch: state.refetch }),
}));
vi.mock("@/features/libraries/browser/useCompleteConnection", () => ({
  useCompleteConnection: (document: unknown, variables: { where?: { wanted?: { eq?: boolean }; id?: { inList: string[] } } }, _select: unknown, options: { skip?: boolean }) => {
    if (options.skip) return { rows: [], loading: false, complete: false, error: undefined, refetch: state.refetch };
    const loading = state.loading;
    const id = `${state.revision}-${variables.where?.wanted?.eq === false ? "missing" : "wanted"}`;
    const movie = { id, title: id, wanted: true, ignored: false, libraryId: "library-1", year: 2026, imdbId: null };
    const rows = document === WantedListMoviesDocument ? [movie] : document === UpgradableFilesDocument ? [{ id: "file-1", movieId: id, episodeId: null, trackId: null, chapterId: null }] : [];
    return { rows, loading, complete: !loading && !state.error, error: state.error, refetch: state.refetch };
  },
}));

beforeEach(() => {
  state.loading = false;
  state.statusLoading = false;
  state.error = undefined;
  state.revision = "old";
  state.principal = "user-1";
  state.refetch.mockClear();
});

describe("Wanted complete snapshots", () => {
  it("keeps the complete title rows until entity and status refresh batches both finish", () => {
    const { result, rerender } = renderHook(() => useWanted("wanted"));
    const previous = result.current.groups;
    expect(previous[0]?.title).toBe("old-wanted");
    state.loading = true;
    state.revision = "new";
    rerender();
    expect(result.current.groups).toBe(previous);
    expect(result.current.loading).toBe(false);
    expect(result.current.refreshing).toBe(true);
    state.loading = false;
    state.statusLoading = true;
    rerender();
    expect(result.current.groups).toBe(previous);
    state.statusLoading = false;
    rerender();
    expect(result.current.groups[0]?.title).toBe("new-wanted");
    expect(result.current.statuses.get("movie:new-wanted")).toBe("WANTED");
  });

  it("preserves readable rows and complete action targets after a failed refresh", () => {
    const { result, rerender } = renderHook(() => useWanted("upgradable"));
    const previous = result.current.groups;
    state.revision = "partial";
    state.error = new Error("Upgrade lookup failed");
    rerender();
    expect(result.current.groups).toBe(previous);
    expect(result.current.error?.message).toBe("Upgrade lookup failed");
    act(() => result.current.refetch());
    expect(state.refetch).toHaveBeenCalled();
    state.error = undefined;
    rerender();
    expect(result.current.groups[0]?.title).toBe("partial-wanted");
  });

  it("keeps a separate complete snapshot per tab and clears them on a principal change", () => {
    const { result, rerender } = renderHook(({ tab }) => useWanted(tab), { initialProps: { tab: "wanted" as "wanted" | "missing" } });
    const wanted = result.current.groups;
    rerender({ tab: "missing" });
    const missing = result.current.groups;
    expect(missing[0]?.title).toBe("old-missing");
    state.loading = true;
    rerender({ tab: "wanted" });
    expect(result.current.groups).toBe(wanted);
    rerender({ tab: "missing" });
    expect(result.current.groups).toBe(missing);
    state.principal = "user-2";
    rerender({ tab: "wanted" });
    expect(result.current.groups).toEqual([]);
    expect(result.current.loading).toBe(true);
  });
});
