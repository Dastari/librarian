import { parseAsString, parseAsStringEnum, useQueryState } from "nuqs";

import type { ViewMode } from "@/components/ui";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import { usePref } from "@/lib/prefs";

import type { AvailabilityFilter } from "./BrowserToolbar";

const AVAILABILITY = ["all", "available", "wanted", "missing"] as const;

/** URL-backed filters (shareable, back-button friendly) plus the persisted view mode. */
export function useBrowserFilters(persistKey: string, defaultView: ViewMode = "card") {
  const [query, setQuery] = useQueryState("q", parseAsString.withDefault(""));
  const [availability, setAvailability] = useQueryState("show", parseAsStringEnum<AvailabilityFilter>([...AVAILABILITY]).withDefault("all"));
  const [view, setView] = usePref<ViewMode>(`browser.${persistKey}.view`, defaultView);
  const debouncedQuery = useDebouncedValue(query.trim(), 250);

  return {
    query,
    setQuery: (value: string) => void setQuery(value || null),
    debouncedQuery,
    availability,
    setAvailability: (value: AvailabilityFilter) => void setAvailability(value === "all" ? null : value),
    view,
    setView,
    /** Changes whenever any server-side filter changes; lists reset paging on it. */
    filterKey: `${debouncedQuery}|${availability}`,
  };
}

/** `title`/`name` filter for the free-text query (the letter rail scrolls instead of filtering). */
export function titleFilter(debouncedQuery: string, letter: string | null) {
  const filter: { contains?: string; startsWith?: string } = {};
  if (debouncedQuery) filter.contains = debouncedQuery;
  if (letter && letter !== "#") filter.startsWith = letter;
  return Object.keys(filter).length ? filter : undefined;
}
