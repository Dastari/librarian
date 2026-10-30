import { useCallback, useEffect, useMemo, useState } from "react";

import type { OrderDirection, PageInput } from "@/graphql/generated/graphql";
import { usePref } from "@/lib/prefs";
import { useTableUrlState } from "./useTableUrlState";

export interface SortingState {
  id: string;
  desc: boolean;
}

interface UseServerTableOptions {
  /** Persist page size per table. */
  persistKey: string;
  defaultPageSize?: number;
  defaultSorting?: SortingState[];
  /** Sortable column ids and the orderBy key they map to (defaults to the same id). */
  sortMap?: Record<string, string>;
  /** Explicit allowlist for new consumers; existing browser datasets are declared below. */
  sortableColumns?: readonly string[];
}

// Existing consumers retain their public API while untrusted URL fields are allowlisted.
const BROWSER_SORTS: Record<string, readonly string[]> = {
  movies: ["title", "runtime", "year", "createdAt"], shows: ["name", "createdAt"],
  albums: ["name", "sizeBytes", "year"], artists: ["name", "albumCount"],
  audiobooks: ["title", "totalDurationSecs", "sizeBytes"], tracks: ["title", "trackNumber", "durationSecs"],
  collections: ["name", "movieCount", "lastSyncedAt"], files: ["path", "duration", "size", "addedAt"],
};

/**
 * Server-side pagination and sorting state for DataTable-backed lists. Produces the `page` and
 * `orderBy` inputs the generated list queries expect and the props the table needs to render
 * its footer. Page index resets whenever sorting or the caller's filter key changes.
 */
export function useServerTable<TOrderBy extends Record<string, OrderDirection | null | undefined>>({
  persistKey,
  defaultPageSize = 50,
  defaultSorting = [],
  sortMap = {},
  sortableColumns,
}: UseServerTableOptions) {
  const [pageSize, setPageSize] = usePref<number>(`table.${persistKey}.pageSize`, defaultPageSize);
  const [pageIndex, setPageIndex] = useState(0);
  const dataset = persistKey.split(".")[0]!;
  const url = useTableUrlState({
    namespace: persistKey, defaultSorting, search: false,
    sortableColumns: sortableColumns ?? (Object.hasOwn(BROWSER_SORTS, dataset) ? BROWSER_SORTS[dataset] : undefined) ?? [...new Set([...Object.keys(sortMap), ...defaultSorting.map((sort) => sort.id)])],
  });
  const sorting = url.state.sorting;
  const sortingKey = JSON.stringify(sorting);
  // Back/forward uses the same reset path as a sort made through the table.
  useEffect(() => setPageIndex(0), [persistKey, sortingKey]);

  const setSorting = useCallback((next: SortingState[]) => {
    url.onStateChange({ ...url.state, sorting: next }, "sorting");
    setPageIndex(0);
  }, [url]);

  const resetPage = useCallback(() => setPageIndex(0), []);

  const page: PageInput = useMemo(() => ({ limit: Math.min(pageSize, 100), offset: pageIndex * Math.min(pageSize, 100) }), [pageIndex, pageSize]);

  const orderBy = useMemo(() => {
    if (sorting.length === 0) return undefined;
    return sorting.map((sort) => ({ [(Object.hasOwn(sortMap, sort.id) ? sortMap[sort.id] ?? sort.id : sort.id)]: sort.desc ? "DESC" : "ASC" }) as TOrderBy);
  }, [sorting, sortMap]);

  const tableProps = useMemo(
    () => ({
      pageIndex,
      pageSize: Math.min(pageSize, 100),
      onPageIndexChange: setPageIndex,
      onPageSizeChange: (size: number) => {
        setPageSize(size);
        setPageIndex(0);
      },
      sorting,
      onSortingChange: setSorting,
      pageSizeOptions: [25, 50, 100],
    }),
    [pageIndex, pageSize, setPageSize, sorting, setSorting],
  );

  return { page, orderBy, pageIndex, pageSize, sorting, setSorting, resetPage, tableProps };
}
