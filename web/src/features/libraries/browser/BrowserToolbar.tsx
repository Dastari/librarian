import { SearchField } from "glass-ux/input";
import { Select } from "glass-ux/select";
import type { ReactNode } from "react";

import { AlphabetRail, GlassSegmented } from "@/components/ui";
import type { SortingState } from "@/hooks/useServerTable";
import { useIsDesktop } from "@/hooks/useMediaQuery";

export type AvailabilityFilter = "all" | "available" | "wanted" | "missing";

interface BrowserToolbarProps {
  query: string;
  onQueryChange: (value: string) => void;
  letter: string | null;
  onLetterChange: (letter: string | null) => void;
  availability?: AvailabilityFilter;
  onAvailabilityChange?: (filter: AvailabilityFilter) => void;
  /** Poster browsers have no column headers, so their orderings are offered here. */
  sort?: BrowserSort;
  /** Extra controls (sort menu, add button). */
  trailing?: ReactNode;
  placeholder?: string;
  /** The table footer reports the count; kept so callers can pass it without a second display. */
  total?: number;
  /** Table-only views have no alphabet jump. */
  tableOnly?: boolean;
}

export interface BrowserSortOption extends SortingState {
  label: string;
}

export interface BrowserSort {
  options: readonly BrowserSortOption[];
  value: readonly SortingState[];
  onChange: (sorting: SortingState[]) => void;
}

const sortKey = (sort: SortingState) => `${sort.id}:${sort.desc ? "desc" : "asc"}`;

/** The ordering menu: one entry per field and direction ("Title", "Newest first"). */
function SortSelect({ options, value, onChange }: BrowserSort) {
  const current = value[0] ? sortKey(value[0]) : sortKey(options[0]!);
  return (
    <Select<string>
      aria-label="Sort by"
      size="sm"
      className="w-48 shrink-0"
      value={current}
      onValueChange={(next) => {
        const option = options.find((item) => sortKey(item) === next);
        if (option) onChange([{ id: option.id, desc: option.desc }]);
      }}
      options={options.map((option) => ({ value: sortKey(option), label: option.label }))}
    />
  );
}

/** Filter row shared by every library list: search, availability chips, sort, letter rail. */
export function BrowserToolbar({ query, onQueryChange, letter, onLetterChange, availability, onAvailabilityChange, sort, trailing, placeholder = "Filter", tableOnly = false }: BrowserToolbarProps) {
  const desktop = useIsDesktop();
  return (
    <div className="flex shrink-0 flex-col gap-2 sm:gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <SearchField aria-label={placeholder} value={query} onValueChange={onQueryChange} placeholder={placeholder} className="w-full sm:w-64" />
        {onAvailabilityChange ? (
          <GlassSegmented<AvailabilityFilter>
            ariaLabel="Availability"
            size="sm"
            value={availability ?? "all"}
            onChange={onAvailabilityChange}
            items={[
              { key: "all", label: "All" },
              { key: "available", label: "Available" },
              { key: "wanted", label: "Wanted" },
              { key: "missing", label: "Missing" },
            ]}
          />
        ) : null}
        <div className="flex-1" />
        {sort ? <SortSelect {...sort} /> : null}
        {trailing}
      </div>
      {!desktop && !tableOnly ? <AlphabetRail orientation="horizontal" value={letter} onChange={onLetterChange} /> : null}
    </div>
  );
}

export function BrowserLetterRail({ letter, onLetterChange }: { letter: string | null; onLetterChange: (letter: string | null) => void }) {
  const desktop = useIsDesktop();
  if (!desktop) return null;
  return <AlphabetRail value={letter} onChange={onLetterChange} className="min-h-0 shrink-0 self-stretch overscroll-contain" />;
}
