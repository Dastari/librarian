import type { ActionDefinition, ActionSet } from "glass-ux/actions";

/** What a playlist entry's menu already does itself, so the target's own copy is left out. */
export const ENTRY_OWN_ACTIONS: ReadonlySet<string> = new Set(["play", "open", "play-next", "enqueue", "add-to-playlist", "reset-progress"]);

/**
 * A target's own actions (a movie's, an album's) as actions of the playlist entries that point at
 * them: each runs on the entries' targets, and shows only when every selected entry points at one
 * of that kind. The entry keeps its own Play from here, queue, move and remove.
 */
export function liftActions<Entry, Target>(set: ActionSet<Target>, pick: (entry: Entry) => Target | null | undefined, skip: ReadonlySet<string> = ENTRY_OWN_ACTIONS): ActionDefinition<Entry>[] {
  const targets = (rows: readonly Entry[]) => rows.flatMap((row) => { const target = pick(row); return target == null ? [] : [target]; });
  const applies = (rows: readonly Entry[]) => rows.length > 0 && targets(rows).length === rows.length;
  const lift = (action: ActionDefinition<Target>, prefix: string): ActionDefinition<Entry> => {
    const { label, visible, disabledReason, confirm, run, successMessage, children } = action;
    return {
      id: `${prefix}-${action.id}`,
      label: typeof label === "function" ? (rows) => label(targets(rows)) : label,
      icon: action.icon,
      group: action.group,
      bulk: action.bulk,
      surfaces: action.surfaces,
      visible: (rows) => applies(rows) && (visible?.(targets(rows)) ?? true),
      disabledReason: disabledReason ? (rows) => disabledReason(targets(rows)) : undefined,
      confirm: confirm ? (rows) => confirm(targets(rows)) : undefined,
      run: run ? (rows, context) => run(targets(rows), context) : undefined,
      successMessage: successMessage ? (rows) => successMessage(targets(rows)) : undefined,
      children: children?.map((child) => lift(child, `${prefix}-${action.id}`)),
    };
  };
  return set.actions.filter((action) => !skip.has(action.id)).map((action) => lift(action, set.entity));
}
