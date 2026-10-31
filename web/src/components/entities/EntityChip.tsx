import { EntityIdentity } from "glass-ux/entity";

import { ENTITY_KINDS, type EntityKindName } from "@/lib/entities/kinds";
import type { LinkedRef } from "@/lib/entities/linked";

/**
 * A linked record as a chip: the kind's glyph, its name, and its page when it has one. The
 * same chip names a movie, a file or a download on every table and detail pane.
 */
export function EntityChip({ entity, className }: { entity: LinkedRef; className?: string }) {
  const spec = ENTITY_KINDS[entity.kind as EntityKindName];
  const Glyph = spec?.icon;
  return <EntityIdentity tier="chip" size="sm" className={className} kind={spec?.noun} name={entity.label} href={entity.href} icon={Glyph ? <Glyph /> : undefined} preview={false} />;
}
