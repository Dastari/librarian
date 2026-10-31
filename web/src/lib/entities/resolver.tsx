import { defineActions, type ActionSet } from "glass-ux/actions";
import type { EntityKind, EntityRef, EntityResolver } from "glass-ux/ai";
import { IconExternalLink } from "glass-ux/icons";

import { ENTITY_KINDS, entityPath, type EntityKindName } from "./kinds";
import type { LinkedRef } from "./linked";

/** A reference's page: the href it was built with, else its kind's route. */
export function refHref(ref: EntityRef): string | undefined {
  return (ref as LinkedRef).href ?? (ref.kind in ENTITY_KINDS ? entityPath(ref.kind as EntityKindName, ref.id) : undefined);
}

/**
 * The app's entity resolver (docs/entity-tables.md): every chip and link below the root
 * reads its kind's glyph and nouns, opens the record's page, previews what its reference
 * carries, and offers the kind's shared actions.
 */
export function createEntityResolver(navigate: (href: string) => void): EntityResolver {
  const actionSets = new Map<EntityKind, ActionSet<EntityRef>>();
  const actionsFor = (kind: EntityKind) => {
    let set = actionSets.get(kind);
    if (!set) {
      const noun = ENTITY_KINDS[kind as EntityKindName]?.noun.toLowerCase() ?? "item";
      set = defineActions<EntityRef>(`entity-${kind}`, [
        { id: "open", label: `Open ${noun}`, icon: IconExternalLink, group: "open", priority: 1, visible: (refs) => refs.length === 1 && Boolean(refHref(refs[0]!)), run: (refs) => { const href = refHref(refs[0]!); if (href) navigate(href); } },
      ]);
      actionSets.set(kind, set);
    }
    return set;
  };
  return {
    kinds: ENTITY_KINDS,
    preview: (ref) => ({
      subtitle: [ENTITY_KINDS[ref.kind as EntityKindName]?.noun, ref.meta].filter(Boolean).join(" · ") || undefined,
      facts: ref.state ? [{ label: "Status", value: ref.state.label }] : undefined,
    }),
    actions: actionsFor,
    open: (ref) => { const href = refHref(ref); if (href) navigate(href); },
    openWindow: (ref) => { const href = refHref(ref); if (href) window.open(href, "_blank", "noopener"); },
  };
}
