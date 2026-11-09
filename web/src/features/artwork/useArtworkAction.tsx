import type { ActionDefinition } from "glass-ux/actions";
import { IconImage } from "glass-ux/icons";
import { useMemo, useRef, useState, type ReactNode } from "react";

import type { ArtworkKind } from "@/graphql/generated/graphql";

import { ArtworkPickerDialog } from "./ArtworkPickerDialog";
import type { ArtworkTarget } from "./slots";

interface Options<T> {
  /** What a row is, for the picker. */
  targetOf: (row: T) => ArtworkTarget;
  /** Who may change it: an admin, or a playlist's owner. */
  allowed: (row: T) => boolean;
  priority?: number;
}

/**
 * "Change artwork…" for any entity's action set, with the picker it opens. Put `action` in the
 * set and render `dialog` beside it; `open` starts the picker at one picture (a settings tab's
 * "Change…" buttons). The action's identity never changes, so it doesn't rebuild its set.
 */
export function useArtworkAction<T>(options: Options<T>): { action: ActionDefinition<T>; dialog: ReactNode; open: (row: T, kind?: ArtworkKind) => void } {
  const [picking, setPicking] = useState<{ target: ArtworkTarget; kind?: ArtworkKind } | null>(null);
  const latest = useRef(options);
  latest.current = options;
  const open = useMemo(() => (row: T, kind?: ArtworkKind) => setPicking({ target: latest.current.targetOf(row), kind }), []);
  const action = useMemo<ActionDefinition<T>>(() => ({
    id: "artwork", label: "Change artwork…", icon: IconImage, group: "manage", priority: latest.current.priority ?? 5,
    visible: (rows) => rows.length === 1 && latest.current.allowed(rows[0]!),
    run: (rows) => open(rows[0]!),
  }), [open]);
  const dialog = picking ? (
    <ArtworkPickerDialog open target={picking.target} initialKind={picking.kind} onOpenChange={(next) => { if (!next) setPicking(null); }} />
  ) : null;
  return { action, dialog, open };
}
