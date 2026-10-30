import type { DataTableState, DataTableStateChange } from "glass-ux/data-table";

import type { AppLogOrderByInput, AppLogWhereInput, EntityAppLogListQueryVariables, StringFilter } from "@/graphql/generated/graphql";

export const LOG_PAGE_SIZES = [25, 50, 100] as const;
export const LOG_LEVELS = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"] as const;

/** Keep the saved preference as a batch size, bounded to the API contract. */
export function logPageSize(value: number): number {
  return LOG_PAGE_SIZES.includes(value as (typeof LOG_PAGE_SIZES)[number]) ? value : 100;
}

export function initialLogTableState(pageSize: number): DataTableState {
  return { sorting: [{ id: "timestamp", desc: true }], filters: [], search: "", pagination: { pageIndex: 0, pageSize: logPageSize(pageSize) }, selection: {} };
}

/** Infinite views always start at the head; the connection loader owns batch offsets. */
export function changeLogTableState(next: DataTableState, _change: DataTableStateChange): DataTableState {
  return {
    ...next,
    selection: {},
    pagination: { pageIndex: 0, pageSize: logPageSize(next.pagination.pageSize) },
  };
}

function stringFilter(raw: unknown, options: boolean): StringFilter {
  if (!raw || typeof raw !== "object" || !("op" in raw)) throw new Error("Invalid log filter");
  if (options && (raw.op === "in" || raw.op === "notIn") && "values" in raw && Array.isArray(raw.values) && raw.values.every((value) => typeof value === "string")) {
    // Level values are server strings, not field names; preserve their case verbatim.
    return raw.op === "in" ? { inList: raw.values } : { notIn: raw.values };
  }
  if (!options && "value" in raw && typeof raw.value === "string") {
    switch (raw.op) {
      case "contains": return { contains: raw.value };
      case "eq": return { eq: raw.value };
      case "ne": return { ne: raw.value };
    }
  }
  throw new Error("Unsupported log filter operator");
}

/** Only the fields/operators declared by the log columns cross this camelCase boundary.
 * Do not pass glass-ux's PascalCase ORM adapter into Librarian operations.
 */
export function logTableVariables(state: DataTableState): EntityAppLogListQueryVariables {
  const conditions: AppLogWhereInput[] = state.filters.map(({ id, value }) => {
    switch (id) {
      case "level": return { level: stringFilter(value, true) };
      case "target": return { target: stringFilter(value, false) };
      case "message": return { message: stringFilter(value, false) };
      default: throw new Error(`Unsupported log filter: ${id}`);
    }
  });
  const query = state.search?.trim();
  if (query) conditions.push({ or: [{ message: { contains: query } }, { target: { contains: query } }] });

  const orderBy: AppLogOrderByInput[] = state.sorting.map(({ id, desc }) => {
    const direction = desc ? "DESC" : "ASC";
    switch (id) {
      case "timestamp": return { timestamp: direction };
      case "level": return { level: direction };
      case "target": return { target: direction };
      default: throw new Error(`Unsupported log sort: ${id}`);
    }
  });
  const limit = logPageSize(state.pagination.pageSize);
  return {
    where: conditions.length === 0 ? {} : conditions.length === 1 ? conditions[0] : { and: conditions },
    orderBy: orderBy.length ? orderBy : [{ timestamp: "DESC" }],
    page: { limit, offset: 0 },
  };
}
