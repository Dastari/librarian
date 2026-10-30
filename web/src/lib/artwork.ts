import { artworkUrl } from "@/lib/api/urls";

/**
 * Artwork resolution per entity. Everything in the library is served from the local artwork
 * cache: the server downloads an entity's provider image on first request and keeps it in
 * object storage, so clients never hotlink provider CDNs. Provider URLs on the entity are
 * the cache's source, not something to load directly.
 */
export const moviePoster = (movieId: string) => artworkUrl("movie", movieId, "poster");
export const movieBackdrop = (movieId: string) => artworkUrl("movie", movieId, "backdrop");

export const showPoster = (show: { id: string }) => artworkUrl("show", show.id, "poster");
export const showBackdrop = (show: { id: string }) => artworkUrl("show", show.id, "backdrop");

/** A season poster when the provider has one; otherwise the show's poster stands in. */
export const seasonPoster = (season: { id: string; posterUrl?: string | null } | null | undefined, show: { id: string }) =>
  season?.posterUrl ? artworkUrl("season", season.id, "poster") : showPoster(show);

export const episodeThumb = (episodeId: string) => artworkUrl("episode", episodeId, "thumbnail");

export const albumCover = (album: { id: string }) => artworkUrl("album", album.id, "cover");
export const artistImage = (artist: { id: string }) => artworkUrl("artist", artist.id, "poster");
export const audiobookCover = (book: { id: string }) => artworkUrl("audiobook", book.id, "cover");

export const collectionPoster = (collection: { tmdbCollectionId: number }) => artworkUrl("collection", collection.tmdbCollectionId, "poster");
export const collectionBackdrop = (collection: { tmdbCollectionId: number }) => artworkUrl("collection", collection.tmdbCollectionId, "backdrop");

/** Cast portraits are cached like any artwork; people without one have no image. */
export const personProfile = (person: { id: string; profileUrl?: string | null } | null | undefined) =>
  person?.profileUrl ? artworkUrl("person", person.id, "profile") : undefined;
