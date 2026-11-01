import { localPage, type DataTableProps, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { useCallback, useEffect, useMemo, useState } from "react";

type Selection = DataTableState["selection"];
type SelectionOptions<T> = {
  scope: unknown;
  rows: readonly T[];
  ready?: boolean;
};

/** Selection is local to a principal and semantic dataset, never URL or refresh state. */
export function useTableSelection<T extends { id: string }>(options: SelectionOptions<T> & { getRowId?: (row: T) => string }): { selection: Selection; onSelectionChange: (next: Selection) => void };
export function useTableSelection<T>(options: SelectionOptions<T> & { getRowId: (row: T) => string }): { selection: Selection; onSelectionChange: (next: Selection) => void };
export function useTableSelection<T>({ scope, rows, getRowId, ready = true }: SelectionOptions<T> & { getRowId?: (row: T) => string }) {
  const key = JSON.stringify(scope);
  const [stored, setStored] = useState<{ key: string; selection: Selection }>({ key, selection: {} });
  const selection = stored.key === key ? stored.selection : {};
  const onSelectionChange = useCallback((next: Selection) => setStored({ key, selection: next }), [key]);
  useEffect(() => {
    const ids = new Set(rows.map((row) => getRowId ? getRowId(row) : (row as { id: string }).id));
    setStored((previous) => {
      if (previous.key !== key) return { key, selection: {} };
      if (!ready) return previous;
      if (Object.keys(previous.selection).every((id) => ids.has(id))) return previous;
      return { key, selection: Object.fromEntries(Object.entries(previous.selection).filter(([id]) => ids.has(id))) };
    });
  }, [key, rows, ready, getRowId]);
  return { selection, onSelectionChange };
}

const initialState = (): DataTableState => ({ pagination: { pageIndex: 0, pageSize: 50 }, sorting: [], filters: [], search: "", selection: {} });

/** Complete local tables need the same scope, query and removal boundaries as server tables. */
export function useSelectableTableState<T extends object>({ scope, rows, getRowId, ready = true, columns, search }: SelectionOptions<T> & {
  getRowId: (row: T) => string;
  /** Supply only for a complete local dataset; server windows own their query. */
  columns?: DataTableProps<T>["columns"];
  search?: DataTableProps<T>["search"];
}) {
  const key = JSON.stringify(scope);
  const [view, setView] = useState(() => ({ key, state: initialState() }));
  const local = view.key === key ? view.state : initialState();
  useEffect(() => { setView((previous) => previous.key === key ? previous : { key, state: initialState() }); }, [key]);
  const visibleRows = useMemo(() => columns ? localPage<T>([...rows], columns, { ...local, pagination: { pageIndex: 0, pageSize: rows.length || 1 } }, search).rows : rows, [rows, columns, local, search]);
  const { selection, onSelectionChange } = useTableSelection({ scope: [key, local.sorting, local.filters, local.search], rows: visibleRows, getRowId, ready });
  const onStateChange = (next: DataTableState, change: DataTableStateChange) => {
    if (change === "selection") onSelectionChange(next.selection);
    else setView({ key, state: next });
  };
  return { state: { ...local, selection }, onStateChange, rows: visibleRows };
}
