import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityIdentityProps, type EntityState, type EntityTierProp } from "glass-ux/entity";
import type { ReactNode } from "react";

import { autoDownloadMeta } from "@/features/acquisition/mode";
import type { AutoDownloadMode } from "@/graphql/generated/graphql";
import { showPoster } from "@/lib/artwork";
import { sizedArtwork } from "@/lib/api/urls";
import { ArtGlyph } from "@/components/entities/ArtGlyph";
import { ENTITY_KINDS } from "@/lib/entities/kinds";

export interface ShowEntity {
  id: string;
  name: string;
  libraryId: string;
  userId: string;
  year?: number | null;
  network?: string | null;
  status?: string | null;
  imdbId?: string | null;
  autoDownloadMode: AutoDownloadMode;
}

/** How much of it is here, or else whether it's being fetched. */
export function showState(show: ShowEntity, have?: number, total?: number): EntityState {
  if (have !== undefined && total !== undefined && total > 0) {
    if (have >= total) return { id: "complete", label: "Complete", tone: "success" };
    if (have > 0) return { id: "partial", label: `${have} of ${total} episodes`, tone: "info" };
  }
  const mode = autoDownloadMeta(show.autoDownloadMode);
  return show.autoDownloadMode === "NONE" ? { id: "manual", label: "Not fetched automatically", tone: "neutral" } : { id: "fetching", label: mode.label, tone: "info" };
}

/** What ShowIdentity draws, for surfaces that take an identity's props (a settings dialog's header). */
export function showIdentityProps<T extends ShowEntity>(show: T, { actions, have, total }: {
  have?: number;
  total?: number;
  actions?: ActionSet<T>;
} = {}): EntityIdentityProps<T> {
  return {
    kind: ENTITY_KINDS.show.noun,
    name: show.name,
    icon: <ArtGlyph src={sizedArtwork(showPoster(show), 96)} fallback={<ENTITY_KINDS.show.icon />} shape="poster" />,
    image: <img src={sizedArtwork(showPoster(show), 120)} alt="" />,
    state: showState(show, have, total),
    facts: [show.year ? String(show.year) : "", show.network ?? "", show.status ?? ""],
    target: show,
    actions,
  };
}

/** A show wherever it's named in a list or a heading, with how much of it is here. */
export function ShowIdentity<T extends ShowEntity>({ show, tier = "row", have, total, actions, headerActions, heading, className, onOpen }: {
  show: T;
  tier?: EntityTierProp;
  have?: number;
  total?: number;
  actions?: ActionSet<T>;
  headerActions?: ReactNode;
  heading?: "h1" | "h2" | "h3" | "h4" | "p";
  className?: string;
  onOpen?: () => void;
}) {
  return (
    <EntityIdentity
      {...showIdentityProps(show, { actions, have, total })}
      tier={tier}
      heading={heading}
      className={className}
      slots={headerActions ? { actions: headerActions } : undefined}
      onOpen={onOpen}
    />
  );
}
