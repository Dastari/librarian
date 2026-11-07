import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityState } from "glass-ux/entity";

import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { formatRelative, pluralize } from "@/lib/format";
import { libraryType } from "@/lib/library-types";

export interface LibraryEntity {
  id: string;
  name: string;
  libraryType: string;
  scanning: boolean;
  lastScannedAt?: string | null;
}

/** What a library is doing, or whether it needs someone. Scanning outranks everything else. */
export function libraryState(library: LibraryEntity, issues = 0): EntityState {
  if (library.scanning) return { id: "scanning", label: "Scanning", tone: "info", activity: true };
  if (issues > 0) return { id: "issues", label: pluralize(issues, "issue"), tone: "warning" };
  return { id: "ready", label: library.lastScannedAt ? "Ready" : "Not scanned yet", tone: library.lastScannedAt ? "success" : "neutral" };
}

/**
 * A library wherever it is named: its type's glyph with a state dot, an issue count on the corner
 * of the glyph, then what it holds and when it was last scanned. Every size shows the same state.
 */
export function LibraryIdentity<T extends LibraryEntity>({ library, tier = "row", items, issues = 0, actions, heading, className, onOpen }: {
  library: T;
  tier?: "header" | "compact" | "row" | "chip" | "icon" | "auto";
  /** How many titles it holds, where that's known. */
  items?: number | null;
  /** Open scan issues, where they're known. */
  issues?: number;
  actions?: ActionSet<T>;
  heading?: "h1" | "h2" | "h3" | "h4" | "p";
  className?: string;
  onOpen?: () => void;
}) {
  const meta = libraryType(library.libraryType);
  return (
    <EntityIdentity
      tier={tier}
      heading={heading}
      className={className}
      kind={ENTITY_KINDS.library.noun}
      name={library.name}
      icon={<meta.icon />}
      state={libraryState(library, issues)}
      count={issues > 0 ? { value: issues, label: pluralize(issues, "open issue"), tone: "danger" } : undefined}
      facts={[
        items != null ? pluralize(items, meta.singular.toLowerCase()) : meta.label,
        library.lastScannedAt ? `Scanned ${formatRelative(library.lastScannedAt)}` : "",
      ]}
      target={library}
      actions={actions}
      onOpen={onOpen}
    />
  );
}
