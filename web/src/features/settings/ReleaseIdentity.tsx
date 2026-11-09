import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityState } from "glass-ux/entity";

import { ENTITY_KINDS } from "@/lib/entities/kinds";

const ReleaseGlyph = ENTITY_KINDS.torrent.icon;

/**
 * A release as an indexer or feed lists it: the title as given, what it is on the second line.
 * Feed items and blocked releases draw the same identity, so a title reads alike in both tables.
 */
export function ReleaseIdentity<T>({ title, facts, state, target, actions, tier = "row", className }: {
  title: string;
  facts?: string;
  state?: EntityState;
  target?: T;
  actions?: ActionSet<T>;
  tier?: "row" | "header";
  className?: string;
}) {
  return (
    <EntityIdentity<T>
      tier={tier}
      heading="h2"
      className={className}
      kind="Release"
      name={title}
      icon={<ReleaseGlyph />}
      state={state}
      facts={facts ? [facts] : []}
      target={target}
      actions={actions}
    />
  );
}
