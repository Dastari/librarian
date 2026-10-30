import { useRouter, type AnyRouter } from "@tanstack/react-router";
import { parseTableSearch, serializeTableSearch, type DataTableState, type DataTableStateChange, type TableUrlConfig } from "glass-ux/data-table";
import { useCallback, useMemo, useState, useSyncExternalStore } from "react";

/** Only query state is shared. Selection, pagination and render windows stay local. */
export type TableUrlState = Pick<DataTableState, "sorting" | "filters" | "search">;
export type TableFilterValidator = (value: unknown) => boolean;
export interface TableUrlSchema {
  sortableColumns: readonly string[];
  /** Public Glass columns supply their own typed filter codecs and operator allowlists. */
  columns?: TableUrlConfig["columns"];
  filters?: Readonly<Record<string, TableFilterValidator>>;
  /** Disable when another URL control (for example the library browser's q) owns search. */
  search?: boolean;
}
export interface UseTableUrlStateOptions extends TableUrlSchema {
  namespace: string;
  defaultSorting?: DataTableState["sorting"];
}
const MAX_TEXT = 1000;
const MAX_PAYLOAD = 12000;
const EMPTY_SEARCH = {};
const noopSubscribe = () => () => {};
const object = (value: unknown): value is Record<string, unknown> => Boolean(value && typeof value === "object" && !Array.isArray(value));

/** Validators return only approved values to the existing camelCase GraphQL boundary. */
export const tableUrlFilters = {
  text: (operators: readonly string[] = ["contains", "eq", "ne"]): TableFilterValidator => (value) =>
    object(value) && typeof value.op === "string" && operators.includes(value.op) && typeof value.value === "string" && value.value.length <= MAX_TEXT,
  options: (values: readonly string[], operators: readonly string[] = ["in", "notIn"]): TableFilterValidator => (value) =>
    object(value) && typeof value.op === "string" && operators.includes(value.op) && Array.isArray(value.values) && value.values.length <= values.length && value.values.every((item) => typeof item === "string" && values.includes(item)),
};

export function parseTableUrlState(raw: unknown, schema: TableUrlSchema, defaultSorting: DataTableState["sorting"] = []): TableUrlState {
  let input: unknown = raw;
  if (typeof raw === "string") {
    try { input = raw.length <= MAX_PAYLOAD ? JSON.parse(raw) : null; } catch { input = null; }
  }
  if (object(input)) {
    try { if (JSON.stringify(input).length > MAX_PAYLOAD) input = null; } catch { input = null; }
  }
  const source = object(input) ? input : {};
  const seenSort = new Set<string>();
  const sorting = (Array.isArray(source.sorting) ? source.sorting : defaultSorting).filter((sort): sort is DataTableState["sorting"][number] => {
    if (!object(sort) || typeof sort.id !== "string" || typeof sort.desc !== "boolean" || !schema.sortableColumns.includes(sort.id) || seenSort.has(sort.id)) return false;
    seenSort.add(sort.id);
    return true;
  }).map(({ id, desc }) => ({ id, desc }));
  const seenFilter = new Set<string>();
  const filters: DataTableState["filters"] = [];
  if (Array.isArray(source.filters)) for (const filter of source.filters.slice(0, 30)) {
    if (!object(filter) || typeof filter.id !== "string" || seenFilter.has(filter.id)) continue;
    if (schema.filters && (!Object.hasOwn(schema.filters, filter.id) || !schema.filters[filter.id]!(filter.value))) continue;
    if (!schema.filters && !schema.columns) continue;
    let value = filter.value;
    if (schema.columns) {
      // Public codecs enforce each column's kinds, operators, options and bounds.
      // An untrusted malformed value may throw during encoding; drop that filter.
      const config: TableUrlConfig = { columns: schema.columns, sortable: [], pageSizes: [50], defaultPageSize: 50 };
      try {
        const decoded = parseTableSearch(serializeTableSearch({ sorting: [], filters: [{ id: filter.id, value }], pagination: { pageIndex: 0, pageSize: 50 } }, config), config);
        const valid = decoded.filters.find((candidate) => candidate.id === filter.id);
        if (!valid) continue;
        // Encoding is intentionally permissive (for example String(null)). Reject
        // type coercions while allowing the decoder to strip extra object keys.
        const inputValue = value;
        const typedInput = object(inputValue) && object(valid.value)
          ? Object.fromEntries(Object.keys(valid.value).map((field) => [field, inputValue[field]])) : inputValue;
        if (JSON.stringify(typedInput) !== JSON.stringify(valid.value)) continue;
        value = valid.value;
      } catch { continue; }
    } else if (object(value)) {
      value = Object.fromEntries(Object.entries(value).filter(([key]) => ["op", "value", "values", "min", "max"].includes(key)));
    }
    filters.push({ id: filter.id, value });
    seenFilter.add(filter.id);
  }
  return { sorting, filters, search: schema.search !== false && typeof source.search === "string" ? source.search.slice(0, MAX_TEXT) : "" };
}

/** Router-native updates coexist with nuqs and preserve unrelated route search keys.
 * With no RouterProvider (embedded components/tests), the same API uses local state.
 * No effect writes URL state: history navigation cannot feed back into navigation.
 */
export function useTableUrlState({ namespace, defaultSorting = [], ...schema }: UseTableUrlStateOptions) {
  const router = useRouter<AnyRouter>({ warn: false });
  const key = `table.${namespace}`;
  const subscribe = useCallback((notify: () => void) => router ? router.subscribe("onResolved", notify) : noopSubscribe(), [router]);
  const snapshot = useCallback(() => router?.state.location.search ?? EMPTY_SEARCH, [router]);
  const search = useSyncExternalStore(subscribe, snapshot, snapshot) as Record<string, unknown>;
  const [local, setLocal] = useState<{ key: string; state: TableUrlState } | null>(null);
  const raw = router ? search[key] : local?.key === key ? local.state : undefined;
  const signature = JSON.stringify([raw, schema.sortableColumns, defaultSorting]);
  // Callers may declare validators inline. Parsing is pure and never changes history.
  const state = useMemo(() => parseTableUrlState(raw, schema, defaultSorting), [signature, schema.columns, schema.filters, schema.search]);
  const onStateChange = useCallback((next: TableUrlState, change?: DataTableStateChange) => {
    if (change && change !== "sorting" && change !== "filters" && change !== "search") return;
    const safe = parseTableUrlState(next, schema, defaultSorting);
    if (JSON.stringify(safe) === JSON.stringify(state)) return;
    if (!router) { setLocal({ key, state: safe }); return; }
    const defaults = parseTableUrlState(undefined, schema, defaultSorting);
    void router.navigate({
      to: router.state.location.pathname,
      search: (previous: Record<string, unknown>) => ({ ...previous, [key]: JSON.stringify(safe) === JSON.stringify(defaults) ? undefined : JSON.stringify(safe) }),
      replace: false,
      resetScroll: false,
    });
  }, [router, key, schema, defaultSorting, state]);
  return { state, onStateChange };
}
