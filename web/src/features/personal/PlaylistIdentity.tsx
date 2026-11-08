import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityIdentityProps, type EntityState, type EntityTierProp } from "glass-ux/entity";
import { useState, type ReactNode } from "react";

import type { PersonalListKind, PlaylistSummaryFragment } from "@/graphql/generated/graphql";
import { sizedArtwork } from "@/lib/api/urls";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { pluralize } from "@/lib/format";

import { playlistCovers } from "./playlistArt";

export type PlaylistEntity = Pick<PlaylistSummaryFragment, "id" | "name" | "kind" | "description"> & Partial<Pick<PlaylistSummaryFragment, "entries">>;

const Glyph = ENTITY_KINDS.playlist.icon;

/** What kind of list this is, in the words a person uses. */
export const playlistKindLabel = (kind: PersonalListKind) => (kind === "SMART" ? "Smart collection" : kind === "WATCH_LATER" ? "Watch Later" : "Playlist");

/** Smart collections and Watch Later say what they are; a playlist says whether it has anything, or is on Home. */
export function playlistState(list: PlaylistEntity, { pinned = false }: { pinned?: boolean } = {}): EntityState {
  const count = list.entries?.pageInfo.totalCount ?? null;
  if (list.kind === "SMART") return { id: "smart", label: "Smart collection", tone: "info" };
  if (pinned) return { id: "pinned", label: "On Home", tone: "info" };
  if (count === 0) return { id: "empty", label: "Empty", tone: "neutral" };
  return { id: list.kind === "WATCH_LATER" ? "watch-later" : "ready", label: count === null ? "Playlist" : pluralize(count, "item"), tone: "success" };
}

/** A playlist's picture: up to four covers from its entries in one tile, or the playlist glyph while there are none. */
function PlaylistArt({ covers, size, fallback }: { covers: readonly string[]; size: number; fallback: ReactNode }) {
  const [failed, setFailed] = useState<readonly string[]>([]);
  const shown = covers.filter((cover) => !failed.includes(cover));
  if (!shown.length) return <>{fallback}</>;
  return (
    <span className="entity-art" data-shape="square" data-mosaic={shown.length > 1 ? shown.length : undefined}>
      {shown.map((cover) => <img key={cover} src={sizedArtwork(cover, size)} alt="" loading="lazy" decoding="async" draggable={false} onError={() => setFailed((previous) => [...previous, cover])} />)}
    </span>
  );
}

/** What PlaylistIdentity draws, for surfaces that take an identity's props (a settings dialog's header). */
export function playlistIdentityProps<T extends PlaylistEntity>(list: T, { actions, pinned, duration }: {
  actions?: ActionSet<T>;
  /** It's a shelf on Home. */
  pinned?: boolean;
  /** How long it runs, where its entries are loaded: "3h 20m". */
  duration?: string;
} = {}): EntityIdentityProps<T> {
  const covers = playlistCovers(list.entries?.edges.map((edge) => edge.node));
  const count = list.entries?.pageInfo.totalCount;
  return {
    kind: ENTITY_KINDS.playlist.noun,
    name: list.name,
    icon: <PlaylistArt covers={covers} size={96} fallback={<Glyph />} />,
    image: <PlaylistArt covers={covers} size={240} fallback={<span className="grid size-full place-items-center bg-surface-secondary text-muted [&>svg]:size-1/2" aria-hidden><Glyph /></span>} />,
    state: playlistState(list, { pinned }),
    facts: [playlistKindLabel(list.kind), list.kind === "SMART" || count == null ? "" : pluralize(count, "item"), duration ?? ""],
    target: list,
    actions,
  };
}

/** A playlist wherever it is named: its picture with a state dot, then what kind of list it is, how many and how long. */
export function PlaylistIdentity<T extends PlaylistEntity>({ list, tier = "row", actions, pinned, duration, headerActions, heading, className, onOpen }: {
  list: T;
  tier?: EntityTierProp;
  actions?: ActionSet<T>;
  pinned?: boolean;
  duration?: string;
  headerActions?: ReactNode;
  heading?: "h1" | "h2" | "h3" | "h4" | "p";
  className?: string;
  onOpen?: () => void;
}) {
  return (
    <EntityIdentity
      {...playlistIdentityProps(list, { actions, pinned, duration })}
      tier={tier}
      heading={heading}
      className={className}
      slots={headerActions ? { actions: headerActions } : undefined}
      onOpen={onOpen}
    />
  );
}
