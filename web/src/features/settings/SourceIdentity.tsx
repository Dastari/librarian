import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityState } from "glass-ux/entity";
import { IconRss } from "@tabler/icons-react";

import { ENTITY_KINDS, glyph } from "@/lib/entities/kinds";

import type { HealthState } from "./sourceHealth";

const SourceGlyph = ENTITY_KINDS.source.icon;
const FeedGlyph = glyph(IconRss, "rss");

/** What a source's health says, as the state its identity shows: a dot and a badge in the row. */
export function sourceIdentityState(health: HealthState, enabled: boolean): EntityState {
  return { id: health.label.toLowerCase().replace(/[^a-z]+/g, "-").replace(/^-|-$/g, ""), label: health.label, tone: health.tone, paused: !enabled };
}

/**
 * An indexer or an RSS feed wherever it is named: its glyph, a state that says whether it works,
 * and a second line that is the reason while it fails and the address or sites it serves otherwise.
 */
export function SourceIdentity<T>({ kind, name, health, enabled, facts, target, actions, tier = "row", className }: {
  kind: "source" | "feed";
  name: string;
  health: HealthState;
  enabled: boolean;
  /** The second line while nothing is wrong. */
  facts: string;
  target?: T;
  actions?: ActionSet<T>;
  tier?: "row" | "header";
  className?: string;
}) {
  const failing = health.status === "error";
  return (
    <EntityIdentity<T>
      tier={tier}
      heading="h2"
      className={className}
      kind={kind === "feed" ? "RSS feed" : ENTITY_KINDS.source.noun}
      name={name}
      icon={kind === "feed" ? <FeedGlyph /> : <SourceGlyph />}
      state={sourceIdentityState(health, enabled)}
      facts={[failing ? health.detail : facts]}
      target={target}
      actions={actions}
    />
  );
}
