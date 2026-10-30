import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ShowEpisodesDocument, type ShowEpisodesQuery } from "@/graphql/generated/graphql";

import { useShowEpisodes } from "../useShowEpisodes";
import { mediaHookHarness } from "./mediaHookHarness";

type Episode = ShowEpisodesQuery["episodes"]["edges"][number]["node"];
const episode = (number: number, showId = "show-a"): Episode => ({
  id: `${showId}-${number}`, showId, season: Math.ceil(number / 20), episode: number,
  title: `Episode ${number}`, absoluteNumber: null, overview: null, airDate: null, airStamp: null,
  runtime: null, stillUrl: null, tvmazeId: null, tmdbId: null, tvdbId: null, wanted: false, ignored: false,
  mediaFileId: null, mediaFile: null, createdAt: "2026-01-01", updatedAt: "2026-01-01",
});
const page = (rows: Episode[], hasNextPage = false, totalCount = rows.length) => ({
  episodes: { __typename: "EpisodeConnection", edges: rows.map((node) => ({ __typename: "EpisodeEdge", node: { __typename: "Episode", ...node } })), pageInfo: { __typename: "PageInfo", hasNextPage, totalCount } },
});
const rows = (count: number, start = 1) => Array.from({ length: count }, (_, i) => episode(start + i));

describe("useShowEpisodes", () => {
  it("retains cached later seasons under the production cache-and-network defaults", async () => {
    const { client, requests, wrapper } = mediaHookHarness();
    client.defaultOptions.watchQuery = { fetchPolicy: "cache-and-network" };
    client.writeQuery({ query: ShowEpisodesDocument, variables: { showId: "show-a" }, data: page(rows(101)) });
    const { result } = renderHook(() => useShowEpisodes("show-a"), { wrapper });
    expect(result.current.rows).toHaveLength(101);
    await waitFor(() => expect(requests).toHaveLength(1));
    await act(async () => requests[0]!.resolve(page(rows(100), true, 101)));
    await waitFor(() => expect(requests).toHaveLength(2));
    expect(result.current.rows).toHaveLength(101);
    expect(result.current.rows.at(-1)?.id).toBe("show-a-101");
    await act(async () => requests[1]!.resolve(page([episode(101)], false, 101)));
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(result.current.rows).toHaveLength(101);
  });

  it("loads every page with the existing page operation and stops at the final page", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useShowEpisodes("show-a"), { wrapper });
    await waitFor(() => expect(requests).toHaveLength(1));
    expect(requests[0]!.operation.operationName).toBe("ShowEpisodes");
    await act(async () => requests[0]!.resolve(page(rows(100), true, 205)));
    await waitFor(() => expect(requests).toHaveLength(2));
    expect(requests[1]!.operation.operationName).toBe("ShowEpisodesPage");
    expect(requests[1]!.operation.variables).toEqual({ showId: "show-a", offset: 100 });
    expect(result.current.complete).toBe(false);
    await act(async () => requests[1]!.resolve(page(rows(100, 101), true, 205)));
    await waitFor(() => expect(requests).toHaveLength(3));
    expect(requests[2]!.operation.variables).toEqual({ showId: "show-a", offset: 200 });
    await act(async () => requests[2]!.resolve(page(rows(5, 201), false, 205)));
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(result.current.rows.map((row) => row.id)).toEqual(rows(205).map((row) => row.id));
    expect(result.current.loadingMore).toBe(false);
    expect(result.current.error).toBeUndefined();
    // Flush another effect turn: a terminal page must not restart paging.
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 20)); });
    expect(requests).toHaveLength(3);
  });

  it("does not request a continuation for a short complete show", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useShowEpisodes("show-a"), { wrapper });
    await act(async () => requests[0]!.resolve(page(rows(3))));
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(requests).toHaveLength(1);
    expect(result.current.rows).toHaveLength(3);
  });

  it("merges overlapping pages by episode ID without duplicating rows", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useShowEpisodes("show-a"), { wrapper });
    await act(async () => requests[0]!.resolve(page(rows(100), true, 102)));
    await waitFor(() => expect(requests).toHaveLength(2));
    await act(async () => requests[1]!.resolve(page([episode(100), episode(101), episode(101), episode(102)], false, 102)));
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(result.current.rows.map((row) => row.id)).toEqual(rows(102).map((row) => row.id));
  });

  it("surfaces a continuation failure, stops automatic retries and recovers on retry", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useShowEpisodes("show-a"), { wrapper });
    await act(async () => requests[0]!.resolve(page(rows(100), true, 101)));
    await waitFor(() => expect(requests).toHaveLength(2));
    await act(async () => requests[1]!.reject(new Error("Remaining episodes unavailable")));
    await waitFor(() => expect(result.current.error?.message).toBe("Remaining episodes unavailable"));
    expect(result.current.complete).toBe(false);
    expect(result.current.rows).toHaveLength(100);
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 20)); });
    expect(requests).toHaveLength(2);
    act(() => { void result.current.refetch().catch(() => {}); });
    await waitFor(() => expect(requests.slice(2).some(({ operation }) => operation.operationName === "ShowEpisodes")).toBe(true));
    const retryFirstPage = requests.slice(2).find(({ operation }) => operation.operationName === "ShowEpisodes")!;
    await act(async () => retryFirstPage.resolve(page(rows(100), true, 101)));
    await waitFor(() => expect(requests.slice(2).some(({ operation }) => operation.operationName === "ShowEpisodesPage")).toBe(true));
    const retryContinuation = requests.slice(2).find(({ operation }) => operation.operationName === "ShowEpisodesPage")!;
    await act(async () => retryContinuation.resolve(page([episode(101)])));
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(result.current.error).toBeUndefined();
    expect(result.current.rows).toHaveLength(101);
  });

  it("turns a non-advancing page into a recoverable error instead of an infinite query", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useShowEpisodes("show-a"), { wrapper });
    await act(async () => requests[0]!.resolve(page(rows(100), true, 101)));
    await waitFor(() => expect(requests).toHaveLength(2));
    await act(async () => requests[1]!.resolve(page(rows(100), true, 101)));
    await waitFor(() => expect(result.current.error?.message).toMatch(/did not advance/i));
    expect(result.current.complete).toBe(false);
    expect(requests).toHaveLength(2);
  });

  it("does not merge a late old-show page into the newly selected show", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result, rerender } = renderHook(({ showId }) => useShowEpisodes(showId), { wrapper, initialProps: { showId: "show-a" } });
    await act(async () => requests[0]!.resolve(page(rows(100), true, 101)));
    await waitFor(() => expect(requests).toHaveLength(2));
    rerender({ showId: "show-b" });
    expect(result.current.rows).toEqual([]);
    await waitFor(() => expect(requests).toHaveLength(3));
    await act(async () => requests[2]!.resolve(page([episode(1, "show-b")])));
    await act(async () => requests[1]!.resolve(page([episode(101)])));
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(result.current.rows.map((row) => row.id)).toEqual(["show-b-1"]);
    expect(result.current.error).toBeUndefined();
  });

  it("finishes the new show's paging when the old show's continuation settles late", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result, rerender } = renderHook(({ showId }) => useShowEpisodes(showId), { wrapper, initialProps: { showId: "show-a" } });
    await act(async () => requests[0]!.resolve(page(rows(100), true, 101)));
    await waitFor(() => expect(requests).toHaveLength(2));
    rerender({ showId: "show-b" });
    await waitFor(() => expect(requests).toHaveLength(3));
    const newRows = Array.from({ length: 100 }, (_, i) => episode(i + 1, "show-b"));
    await act(async () => requests[2]!.resolve(page(newRows, true, 101)));
    await act(async () => requests[1]!.resolve(page([episode(101)])));
    await waitFor(() => expect(requests.some(({ operation }) => operation.operationName === "ShowEpisodesPage" && operation.variables.showId === "show-b")).toBe(true));
    const continuation = requests.find(({ operation }) => operation.operationName === "ShowEpisodesPage" && operation.variables.showId === "show-b")!;
    expect(continuation.operation.variables.offset).toBe(100);
    await act(async () => continuation.resolve(page([episode(101, "show-b")])));
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(result.current.rows).toHaveLength(101);
    expect(result.current.rows.every((row) => row.showId === "show-b")).toBe(true);
  });

  it("retains the complete connection throughout refresh and removes deleted episodes only at completion", async () => {
    const { client, requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useShowEpisodes("show-a"), { wrapper });
    await act(async () => requests[0]!.resolve(page(rows(100), true, 103)));
    await waitFor(() => expect(requests).toHaveLength(2));
    await act(async () => requests[1]!.resolve(page(rows(3, 101), false, 103)));
    await waitFor(() => expect(result.current.complete).toBe(true));
    const originalIds = rows(103).map(({ id }) => id);
    const cachedIds = () => client.readQuery({ query: ShowEpisodesDocument, variables: { showId: "show-a" } })?.episodes.edges.map(({ node }) => node.id);

    let refresh!: ReturnType<typeof result.current.refetch>;
    act(() => { refresh = result.current.refetch(); });
    await waitFor(() => expect(requests).toHaveLength(3));
    expect(result.current.rows.map(({ id }) => id)).toEqual(originalIds);
    expect(cachedIds()).toEqual(originalIds);

    // Episode 102 was deleted, but later-season controls must survive until all
    // refreshed pages have arrived, including the still-present episode 103.
    await act(async () => requests[2]!.resolve(page(rows(100), true, 102)));
    await waitFor(() => expect(requests).toHaveLength(4));
    expect(requests[3]!.operation.variables).toEqual({ showId: "show-a", offset: 100 });
    expect(result.current.rows.map(({ id }) => id)).toEqual(originalIds);
    expect(cachedIds()).toEqual(originalIds);
    await act(async () => {
      requests[3]!.resolve(page([episode(101), episode(103)], false, 102));
      await refresh;
    });
    await waitFor(() => expect(result.current.complete).toBe(true));
    const refreshedIds = originalIds.filter((id) => id !== "show-a-102");
    expect(result.current.rows.map(({ id }) => id)).toEqual(refreshedIds);
    expect(cachedIds()).toEqual(refreshedIds);
    expect(result.current.error).toBeUndefined();
    expect(requests).toHaveLength(4);
  });

  it.each(["first page", "continuation"] as const)("retains all previously loaded IDs after a refresh %s failure", async (failureAt) => {
    const { client, requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useShowEpisodes("show-a"), { wrapper });
    await act(async () => requests[0]!.resolve(page(rows(100), true, 103)));
    await waitFor(() => expect(requests).toHaveLength(2));
    await act(async () => requests[1]!.resolve(page(rows(3, 101), false, 103)));
    await waitFor(() => expect(result.current.complete).toBe(true));
    const originalIds = rows(103).map(({ id }) => id);
    let refresh!: Promise<unknown>;
    act(() => { refresh = result.current.refetch().catch((error: unknown) => error); });
    await waitFor(() => expect(requests).toHaveLength(3));
    expect(result.current.rows.map(({ id }) => id)).toEqual(originalIds);
    if (failureAt === "continuation") {
      await act(async () => requests[2]!.resolve(page(rows(100), true, 102)));
      await waitFor(() => expect(requests).toHaveLength(4));
      expect(result.current.rows.map(({ id }) => id)).toEqual(originalIds);
    }
    const error = new Error(`Refresh ${failureAt} unavailable`);
    await act(async () => {
      requests.at(-1)!.reject(error);
      expect(await refresh).toBe(error);
    });
    await waitFor(() => expect(result.current.error?.message).toBe(error.message));
    expect(result.current.rows.map(({ id }) => id)).toEqual(originalIds);
    expect(client.readQuery({ query: ShowEpisodesDocument, variables: { showId: "show-a" } })?.episodes.edges.map(({ node }) => node.id)).toEqual(originalIds);
    expect(result.current.loadingMore).toBe(false);
    expect(requests).toHaveLength(failureAt === "first page" ? 3 : 4);
  });

  it("ignores an already-pending continuation while a newer refresh assembles its pages", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useShowEpisodes("show-a"), { wrapper });
    await act(async () => requests[0]!.resolve(page(rows(100), true, 103)));
    await waitFor(() => expect(requests).toHaveLength(2));
    let refresh!: ReturnType<typeof result.current.refetch>;
    act(() => { refresh = result.current.refetch(); });
    await waitFor(() => expect(requests).toHaveLength(3));
    await act(async () => requests[1]!.resolve(page(rows(3, 101), false, 103)));
    expect(result.current.rows.map(({ id }) => id)).toEqual(rows(100).map(({ id }) => id));
    expect(result.current.loadingMore).toBe(true);
    expect(result.current.complete).toBe(false);
    expect(requests).toHaveLength(3);
    await act(async () => requests[2]!.resolve(page(rows(100), true, 102)));
    await waitFor(() => expect(requests).toHaveLength(4));
    expect(result.current.rows.map(({ id }) => id)).toEqual(rows(100).map(({ id }) => id));
    expect(requests).toHaveLength(4);
    await act(async () => {
      requests[3]!.resolve(page(rows(2, 101), false, 102));
      await refresh;
    });
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(result.current.rows.map(({ id }) => id)).toEqual(rows(102).map(({ id }) => id));
    expect(result.current.error).toBeUndefined();
    expect(requests).toHaveLength(4);
  });

  it("does not reuse a stale in-flight continuation as a refreshed page", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useShowEpisodes("show-a"), { wrapper });
    await act(async () => requests[0]!.resolve(page(rows(100), true, 103)));
    await waitFor(() => expect(requests).toHaveLength(2));
    let refresh!: ReturnType<typeof result.current.refetch>;
    act(() => { refresh = result.current.refetch(); });
    await waitFor(() => expect(requests).toHaveLength(3));
    // The old continuation describes a snapshot containing episode 102. The
    // refreshed snapshot has deleted it, while keeping episodes 101 and 103.
    await act(async () => requests[2]!.resolve(page(rows(100), true, 102)));
    await act(async () => {
      // If refresh issued its own request, deliver the newer snapshot to it.
      // Otherwise Apollo has reused request 1, and its stale response must not
      // be accepted as the completed refresh.
      requests[1]!.resolve(page(rows(3, 101), false, 103));
      requests[3]?.resolve(page([episode(101), episode(103)], false, 102));
      await refresh;
    });
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(result.current.rows.map(({ id }) => id)).toEqual(rows(103).filter(({ id }) => id !== "show-a-102").map(({ id }) => id));
  });

  it.each(["first page", "continuation"] as const)("ignores an old show's refresh with a pending %s after route changes", async (pendingAt) => {
    const { client, requests, wrapper } = mediaHookHarness();
    const { result, rerender } = renderHook(({ showId }) => useShowEpisodes(showId), { wrapper, initialProps: { showId: "show-a" } });
    await act(async () => requests[0]!.resolve(page(rows(100), true, 103)));
    await waitFor(() => expect(requests).toHaveLength(2));
    await act(async () => requests[1]!.resolve(page(rows(3, 101), false, 103)));
    await waitFor(() => expect(result.current.complete).toBe(true));
    let refresh!: ReturnType<typeof result.current.refetch>;
    act(() => { refresh = result.current.refetch(); });
    await waitFor(() => expect(requests).toHaveLength(3));
    if (pendingAt === "continuation") {
      await act(async () => requests[2]!.resolve(page(rows(100), true, 101)));
      await waitFor(() => expect(requests).toHaveLength(4));
    }
    const staleRequest = requests.at(-1)!;
    rerender({ showId: "show-b" });
    await waitFor(() => expect(requests.at(-1)!.operation.variables.showId).toBe("show-b"));
    const newRows = Array.from({ length: 101 }, (_, index) => episode(index + 1, "show-b"));
    await act(async () => requests.at(-1)!.resolve(page(newRows.slice(0, 100), true, 101)));
    await waitFor(() => expect(requests.at(-1)!.operation.variables).toEqual({ showId: "show-b", offset: 100 }));
    const newContinuation = requests.at(-1)!;
    await act(async () => {
      // A complete stale first page avoids issuing additional old-show requests.
      staleRequest.resolve(page(pendingAt === "first page" ? [episode(1)] : [episode(101)]));
      await refresh;
    });
    expect(result.current.rows.map(({ id }) => id)).toEqual(newRows.slice(0, 100).map(({ id }) => id));
    await act(async () => newContinuation.resolve(page(newRows.slice(100), false, 101)));
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(result.current.rows.map(({ id }) => id)).toEqual(newRows.map(({ id }) => id));
    expect(result.current.error).toBeUndefined();
    expect(client.readQuery({ query: ShowEpisodesDocument, variables: { showId: "show-a" } })?.episodes.edges.map(({ node }) => node.id)).toEqual(rows(103).map(({ id }) => id));
    expect(client.readQuery({ query: ShowEpisodesDocument, variables: { showId: "show-b" } })?.episodes.edges.map(({ node }) => node.id)).toEqual(newRows.map(({ id }) => id));
  });
});
