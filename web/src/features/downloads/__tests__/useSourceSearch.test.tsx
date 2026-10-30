import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SearchSourcesQuery, SearchSourcesQueryVariables } from "@/graphql/generated/graphql";
import { mergeSourcePages, useSourceSearch } from "../useSourceSearch";

const state = vi.hoisted(() => ({ epoch: "principal-a", client: { query: vi.fn() } }));
vi.mock("@apollo/client/react", () => ({ useApolloClient: () => state.client }));
vi.mock("@/lib/graphql/live/hooks", () => ({ useLiveEpoch: () => state.epoch }));
type Source = SearchSourcesQuery["searchSources"]["sources"][number];
type Release = Source["releases"][number];
const release = (stableId: string, title = stableId) => ({ stableId, title } as Release);
const source = (sourceId: string, releases: Release[], overrides: Partial<Source> = {}): Source => ({
  sourceId, sourceName: sourceId, releases, elapsedMs: 1, fromCache: false, error: null,
  pageInfo: { support: "SUPPORTED", state: "UNKNOWN", hasMore: null, nextCursor: "opaque", retryCursor: null, returnedCount: releases.length },
  ...overrides,
});
const response = (sources: Source[]) => ({ data: { searchSources: { sources, totalReleases: sources.reduce((count, source) => count + source.releases.length, 0), totalElapsedMs: 1, sourcesSearched: sources.length } } });
const variables: SearchSourcesQueryVariables = { input: { query: "recording", albumId: "album", limit: null } };
beforeEach(() => { state.epoch = "principal-a"; state.client.query.mockReset(); });

describe("source continuation snapshots", () => {
  it("merges by stable source/release identity without clearing unrelated sources", () => {
    const merged = mergeSourcePages([source("a", [release("a:1", "old")]), source("b", [release("b:1")])], [source("a", [release("a:1", "fresh"), release("a:2")])]);
    expect(merged[0]?.releases.map((release) => release.title)).toEqual(["fresh", "a:2"]);
    expect(merged[1]?.sourceId).toBe("b");
  });

  it("continues UNKNOWN pages with nextCursor and names only the sources being advanced", async () => {
    state.client.query.mockResolvedValueOnce(response([
      source("a", [release("a:1")]),
      source("b", [release("b:1")], { pageInfo: { support: "UNSUPPORTED", state: "UNKNOWN", hasMore: null, nextCursor: null, retryCursor: null, returnedCount: 1 } }),
    ])).mockResolvedValueOnce(response([source("a", [release("a:1", "fresh"), release("a:2")], { pageInfo: { support: "SUPPORTED", state: "EXHAUSTED", hasMore: false, nextCursor: null, retryCursor: null, returnedCount: 2 } })]));
    const { result } = renderHook(() => useSourceSearch());
    await act(async () => { await result.current.search({ variables }); });
    expect(result.current.hasMore).toBe(true);
    await act(async () => { await result.current.loadMore(); });
    expect(state.client.query.mock.calls[1]![0].variables).toEqual({ input: { ...variables.input, sourceIds: ["a"], continuations: [{ sourceId: "a", cursor: "opaque" }] } });
    expect(result.current.hasMore).toBe(false);
    expect(result.current.result.data?.searchSources.sources.flatMap((source) => source.releases)).toHaveLength(3);
  });

  it("retains rows on failure and retries exactly the same source page cursor", async () => {
    const pageInfo: Source["pageInfo"] = { support: "SUPPORTED", state: "ERROR", hasMore: null, nextCursor: null, retryCursor: "same-page", returnedCount: 0 };
    state.client.query.mockResolvedValueOnce(response([source("broken", [release("old")], { error: "temporarily offline", pageInfo })]))
      .mockRejectedValueOnce(new Error("transport offline"))
      .mockResolvedValueOnce(response([source("broken", [release("new")], { pageInfo: { ...pageInfo, state: "EXHAUSTED", hasMore: false, retryCursor: null } })]));
    const { result } = renderHook(() => useSourceSearch());
    await act(async () => { await result.current.search({ variables }); });
    await act(async () => { await result.current.retrySource("broken"); });
    expect(result.current.result.data?.searchSources.sources[0]?.releases[0]?.stableId).toBe("old");
    expect(result.current.result.moreError?.message).toBe("transport offline");
    await act(async () => { await result.current.retrySource("broken"); });
    expect(state.client.query.mock.calls[1]![0].variables).toEqual(state.client.query.mock.calls[2]![0].variables);
    expect(result.current.result.data?.searchSources.sources[0]?.releases).toHaveLength(2);
  });

  it("discards prior principal cursors and ignores its in-flight response", async () => {
    let resolve!: (value: ReturnType<typeof response>) => void;
    state.client.query.mockReturnValueOnce(new Promise<ReturnType<typeof response>>((done) => { resolve = done; }));
    const { result, rerender } = renderHook(() => useSourceSearch());
    let request!: Promise<void>;
    act(() => { request = result.current.search({ variables }); });
    state.epoch = "principal-b";
    rerender();
    await act(async () => { resolve(response([source("old-user", [release("private")])])); await request; });
    expect(result.current.result.data).toBeUndefined();
    expect(result.current.hasMore).toBe(false);
    await act(async () => { await result.current.retrySource("old-user"); });
    expect(state.client.query).toHaveBeenCalledTimes(1);
  });
});
