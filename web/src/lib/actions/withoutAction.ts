import { defineActions, type ActionSet } from "glass-ux/actions";

/**
 * The same actions less some: inside an entity's settings dialog, Settings would reopen it and
 * Remove is a tab of its own.
 */
export function withoutAction<T>(set: ActionSet<T>, ...ids: string[]): ActionSet<T> {
  return defineActions(set.entity, set.actions.filter((action) => !ids.includes(action.id)));
}
