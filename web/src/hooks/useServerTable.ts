import { useCallback, useMemo, useState } from "react";

import type { OrderDirection, PageInput } from "@/graphql/generated/graphql";
import { usePref } from "@/lib/prefs";

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
}

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
}: UseServerTableOptions) {
  const [pageSize, setPageSize] = usePref<number>(`table.${persistKey}.pageSize`, defaultPageSize);
  const [pageIndex, setPageIndex] = useState(0);
  const [sorting, setSortingState] = useState<SortingState[]>(defaultSorting);

  const setSorting = useCallback((next: SortingState[]) => {
    setSortingState(next);
    setPageIndex(0);
  }, []);

  const resetPage = useCallback(() => setPageIndex(0), []);

  const page: PageInput = useMemo(() => ({ limit: Math.min(pageSize, 100), offset: pageIndex * Math.min(pageSize, 100) }), [pageIndex, pageSize]);

  const orderBy = useMemo(() => {
    if (sorting.length === 0) return undefined;
    return sorting.map((sort) => ({ [sortMap[sort.id] ?? sort.id]: sort.desc ? "DESC" : "ASC" }) as TOrderBy);
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
