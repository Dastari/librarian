import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityTierProp } from "glass-ux/entity";
import type { ReactNode } from "react";

import { moviePoster } from "@/lib/artwork";
import { sizedArtwork } from "@/lib/api/urls";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { formatRuntime } from "@/lib/format";
import { statusMeta, type StatusMeta } from "@/lib/status";

import { mediaEntityState } from "./entity/mediaState";

export interface MovieEntity {
  id: string;
  title: string;
  libraryId: string;
  userId: string;
  year?: number | null;
  runtime?: number | null;
  certification?: string | null;
  imdbId?: string | null;
  mediaFileId?: string | null;
  hasFile: boolean;
  wanted: boolean;
  monitored: boolean;
}

/** Whether it's here, wanted or neither, for wherever the live status isn't loaded. */
const statusOf = (movie: MovieEntity): StatusMeta => statusMeta(movie.hasFile ? "AVAILABLE" : movie.wanted ? "WANTED" : "MISSING");

/**
 * A movie wherever it's named in a list or a heading: its poster or glyph with a state dot, then
 * its year, length and rating. Poster grids draw posters instead, with the same state and actions.
 */
export function MovieIdentity<T extends MovieEntity>({ movie, tier = "row", status, actions, headerActions, heading, className, onOpen }: {
  movie: T;
  tier?: EntityTierProp;
  /** Its live status, where it's loaded. */
  status?: StatusMeta;
  actions?: ActionSet<T>;
  /** What sits at the end of the header, in place of its own action buttons. */
  headerActions?: ReactNode;
  heading?: "h1" | "h2" | "h3" | "h4" | "p";
  className?: string;
  onOpen?: () => void;
}) {
  return (
    <EntityIdentity
      tier={tier}
      heading={heading}
      className={className}
      kind={ENTITY_KINDS.movie.noun}
      name={movie.title}
      icon={<ENTITY_KINDS.movie.icon />}
      image={<img src={sizedArtwork(moviePoster(movie.id), 120)} alt="" />}
      state={mediaEntityState(status ?? statusOf(movie))}
      facts={[movie.year ? String(movie.year) : "", movie.runtime ? formatRuntime(movie.runtime, "minutes") : "", movie.certification ?? ""]}
      target={movie}
      actions={actions}
      slots={headerActions ? { actions: headerActions } : undefined}
      onOpen={onOpen}
    />
  );
}
