import { gql, type TypedDocumentNode } from "@apollo/client";
import { MockedProvider } from "@apollo/client/testing/react";
import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, it } from "vitest";
import type { ReactNode } from "react";

import type { PageInput } from "@/graphql/generated/graphql";
import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import { LiveContext } from "../hooks";
import { LiveRegistry, type LiveEvent } from "../registry";
import type { LiveTopic } from "../documents";

type Row = { id: string; name: string };
type Data = { movies: { edges: Array<{ node: Row }>; pageInfo: { hasNextPage: boolean; totalCount: number } } };
const Document = gql`query LiveSnapshot($scope: String!, $page: PageInput) {
  movies(scope: $scope, page: $page) { edges { node { id name } } pageInfo { hasNextPage totalCount } }
}` as TypedDocumentNode<Data, { scope: string; page?: PageInput | null }>;
const rows = (offset: number, length: number, prefix = "old") => Array.from({ length }, (_, index) => ({ id: String(offset + index), name: `${prefix}-${offset + index}` }));
const batch = (scope: string, offset: number, nodes: Row[], total: number, error?: Error) => ({
  request: { query: Document, variables: { scope, page: { limit: 100, offset } } },
  ...(error ? { error } : { result: { data: { movies: { edges: nodes.map((node) => ({ node })), pageInfo: { hasNextPage: offset + nodes.length < total, totalCount: total } } } } }),
});

it("refreshes a registered imperative snapshot beyond 100 and preserves every prior row on a later-page failure", async () => {
  const emit = new Map<LiveTopic, (event: LiveEvent) => void>();
  const registry = new LiveRegistry((topic, callback) => { emit.set(topic, callback); return () => { emit.delete(topic); }; }, { coalesceMs: 0, minGapMs: 0, concurrency: 4, maxRetries: 0 });
  registry.reset(true);
  const mocks = [
    batch("a", 0, rows(0, 100), 120), batch("a", 100, rows(100, 20), 120),
    batch("a", 0, rows(0, 100, "fresh"), 120), batch("a", 100, [], 120, new Error("page offline")),
    batch("a", 0, rows(0, 100, "fresh"), 120), batch("a", 100, rows(100, 20, "fresh"), 120),
    batch("b", 0, rows(1000, 1, "principal-b"), 1),
  ];
  const wrapper = ({ children }: { children: ReactNode }) => <MockedProvider mocks={mocks}><LiveContext.Provider value={{ registry, epoch: "test-principal" }}>{children}</LiveContext.Provider></MockedProvider>;
  const { result, rerender, unmount } = renderHook(({ scope }) => useSettingsConnection(Document, { scope }, (data) => data.movies), { initialProps: { scope: "a" }, wrapper });
  await waitFor(() => expect(result.current.rows).toHaveLength(120));
  const previous = result.current.rows;
  act(() => emit.get("Movie")!({ topic: "Movie", id: "new-member" }));
  await waitFor(() => expect(result.current.error?.message).toBe("page offline"));
  expect(result.current.rows).toBe(previous);
  expect(result.current.rows.at(-1)?.name).toBe("old-119");
  act(() => emit.get("Movie")!({ topic: "Movie" }));
  await waitFor(() => expect(result.current.rows.at(-1)?.name).toBe("fresh-119"));
  rerender({ scope: "b" });
  expect(result.current.rows).toEqual([]);
  await waitFor(() => expect(result.current.rows[0]?.name).toBe("principal-b-1000"));
  unmount();
  expect(emit.size).toBe(0);
  registry.reset(false);
});

it("retains a loaded keyset window on failure and retries its new cursor chain atomically", async () => {
  const KeysetDocument = gql`query CursorSnapshot($scope: String!, $page: KeysetPageInput) {
    movies: appLogsKeyset(scope: $scope, page: $page) { edges { node { id name } } pageInfo { hasNextPage totalCount endCursor } }
  }` as TypedDocumentNode<Data & { movies: Data["movies"] & { pageInfo: Data["movies"]["pageInfo"] & { endCursor: string } } }, { scope: string; page?: import("@/graphql/generated/graphql").KeysetPageInput | null }>;
  const cursorBatch = (after: string | null, nodes: Row[], next: string, more: boolean, error?: Error) => ({
    request: { query: KeysetDocument, variables: { scope: "a", page: { limit: 100, after, includeTotalCount: true } } },
    ...(error ? { error } : { result: { data: { movies: { edges: nodes.map((node) => ({ node })), pageInfo: { hasNextPage: more, totalCount: 120, endCursor: next } } } } }),
  });
  const mocks = [
    cursorBatch(null, rows(0, 100), "old-head", true), cursorBatch("old-head", rows(100, 20), "old-tail", false),
    cursorBatch(null, rows(0, 100, "fresh"), "fresh-head", true), cursorBatch("fresh-head", [], "", false, new Error("cursor page offline")),
    cursorBatch(null, rows(0, 100, "fresh"), "fresh-head", true), cursorBatch("fresh-head", rows(100, 20, "fresh"), "fresh-tail", false),
  ];
  const wrapper = ({ children }: { children: ReactNode }) => <MockedProvider mocks={mocks}>{children}</MockedProvider>;
  const { result } = renderHook(() => useSettingsConnection(KeysetDocument, { scope: "a" }, (data) => data.movies, { infinite: true, keyset: true }), { wrapper });
  await waitFor(() => expect(result.current.rows).toHaveLength(100));
  await act(async () => { await result.current.loadMore(); });
  expect(result.current.rows).toHaveLength(120);
  const previous = result.current.rows;
  await act(async () => { await result.current.refetch(); });
  expect(result.current.error?.message).toBe("cursor page offline");
  expect(result.current.rows).toBe(previous);
  await act(async () => { await result.current.refetch(); });
  expect(result.current.rows).toHaveLength(120);
  expect(result.current.rows.at(-1)?.name).toBe("fresh-119");
});
