import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityIdentityProps, type EntityState } from "glass-ux/entity";
import type { ReactNode } from "react";

import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { formatRelative, pluralize } from "@/lib/format";
import { libraryType } from "@/lib/library-types";

import { libraryWatch, type LibraryWatchFields } from "./libraryWatch";

export interface LibraryEntity {
  id: string;
  name: string;
  libraryType: string;
  scanning: boolean;
  lastScannedAt?: string | null;
  path?: string;
  watchForChanges?: boolean;
  watchStatus?: string | null;
  watchStatusSince?: string | null;
}

/** What a library is doing, or whether it needs someone. Scanning outranks everything else. */
export function libraryState(library: LibraryEntity, issues = 0): EntityState {
  if (library.scanning) return { id: "scanning", label: "Scanning", tone: "info", activity: true };
  if (issues > 0) return { id: "issues", label: pluralize(issues, "issue"), tone: "warning" };
  if (library.watchForChanges && library.path !== undefined) {
    const watch = libraryWatch(library as LibraryWatchFields);
    if (watch.id === "unavailable" || watch.id === "failed") return { id: `watch-${watch.id}`, label: watch.label, tone: "warning" };
  }
  return { id: "ready", label: library.lastScannedAt ? "Ready" : "Not scanned yet", tone: library.lastScannedAt ? "success" : "neutral" };
}

/** What LibraryIdentity draws, for surfaces that take an identity's props (a settings dialog's header). */
export function libraryIdentityProps<T extends LibraryEntity>(library: T, { items, issues = 0, actions }: {
  /** How many titles it holds, where that's known. */
  items?: number | null;
  /** Open scan issues, where they're known. */
  issues?: number;
  actions?: ActionSet<T>;
} = {}): EntityIdentityProps<T> {
  const meta = libraryType(library.libraryType);
  return {
    kind: ENTITY_KINDS.library.noun,
    name: library.name,
    icon: <meta.icon />,
    state: libraryState(library, issues),
    count: issues > 0 ? { value: issues, label: pluralize(issues, "open issue"), tone: "danger" } : undefined,
    facts: [
      items != null ? pluralize(items, meta.singular.toLowerCase()) : meta.label,
      library.lastScannedAt ? `Scanned ${formatRelative(library.lastScannedAt)}` : "",
    ],
    target: library,
    actions,
  };
}

/**
 * A library wherever it is named: its type's glyph with a state dot, an issue count on the corner
 * of the glyph, then what it holds and when it was last scanned. Every size shows the same state.
 */
export function LibraryIdentity<T extends LibraryEntity>({ library, tier = "row", items, issues = 0, actions, headerActions, heading, className, onOpen }: {
  library: T;
  tier?: "header" | "compact" | "row" | "chip" | "icon" | "auto";
  /** How many titles it holds, where that's known. */
  items?: number | null;
  /** Open scan issues, where they're known. */
  issues?: number;
  actions?: ActionSet<T>;
  /** What sits at the end of the header, in place of its own action buttons. */
  headerActions?: ReactNode;
  heading?: "h1" | "h2" | "h3" | "h4" | "p";
  className?: string;
  onOpen?: () => void;
}) {
  return (
    <EntityIdentity
      {...libraryIdentityProps(library, { items, issues, actions })}
      tier={tier}
      heading={heading}
      className={className}
      slots={headerActions ? { actions: headerActions } : undefined}
      onOpen={onOpen}
    />
  );
}
