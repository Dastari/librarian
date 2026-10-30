import { SearchField } from "glass-ux/input";
import type { ReactNode } from "react";

import { AlphabetRail, GlassSegmented, ViewToggle, type ViewMode } from "@/components/ui";
import { useIsDesktop } from "@/hooks/useMediaQuery";

export type AvailabilityFilter = "all" | "available" | "wanted" | "missing";

interface BrowserToolbarProps {
  query: string;
  onQueryChange: (value: string) => void;
  letter: string | null;
  onLetterChange: (letter: string | null) => void;
  availability?: AvailabilityFilter;
  onAvailabilityChange?: (filter: AvailabilityFilter) => void;
  view: ViewMode;
  onViewChange: (view: ViewMode) => void;
  /** Extra controls (sort menu, add button). */
  trailing?: ReactNode;
  placeholder?: string;
  /** The table footer reports the count; kept so callers can pass it without a second display. */
  total?: number;
  /** Table-only views have no gallery toggle or alphabet jump. */
  tableOnly?: boolean;
}

/** Filter row shared by every library list: search, availability chips, letter rail, view. */
export function BrowserToolbar({ query, onQueryChange, letter, onLetterChange, availability, onAvailabilityChange, view, onViewChange, trailing, placeholder = "Filter", tableOnly = false }: BrowserToolbarProps) {
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
        {trailing}
        {!tableOnly ? <ViewToggle value={view} onChange={onViewChange} /> : null}
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
