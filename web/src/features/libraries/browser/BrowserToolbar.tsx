import { SearchField } from "@heroui/react";
import { IconSearch, IconX } from "@tabler/icons-react";
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
  total?: number;
}

/** Filter row shared by every library list: search, availability chips, letter rail, view. */
export function BrowserToolbar({ query, onQueryChange, letter, onLetterChange, availability, onAvailabilityChange, view, onViewChange, trailing, placeholder = "Filter", total }: BrowserToolbarProps) {
  const desktop = useIsDesktop();
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <SearchField aria-label={placeholder} value={query} onChange={onQueryChange} className="w-full sm:w-64" variant="secondary">
          <SearchField.Group>
            <SearchField.SearchIcon>
              <IconSearch size={16} />
            </SearchField.SearchIcon>
            <SearchField.Input placeholder={placeholder} />
            <SearchField.ClearButton>
              <IconX size={14} />
            </SearchField.ClearButton>
          </SearchField.Group>
        </SearchField>
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
        {typeof total === "number" ? <span className="text-numeric text-label text-muted">{total.toLocaleString()}</span> : null}
        {trailing}
        <ViewToggle value={view} onChange={onViewChange} />
      </div>
      {!desktop ? <AlphabetRail orientation="horizontal" value={letter} onChange={onLetterChange} /> : null}
    </div>
  );
}

export function BrowserLetterRail({ letter, onLetterChange }: { letter: string | null; onLetterChange: (letter: string | null) => void }) {
  const desktop = useIsDesktop();
  if (!desktop) return null;
  return <AlphabetRail value={letter} onChange={onLetterChange} className="sticky top-4 shrink-0 self-start" />;
}
