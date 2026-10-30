import { gql, type TypedDocumentNode } from "@apollo/client";
import { MockedProvider } from "@apollo/client/testing/react";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it } from "vitest";

import type { PageInput } from "@/graphql/generated/graphql";

import { useCompleteConnection } from "../useCompleteConnection";
import { INFINITE_PAGE_SIZE, useInfiniteConnection } from "../useInfiniteConnection";

type Data = { records: { edges: Array<{ node: { id: string } }>; pageInfo: { hasNextPage: boolean; totalCount: number } } };
type Variables = { scope: string; page?: PageInput | null };
const Document = gql`
  query InfiniteRecords($scope: String!, $page: PageInput) {
    records(scope: $scope, page: $page) {
      edges { node { id } }
      pageInfo { hasNextPage totalCount }
    }
  }
` as TypedDocumentNode<Data, Variables>;
const select = (data: Data) => data.records;
const batch = (scope: string, offset: number, ids: string[], hasMore: boolean, options: { delay?: number; error?: Error; total?: number } = {}) => ({
  request: { query: Document, variables: { scope, page: { limit: INFINITE_PAGE_SIZE, offset } } },
  delay: options.delay ?? 1,
  ...(options.error ? { error: options.error } : {
    result: { data: { records: { edges: ids.map((id) => ({ node: { id } })), pageInfo: { hasNextPage: hasMore, totalCount: options.total ?? ids.length } } } },
  }),
});
const wrapper = (mocks: ReturnType<typeof batch>[]) => function Wrapper({ children }: { children: ReactNode }) {
  return <MockedProvider mocks={mocks}>{children}</MockedProvider>;
};

describe("useInfiniteConnection", () => {
  it("deduplicates shifted rows while advancing by the server edge count", async () => {
    const { result } = renderHook(() => useInfiniteConnection(Document, { scope: "a" }, select), {
      wrapper: wrapper([
        batch("a", 0, ["one", "two"], true, { total: 4 }),
        batch("a", 2, ["two", "three"], true, { total: 4 }),
        batch("a", 4, ["four"], false, { total: 4 }),
      ]),
    });
    await waitFor(() => expect(result.current.rows).toHaveLength(2));
    await act(async () => { await result.current.loadMore(); });
    await waitFor(() => expect(result.current.rows.map((row) => row.id)).toEqual(["one", "two", "three"]));
    await act(async () => { await result.current.loadMore(); });
    await waitFor(() => expect(result.current.rows.map((row) => row.id)).toEqual(["one", "two", "three", "four"]));
    expect(result.current.hasMore).toBe(false);
  });

  it("clears rows on a filter change and ignores the old filter's pending batch", async () => {
    const { result, rerender } = renderHook(({ scope }) => useInfiniteConnection(Document, { scope }, select), {
      initialProps: { scope: "a" },
      wrapper: wrapper([batch("a", 0, ["a1"], true), batch("a", 1, ["a2"], false, { delay: 80 }), batch("b", 0, ["b1"], false, { delay: 5 })]),
    });
    await waitFor(() => expect(result.current.rows[0]?.id).toBe("a1"));
    let pending: Promise<boolean>;
    act(() => { pending = result.current.loadMore(); });
    rerender({ scope: "b" });
    expect(result.current.rows).toEqual([]);
    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.rows[0]?.id).toBe("b1"));
    await act(async () => { await pending; });
    expect(result.current.rows.map((row) => row.id)).toEqual(["b1"]);
  });

  it("does not append a batch that was superseded by a subscription refresh", async () => {
    const { result } = renderHook(() => useInfiniteConnection(Document, { scope: "a" }, select), {
      wrapper: wrapper([batch("a", 0, ["old"], true), batch("a", 1, ["obsolete"], false, { delay: 80 }), batch("a", 0, ["fresh"], false)]),
    });
    await waitFor(() => expect(result.current.rows[0]?.id).toBe("old"));
    const revision = result.current.loadRevision;
    const dataset = result.current.queryKey;
    let pending: Promise<boolean>;
    act(() => { pending = result.current.loadMore(); });
    await act(async () => { await result.current.refetch(); });
    expect(result.current.loadRevision).toBeGreaterThan(revision);
    expect(result.current.queryKey).toBe(dataset);
    await act(async () => { await pending; });
    expect(result.current.rows.map((row) => row.id)).toEqual(["fresh"]);
  });
});

describe("useCompleteConnection", () => {
  it("drains every server batch beyond 100 records before exposing completion", async () => {
    const ids = Array.from({ length: 125 }, (_, index) => String(index));
    const { result } = renderHook(() => useCompleteConnection(Document, { scope: "all" }, select), {
      wrapper: wrapper([batch("all", 0, ids.slice(0, 60), true, { total: 125 }), batch("all", 60, ids.slice(60, 120), true, { total: 125 }), batch("all", 120, ids.slice(120), false, { total: 125 })]),
    });
    expect(result.current.complete).toBe(false);
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(result.current.rows.map((row) => row.id)).toEqual(ids);
  });

  it("retains loaded records and stops on failure until an explicit retry", async () => {
    const { result } = renderHook(() => useCompleteConnection(Document, { scope: "all" }, select), {
      wrapper: wrapper([
        batch("all", 0, ["first"], true),
        batch("all", 1, [], false, { error: new Error("Batch unavailable") }),
        batch("all", 0, ["first"], true),
        batch("all", 1, ["second"], false),
      ]),
    });
    await waitFor(() => expect(result.current.error?.message).toContain("Batch unavailable"));
    expect(result.current.complete).toBe(false);
    expect(result.current.rows.map((row) => row.id)).toEqual(["first"]);
    await act(async () => { await result.current.refetch(); });
    await waitFor(() => expect(result.current.complete).toBe(true));
    expect(result.current.rows.map((row) => row.id)).toEqual(["first", "second"]);
  });
});
