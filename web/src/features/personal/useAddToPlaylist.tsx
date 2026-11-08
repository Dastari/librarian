import type { ActionDefinition } from "glass-ux/actions";
import { IconBookmark } from "glass-ux/icons";
import { useMemo, useRef, useState, type ReactNode } from "react";

import { AddToPlaylistDialog } from "./AddToPlaylistDialog";
import type { PlaylistTarget } from "./types";

interface Options<T> {
  /** What the rows stand for. */
  targetsOf: (rows: readonly T[]) => PlaylistTarget[];
  /** How the dialog names them: "Heat", "3 tracks". */
  titleOf: (rows: readonly T[]) => string;
  /** Hide the action where these rows aren't the person's own. */
  allowed?: (rows: readonly T[]) => boolean;
  /** One row at a time, or from a table's bulk bar too. */
  bulk?: boolean;
  priority?: number;
}

/**
 * The "Add to playlist…" action for any entity's action set, with the dialog it opens. Put
 * `action` among the set's definitions and render `dialog` beside the component. The action's
 * identity never changes, so it doesn't rebuild the set it's in.
 */
export function useAddToPlaylist<T>(options: Options<T>): { action: ActionDefinition<T>; dialog: ReactNode } {
  const [rows, setRows] = useState<readonly T[] | null>(null);
  const latest = useRef(options);
  latest.current = options;
  const action = useMemo<ActionDefinition<T>>(() => ({
    id: "add-to-playlist", label: "Add to playlist…", icon: IconBookmark, group: "organise", priority: latest.current.priority, bulk: latest.current.bulk,
    visible: (targets) => targets.length > 0 && (latest.current.allowed?.(targets) ?? true),
    run: (targets) => setRows(targets),
  }), []);
  const dialog = rows ? <AddToPlaylistDialog open onOpenChange={(next) => { if (!next) setRows(null); }} targets={options.targetsOf(rows)} title={options.titleOf(rows)} /> : null;
  return { action, dialog };
}

export const playlistTitle = (rows: readonly unknown[], one: string, noun: { one: string; other: string }) => rows.length === 1 ? one : `${rows.length} ${noun.other}`;
