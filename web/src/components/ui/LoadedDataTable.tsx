import { DataTable, localPage, type DataTableProps, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { useMemo, useRef, useState } from "react";

import { useTableUrlState } from "@/hooks/useTableUrlState";
import { usePrintMode } from "@/hooks/usePrintMode";

type LoadedDataTableProps<T extends object> = Omit<DataTableProps<T>, "state" | "onStateChange" | "infinite" | "rowCount"> & {
  /** Filtering is enabled only after the collection has been loaded completely. */
  complete?: boolean;
  batchSize?: number;
};

/** A complete media collection: filter before revealing successive batches, never page it. */
export function LoadedDataTable<T extends object>({ rows, columns, search, complete = true, batchSize = 100, ...props }: LoadedDataTableProps<T>) {
  const printing = usePrintMode();
  const [view, setView] = useState<{ id: string; state: DataTableState; limit: number; revision: number; windowRevision?: number }>(() => ({
    id: props.id,
    state: { sorting: [], filters: [], search: "", pagination: { pageIndex: 0, pageSize: batchSize }, selection: {} },
    limit: batchSize,
    revision: 0,
  }));
  const current = view.id === props.id ? view : {
    id: props.id,
    state: { sorting: [], filters: [], search: "", pagination: { pageIndex: 0, pageSize: batchSize }, selection: {} },
    limit: batchSize,
    revision: view.revision + 1,
  };
  const snapshot = useRef({ id: props.id, ready: complete });
  if (snapshot.current.id !== props.id) snapshot.current = { id: props.id, ready: complete };
  if (complete) snapshot.current.ready = true;
  // A refresh retains a previously complete snapshot and its active filters.
  const ready = snapshot.current.ready;
  const availableColumns = useMemo(() => ready ? columns : columns.map((column) => ({
    ...column, meta: column.meta ? { ...column.meta, filter: null } : undefined,
  })), [columns, ready]);
  const url = useTableUrlState({
    namespace: props.id, columns: availableColumns,
    sortableColumns: availableColumns.flatMap((column) => column.enableSorting === false ? [] :
      typeof column.id === "string" ? [column.id] : "accessorKey" in column && typeof column.accessorKey === "string" ? [column.accessorKey] : []),
    search: ready && Boolean(search),
  });
  const state = useMemo(() => ({ ...current.state, ...url.state }), [current.state, url.state]);
  const datasetKey = JSON.stringify([props.id, url.state]);
  const dataset = useRef({ key: datasetKey, revision: current.revision });
  if (dataset.current.key !== datasetKey) dataset.current = { key: datasetKey, revision: dataset.current.revision + 1 };
  const limit = current.windowRevision === dataset.current.revision ? current.limit : batchSize;
  const page = useMemo(() => localPage(rows, availableColumns, {
    ...state,
    pagination: { pageIndex: 0, pageSize: printing ? rows.length || 1 : limit },
  }, ready ? search : undefined), [rows, availableColumns, state, limit, ready, search, printing]);
  const onStateChange = (state: DataTableState, change: DataTableStateChange) => {
    url.onStateChange(state, change);
    setView({
      id: props.id,
      windowRevision: dataset.current.revision,
      state: { ...state, pagination: { pageIndex: 0, pageSize: batchSize } },
      limit: change === "selection" ? current.limit : batchSize,
      revision: current.revision + Number(change !== "selection"),
    });
  };
  return <DataTable {...props} columns={availableColumns} rows={page.rows}
    state={state} onStateChange={onStateChange} search={ready ? search : undefined}
    queryKey={[props.id, props.queryKey, current.revision, dataset.current.revision]} rowCount={page.count} infinite={{ hasMore: !printing && page.rows.length < page.count,
      onLoadMore: () => setView({ ...current, state, windowRevision: dataset.current.revision, limit: limit + batchSize }),
    }} />;
}
