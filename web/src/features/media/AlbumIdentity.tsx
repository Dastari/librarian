import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityIdentityProps, type EntityState, type EntityTierProp } from "glass-ux/entity";
import type { ReactNode } from "react";

import { autoDownloadMeta } from "@/features/acquisition/mode";
import type { AutoDownloadMode } from "@/graphql/generated/graphql";
import { albumCover } from "@/lib/artwork";
import { sizedArtwork } from "@/lib/api/urls";
import { ArtGlyph, ArtImage } from "@/components/entities/ArtGlyph";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { pluralize } from "@/lib/format";

export interface AlbumEntity {
  id: string;
  name: string;
  libraryId: string;
  userId: string;
  year?: number | null;
  albumType?: string | null;
  trackCount?: number | null;
  /** Its artist, where known: music recommendations are the artist's plan. */
  artistId?: string;
  hasFiles: boolean;
  autoDownloadMode: AutoDownloadMode;
}

/** Whether its files are here, or else whether they're being fetched. */
export function albumState(album: AlbumEntity): EntityState {
  if (album.hasFiles) return { id: "available", label: "Available", tone: "success" };
  const mode = autoDownloadMeta(album.autoDownloadMode);
  return album.autoDownloadMode === "NONE" ? { id: "missing", label: "No files yet", tone: "neutral" } : { id: "fetching", label: mode.label, tone: "info" };
}

/** What AlbumIdentity draws, for surfaces that take an identity's props (a settings dialog's header). */
export function albumIdentityProps<T extends AlbumEntity>(album: T, { actions }: {
  actions?: ActionSet<T>;
} = {}): EntityIdentityProps<T> {
  return {
    kind: ENTITY_KINDS.album.noun,
    name: album.name,
    icon: <ArtGlyph src={sizedArtwork(albumCover(album), 96)} fallback={<ENTITY_KINDS.album.icon />} shape="square" />,
    image: <ArtImage src={sizedArtwork(albumCover(album), 120)} fallback={<ENTITY_KINDS.album.icon />} />,
    state: albumState(album),
    facts: [album.year ? String(album.year) : "", album.albumType ?? "", album.trackCount ? pluralize(album.trackCount, "track") : ""],
    target: album,
    actions,
  };
}

/** An album wherever it's named in a list or a heading: its cover or glyph, year, type and tracks. */
export function AlbumIdentity<T extends AlbumEntity>({ album, tier = "row", actions, headerActions, heading, className, onOpen }: {
  album: T;
  tier?: EntityTierProp;
  actions?: ActionSet<T>;
  headerActions?: ReactNode;
  heading?: "h1" | "h2" | "h3" | "h4" | "p";
  className?: string;
  onOpen?: () => void;
}) {
  return (
    <EntityIdentity
      {...albumIdentityProps(album, { actions })}
      tier={tier}
      heading={heading}
      className={className}
      slots={headerActions ? { actions: headerActions } : undefined}
      onOpen={onOpen}
    />
  );
}
