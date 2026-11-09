import { useQuery } from "@apollo/client/react";
import { useState } from "react";

import { ArtworkSlotsDocument } from "@/graphql/generated/graphql";
import { artworkUrl, sizedArtwork } from "@/lib/api/urls";
import { useArtworkRevisions } from "@/lib/artworkRevisions";

/**
 * A movie's title in its hero: the logo someone chose for it, or the title as text. The logo is
 * the title's picture, so it carries the title as its text alternative.
 */
export function MovieTitle({ movieId, title }: { movieId: string; title: string }) {
  useArtworkRevisions();
  const slots = useQuery(ArtworkSlotsDocument, { variables: { entityType: "MOVIE", entityId: movieId }, fetchPolicy: "cache-first" });
  const [failed, setFailed] = useState(false);
  const logo = slots.data?.artworkSlots.find((slot) => slot.kind === "LOGO");
  if (!logo?.hasArtwork || failed) return <>{title}</>;
  return (
    <img src={sizedArtwork(artworkUrl("movie", movieId, "logo"), 640)} alt={title} draggable={false} onError={() => setFailed(true)}
      className="block h-auto max-h-[min(9rem,22vh)] w-auto max-w-[min(32rem,100%)] object-contain object-left" />
  );
}
