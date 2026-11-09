import { useMutation } from "@apollo/client/react";
import { useState } from "react";

import {
  EntityAlbumUpdateDocument,
  EntityArtistUpdateDocument,
  EntityAudiobookUpdateDocument,
  EntityEpisodeUpdateDocument,
  EntityMovieUpdateDocument,
  EntitySeasonUpdateDocument,
  EntityShowUpdateDocument,
  type ArtworkKind,
} from "@/graphql/generated/graphql";
import { assertSuccess } from "@/lib/graphql/errors";

import type { ArtworkTarget } from "./slots";

/** The lock name of a picture: its field's GraphQL name. */
export function artworkLockField(target: Pick<ArtworkTarget, "entityType">, kind: ArtworkKind): string | null {
  switch (target.entityType) {
    case "MOVIE":
    case "SHOW":
      return kind === "BACKDROP" ? "backdropUrl" : kind === "POSTER" ? "posterUrl" : null;
    case "SEASON":
      return "posterUrl";
    case "EPISODE":
      return "stillUrl";
    case "ALBUM":
    case "AUDIOBOOK":
      return "coverUrl";
    case "ARTIST":
      return "imageUrl";
    default:
      return null;
  }
}

const REFETCH = ["ArtworkOptions", "ArtworkSlots"];

/**
 * Unlock a picture so metadata refreshes may change it again: the entity's own generated update of
 * `lockedFields`, without the picture's field. The picture shown stays until the next refresh.
 */
export function useUnlockArtwork(target: ArtworkTarget) {
  const options = { refetchQueries: REFETCH };
  const [movie] = useMutation(EntityMovieUpdateDocument, options);
  const [show] = useMutation(EntityShowUpdateDocument, options);
  const [season] = useMutation(EntitySeasonUpdateDocument, options);
  const [episode] = useMutation(EntityEpisodeUpdateDocument, options);
  const [album] = useMutation(EntityAlbumUpdateDocument, options);
  const [artist] = useMutation(EntityArtistUpdateDocument, options);
  const [audiobook] = useMutation(EntityAudiobookUpdateDocument, options);
  const [unlocking, setUnlocking] = useState(false);

  const unlock = async (kind: ArtworkKind, lockedFields: readonly string[]) => {
    const field = artworkLockField(target, kind);
    if (!field) return;
    const variables = { id: target.entityId, input: { lockedFields: lockedFields.filter((name) => name !== field) } };
    const failed = "Could not unlock the picture";
    setUnlocking(true);
    try {
      switch (target.entityType) {
        case "MOVIE": assertSuccess((await movie({ variables })).data?.updateMovie, failed); break;
        case "SHOW": assertSuccess((await show({ variables })).data?.updateShow, failed); break;
        case "SEASON": assertSuccess((await season({ variables })).data?.updateSeason, failed); break;
        case "EPISODE": assertSuccess((await episode({ variables })).data?.updateEpisode, failed); break;
        case "ALBUM": assertSuccess((await album({ variables })).data?.updateAlbum, failed); break;
        case "ARTIST": assertSuccess((await artist({ variables })).data?.updateArtist, failed); break;
        case "AUDIOBOOK": assertSuccess((await audiobook({ variables })).data?.updateAudiobook, failed); break;
        default: break;
      }
    } finally {
      setUnlocking(false);
    }
  };
  return { unlock, unlocking };
}
