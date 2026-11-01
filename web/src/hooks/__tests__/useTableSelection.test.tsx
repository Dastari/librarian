import { act, renderHook } from "@testing-library/react";
import { createDataTableColumns, filters } from "glass-ux/data-table";
import { describe, expect, it } from "vitest";

import { useSelectableTableState, useTableSelection } from "../useTableSelection";

type Row = { id: string; title: string };
const rows: Row[] = [{ id: "a", title: "Alpha" }, { id: "b", title: "Beta" }];
const col = createDataTableColumns<Row>();
const columns = col.columns([col.accessor("title", { header: "Title", filter: filters.text({ field: "title" }) })]);
const getRowId = (row: Row) => row.id;

describe("scoped table selection", () => {
  it("retains appended and live-updated targets, and prunes removed rows", () => {
    const hook = renderHook(({ items }) => useTableSelection({ scope: ["user", "library"], rows: items }), { initialProps: { items: rows } });
    act(() => hook.result.current.onSelectionChange({ a: true, b: true }));
    hook.rerender({ items: [...rows.map(row => ({ ...row, title: `${row.title} updated` })), { id: "c", title: "Gamma" }] });
    expect(hook.result.current.selection).toEqual({ a: true, b: true });
    hook.rerender({ items: [rows[1]!] });
    expect(hook.result.current.selection).toEqual({ b: true });
  });
  it("retains targets during refresh/failure and adopts only successful replacement rows", () => {
    const hook = renderHook(({ items, ready }) => useTableSelection({ scope: "one", rows: items, ready }), { initialProps: { items: rows, ready: true } });
    act(() => hook.result.current.onSelectionChange({ a: true, b: true }));
    hook.rerender({ items: [], ready: false });
    expect(hook.result.current.selection).toEqual({ a: true, b: true });
    hook.rerender({ items: rows, ready: false });
    expect(hook.result.current.selection).toEqual({ a: true, b: true });
    hook.rerender({ items: [rows[1]!], ready: true });
    expect(hook.result.current.selection).toEqual({ b: true });
  });
  it("never revives another principal's targets, including while the new scope loads", () => {
    const hook = renderHook(({ scope, ready }) => useTableSelection({ scope, rows, ready }), { initialProps: { scope: "owner-a", ready: true } });
    act(() => hook.result.current.onSelectionChange({ a: true }));
    hook.rerender({ scope: "owner-b", ready: false });
    expect(hook.result.current.selection).toEqual({});
    hook.rerender({ scope: "owner-a", ready: false });
    expect(hook.result.current.selection).toEqual({});
  });
  it("supports explicitly keyed heterogeneous datasets", () => {
    const hook = renderHook(() => useTableSelection({ scope: "groups", rows: [{ key: "show:a" }], getRowId: (row) => row.key }));
    act(() => hook.result.current.onSelectionChange({ "show:a": true }));
    expect(hook.result.current.selection).toEqual({ "show:a": true });
  });
  it("clears local query selection and actually filters a complete dataset", () => {
    const hook = renderHook(() => useSelectableTableState({ scope: "local", rows, columns, getRowId, search: { fields: ["title"] } }));
    act(() => hook.result.current.onStateChange({ ...hook.result.current.state, selection: { a: true } }, "selection"));
    act(() => hook.result.current.onStateChange({ ...hook.result.current.state, search: "Beta" }, "search"));
    expect(hook.result.current.state.selection).toEqual({});
    expect(hook.result.current.rows.map(row => row.id)).toEqual(["b"]);
    act(() => hook.result.current.onStateChange({ ...hook.result.current.state, search: "" }, "search"));
    expect(hook.result.current.rows).toHaveLength(2);
    expect(hook.result.current.state.selection).toEqual({});
  });
  it("drops a target that leaves the active filter after an update", () => {
    const hook = renderHook(({ items }) => useSelectableTableState({ scope: "filtered", rows: items, columns, getRowId, search: { fields: ["title"] } }), { initialProps: { items: rows } });
    act(() => hook.result.current.onStateChange({ ...hook.result.current.state, search: "Alpha" }, "search"));
    act(() => hook.result.current.onStateChange({ ...hook.result.current.state, selection: { a: true } }, "selection"));
    hook.rerender({ items: [{ ...rows[0]!, title: "Renamed" }, rows[1]!] });
    expect(hook.result.current.rows).toEqual([]);
    expect(hook.result.current.state.selection).toEqual({});
  });
});
