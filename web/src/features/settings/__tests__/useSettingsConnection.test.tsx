import { ApolloClient, ApolloLink, InMemoryCache, Observable } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import { EntityAppLogListDocument, type AppLogFieldsFragment, type EntityAppLogListQueryVariables } from "@/graphql/generated/graphql";

import { useSettingsConnection } from "../useSettingsConnection";

const row = (id: number): AppLogFieldsFragment => ({ id: String(id), timestamp: "2026-10-24T00:00:00Z", createdAt: "2026-10-24T00:00:00Z", level: "INFO", target: "test", message: `Entry ${id}`, fields: null, spanName: null, spanId: null });
const connection = (ids: number[], hasNextPage: boolean, totalCount = ids.length) => ({ appLogs: { edges: ids.map((id) => ({ node: { __typename: "AppLog", ...row(id) } })), pageInfo: { hasNextPage, hasPreviousPage: false, totalCount } } });
function harness(respond: (variables: EntityAppLogListQueryVariables) => Promise<ReturnType<typeof connection>>) {
  const requests = vi.fn(respond);
  const client = new ApolloClient({ cache: new InMemoryCache(), link: new ApolloLink((operation) => new Observable((observer) => {
    void requests(operation.variables).then((data) => { observer.next({ data }); observer.complete(); }, (error: unknown) => observer.error(error));
  })) });
  const wrapper = ({ children }: { children: ReactNode }) => <ApolloProvider client={client}>{children}</ApolloProvider>;
  return { wrapper, requests };
}

describe("complete settings snapshots", () => {
  it("loads beyond 100 before exposing rows to local filters", async () => {
    let finish!: () => void;
    const gate = new Promise<void>((resolve) => { finish = resolve; });
    const view = harness(async (variables) => {
      if (variables.page?.offset === 0) return connection(Array.from({ length: 100 }, (_, i) => i), true, 101);
      await gate;
      return connection([100], false, 101);
    });
    const { result } = renderHook(() => useSettingsConnection(EntityAppLogListDocument, {}, (data) => data.appLogs), view);
    await waitFor(() => expect(view.requests).toHaveBeenCalledTimes(2));
    expect(result.current.rows).toEqual([]);
    expect(result.current.loading).toBe(true);
    await act(async () => { finish(); });
    await waitFor(() => expect(result.current.rows).toHaveLength(101));
    expect(result.current.rows.at(-1)?.message).toBe("Entry 100");
    expect(view.requests.mock.lastCall?.[0].page).toEqual({ limit: 100, offset: 100 });
  });

  it("keeps the complete previous snapshot after a later batch fails, then retries atomically", async () => {
    let fail = false;
    const view = harness(async (variables) => {
      if (variables.page?.offset === 0) return connection([1], true, 2);
      if (fail) throw new Error("offline");
      return connection([2], false, 2);
    });
    const { result } = renderHook(() => useSettingsConnection(EntityAppLogListDocument, {}, (data) => data.appLogs), view);
    await waitFor(() => expect(result.current.rows).toHaveLength(2));
    const original = result.current.rows;
    fail = true;
    await act(async () => { await result.current.refetch(); });
    expect(result.current.error?.message).toBe("offline");
    expect(result.current.rows).toBe(original);
    fail = false;
    await act(async () => { await result.current.refetch(); });
    expect(result.current.error).toBeUndefined();
    expect(result.current.rows).toHaveLength(2);
  });

  it("reports a short snapshot when the server count says rows are missing", async () => {
    const view = harness(async () => connection([1], false, 2));
    const { result } = renderHook(() => useSettingsConnection(EntityAppLogListDocument, {}, (data) => data.appLogs), view);
    await waitFor(() => expect(result.current.error?.message).toContain("load every row"));
    expect(result.current.rows).toEqual([]);
  });

  it("does not expose an incomplete first snapshot or loop on an empty server page", async () => {
    const view = harness(async (variables) => variables.page?.offset === 0 ? connection([1], true, 2) : connection([], true, 2));
    const { result } = renderHook(() => useSettingsConnection(EntityAppLogListDocument, {}, (data) => data.appLogs), view);
    await waitFor(() => expect(result.current.error).toBeDefined());
    expect(result.current.rows).toEqual([]);
    expect(result.current.loading).toBe(false);
    expect(view.requests).toHaveBeenCalledTimes(2);
  });
});

describe("server infinite batches", () => {
  it("deduplicates shifted IDs while advancing by raw server edges; clear resets the loaded window", async () => {
    const view = harness(async (variables) => {
      switch (variables.page?.offset) {
        case 0: return connection([1, 2], true, 5);
        case 2: return connection([2, 3], true, 5);
        default: return connection([4], false, 5);
      }
    });
    const { result } = renderHook(() => useSettingsConnection(EntityAppLogListDocument, {}, (data) => data.appLogs, { infinite: true, batchSize: 2 }), view);
    await waitFor(() => expect(result.current.rows).toHaveLength(2));
    await act(async () => { await Promise.all([result.current.loadMore(), result.current.loadMore()]); });
    expect(view.requests).toHaveBeenCalledTimes(2);
    expect(result.current.rows.map((node) => node.id)).toEqual(["1", "2", "3"]);
    await act(async () => { await result.current.loadMore(); });
    expect(view.requests.mock.lastCall?.[0].page?.offset).toBe(4);
    expect(result.current.hasMore).toBe(false);
    await act(async () => { await result.current.refetch({ reset: true }); });
    expect(result.current.rows.map((node) => node.id)).toEqual(["1", "2"]);
    expect(result.current.hasMore).toBe(true);
  });

  it("keeps loaded rows after a batch failure and retries the same offset", async () => {
    let fail = true;
    const view = harness(async (variables) => {
      if (variables.page?.offset === 0) return connection([1], true, 2);
      if (fail) throw new Error("batch offline");
      return connection([2], false, 2);
    });
    const { result } = renderHook(() => useSettingsConnection(EntityAppLogListDocument, {}, (data) => data.appLogs, { infinite: true }), view);
    await waitFor(() => expect(result.current.rows).toHaveLength(1));
    await act(async () => { await expect(result.current.loadMore()).rejects.toThrow("batch offline"); });
    expect(result.current.rows).toHaveLength(1);
    expect(result.current.moreError?.message).toBe("batch offline");
    fail = false;
    await act(async () => { await result.current.loadMore(); });
    expect(view.requests.mock.lastCall?.[0].page?.offset).toBe(1);
    expect(result.current.rows).toHaveLength(2);
    expect(result.current.moreError).toBeUndefined();
  });

  it("discards a stale batch when search changes and starts from offset zero", async () => {
    let finish!: () => void;
    const gate = new Promise<void>((resolve) => { finish = resolve; });
    const view = harness(async (variables) => {
      if (variables.where?.message) return connection([9], false);
      if (variables.page?.offset) { await gate; return connection([2], false); }
      return connection([1], true, 2);
    });
    const { result, rerender } = renderHook(({ search }) => useSettingsConnection(EntityAppLogListDocument, { where: search ? { message: { contains: search } } : {} }, (data) => data.appLogs, { infinite: true }), { ...view, initialProps: { search: "" } });
    await waitFor(() => expect(result.current.rows).toHaveLength(1));
    let pending!: Promise<void>;
    act(() => { pending = result.current.loadMore(); });
    rerender({ search: "filtered" });
    await waitFor(() => expect(result.current.rows[0]?.id).toBe("9"));
    await act(async () => { finish(); await pending; });
    expect(result.current.rows.map((node) => node.id)).toEqual(["9"]);
    expect(view.requests.mock.lastCall?.[0].page?.offset).toBe(0);
  });
});
