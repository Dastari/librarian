import { createMemoryHistory, createRootRoute, createRouter, RouterProvider } from "@tanstack/react-router";
import { act, render, renderHook, waitFor } from "@testing-library/react";
import { createDataTableColumns, filters } from "glass-ux/data-table";
import { describe, expect, it, vi } from "vitest";

import { parseTableUrlState, tableUrlFilters, useTableUrlState } from "../useTableUrlState";
import { useServerTable } from "../useServerTable";
import type { MovieOrderByInput } from "@/graphql/generated/graphql";

const defaults = [{ id: "title", desc: false }];
const col = createDataTableColumns<{ title: string; size: number }>();
const columns = col.columns([
  col.accessor("title", { header: "Title", filter: filters.text({ field: "title", operators: ["contains", "eq"], nullable: false }) }),
  col.accessor("size", { header: "Size", filter: filters.number({ field: "size", server: "decimal", operators: ["gt", "between"], min: 0, max: 100 }) }),
]);
const schema = { sortableColumns: ["title", "size"], columns };
function entry(search: Record<string, unknown>) {
  return `/?${new URLSearchParams(Object.entries(search).map(([key, value]) => [key, typeof value === "string" ? value : JSON.stringify(value)]))}`;
}
async function mount<T>(hook: () => T, search: Record<string, unknown> = {}) {
  let value: T;
  const root = createRootRoute({ component: () => { value = hook(); return <div />; } });
  const history = createMemoryHistory({ initialEntries: [entry(search)] });
  const router = createRouter({ routeTree: root, history, defaultPendingMinMs: 0 });
  await router.load();
  render(<RouterProvider router={router} />);
  await waitFor(() => expect(value).toBeDefined());
  return { get value() { return value!; }, router, history };
}

describe("validated table URL query state", () => {
  it("validates public column codecs and strips private state and injected conditions", () => {
    expect(parseTableUrlState({
      sorting: [{ id: "__proto__", desc: false }, { id: "title", desc: true }, { id: "title", desc: false }, { id: "size", desc: "yes" }],
      filters: [{ id: "title", value: { op: "contains", value: "Movie", toCondition: { admin: true } } }, { id: "size", value: { op: "gt", value: 20 } }, { id: "unknown", value: { op: "eq", value: "secret" } }],
      search: "film", selection: { privateId: true }, pagination: { pageIndex: 999 },
    }, schema)).toEqual({ sorting: [{ id: "title", desc: true }], filters: [{ id: "title", value: { op: "contains", value: "Movie" } }, { id: "size", value: { op: "gt", value: 20 } }], search: "film" });
  });

  it("rejects malformed URLs, forbidden operators and out-of-range values", () => {
    for (const raw of ["{", "x".repeat(12001), null, []]) expect(parseTableUrlState(raw, schema, defaults)).toEqual({ sorting: defaults, filters: [], search: "" });
    for (const value of [{ op: "ne", value: "x" }, { op: "contains", value: null }]) expect(parseTableUrlState({ filters: [{ id: "title", value }] }, schema).filters).toEqual([]);
    expect(parseTableUrlState({ filters: [{ id: "size", value: { op: "gt", value: 101 } }] }, schema).filters).toEqual([]);
    expect(parseTableUrlState({ filters: [{ id: "level", value: { op: "in", values: ["ROOT"] } }] }, { sortableColumns: [], filters: { level: tableUrlFilters.options(["WARN", "ERROR"]) } }).filters).toEqual([]);
  });

  it("hydrates links, preserves unrelated params and restores Back/Forward without loops", async () => {
    const initial = { sorting: [{ id: "size", desc: true }], filters: [{ id: "title", value: { op: "eq", value: "Shared" } }], search: "old" };
    const view = await mount(() => useTableUrlState({ namespace: "movies.one", ...schema, defaultSorting: defaults }), { q: "external", show: "wanted", tab: "files", "table.movies.one": initial });
    expect(view.value.state).toEqual(initial);
    const navigate = vi.spyOn(view.router, "navigate");
    const next = { ...initial, search: "new" };
    act(() => view.value.onStateChange(next));
    await waitFor(() => expect(view.value.state.search).toBe("new"));
    expect(view.router.state.location.search).toMatchObject({ q: "external", show: "wanted", tab: "files" });
    expect(String(view.router.state.location.searchStr)).not.toMatch(/pageIndex|selection|cursor|offset/);
    act(() => view.history.back());
    await waitFor(() => expect(view.value.state).toEqual(initial));
    act(() => view.history.forward());
    await waitFor(() => expect(view.value.state.search).toBe("new"));
    expect(navigate).toHaveBeenCalledTimes(1);
  });

  it("isolates namespaces and ignores selection or pagination changes", async () => {
    const view = await mount(() => ({ a: useTableUrlState({ namespace: "a", ...schema }), b: useTableUrlState({ namespace: "b", ...schema }) }));
    act(() => view.value.a.onStateChange({ sorting: defaults, filters: [], search: "one" }));
    await waitFor(() => expect(view.value.a.state.search).toBe("one"));
    expect(view.value.b.state.search).toBe("");
    const before = view.router.state.location.href;
    act(() => view.value.a.onStateChange({ ...view.value.a.state, search: "ignored" }, "selection"));
    expect(view.router.state.location.href).toBe(before);
  });

  it("works locally without a router or nuqs adapter", () => {
    const { result } = renderHook(() => useTableUrlState({ namespace: "embedded", ...schema }));
    act(() => result.current.onStateChange({ sorting: defaults, filters: [], search: "local" }));
    expect(result.current.state.search).toBe("local");
  });

  it("hydrates server sorting with camelCase mappings and resets offsets on history changes", async () => {
    const view = await mount(() => useServerTable<MovieOrderByInput>({ persistKey: "movies.library", defaultSorting: defaults, sortMap: { title: "sortTitle" } }), { "table.movies.library": { sorting: [{ id: "title", desc: true }, { id: "secret", desc: true }] } });
    expect(view.value.orderBy).toEqual([{ sortTitle: "DESC" }]);
    act(() => view.value.tableProps.onPageIndexChange(3));
    expect(view.value.page.offset).toBe(150);
    act(() => view.value.setSorting([{ id: "year", desc: true }]));
    await waitFor(() => expect(view.value.orderBy).toEqual([{ year: "DESC" }]));
    expect(view.value.page.offset).toBe(0);
    act(() => view.value.tableProps.onPageIndexChange(2));
    act(() => view.history.back());
    await waitFor(() => expect(view.value.orderBy).toEqual([{ sortTitle: "DESC" }]));
    expect(view.value.page.offset).toBe(0);
  });
});
