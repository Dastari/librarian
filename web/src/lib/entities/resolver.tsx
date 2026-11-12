import { defineActions, type ActionSet } from "glass-ux/actions";
import type { EntityKind, EntityRef, EntityResolver } from "glass-ux/ai";
import type { DataTableCellEntity } from "glass-ux/data-table";
import { IconExternalLink } from "glass-ux/icons";

import { openFileDialog } from "@/features/files/fileDialog";

import { ENTITY_KINDS, entityPath, type EntityKindName } from "./kinds";
import type { LinkedRef } from "./linked";

/** A reference's page: the href it was built with, else its kind's route. */
export function refHref(ref: EntityRef): string | undefined {
  return (ref as LinkedRef).href ?? (ref.kind in ENTITY_KINDS ? entityPath(ref.kind as EntityKindName, ref.id) : undefined);
}

let navigateTo: (href: string) => void = (href) => window.location.assign(href);

/** Opens a reference: a file in its dialog, anything else at its page. */
function openRef(ref: EntityRef) {
  if (ref.kind === "mediaFile") return openFileDialog(ref.id);
  const href = refHref(ref);
  if (href) navigateTo(href);
}

const actionSets = new Map<EntityKind, ActionSet<EntityRef>>();

/** A kind's shared actions on a reference, where only its id and name are loaded. */
export function refActions(kind: EntityKind): ActionSet<EntityRef> {
  let set = actionSets.get(kind);
  if (!set) {
    const noun = ENTITY_KINDS[kind as EntityKindName]?.noun.toLowerCase() ?? "item";
    set = defineActions<EntityRef>(`entity-${kind}`, [
      { id: "open", label: kind === "mediaFile" ? "File details" : `Open ${noun}`, icon: IconExternalLink, group: "open", priority: 1, visible: (refs) => refs.length === 1 && (refs[0]!.kind === "mediaFile" || Boolean(refHref(refs[0]!))), run: (refs) => openRef(refs[0]!) },
    ]);
    actionSets.set(kind, set);
  }
  return set;
}

/** A table cell that shows a linked record: right-click there offers that record's actions. */
export function refCellEntity(ref: LinkedRef | null | undefined): DataTableCellEntity<EntityRef> | null {
  return ref ? { name: ref.label, actions: refActions(ref.kind), target: ref } : null;
}

/**
 * The app's entity resolver (docs/entity-tables.md): every chip and link below the root
 * reads its kind's glyph and nouns, opens the record's page, previews what its reference
 * carries, and offers the kind's shared actions.
 */
export function createEntityResolver(navigate: (href: string) => void): EntityResolver {
  navigateTo = navigate;
  return {
    kinds: ENTITY_KINDS,
    preview: (ref) => ({
      subtitle: [ENTITY_KINDS[ref.kind as EntityKindName]?.noun, ref.meta].filter(Boolean).join(" · ") || undefined,
      facts: ref.state ? [{ label: "Status", value: ref.state.label }] : undefined,
    }),
    actions: refActions,
    open: openRef,
    openWindow: (ref) => { const href = refHref(ref); if (href) window.open(href, "_blank", "noopener"); },
  };
}
