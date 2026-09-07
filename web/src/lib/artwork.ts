import { artworkUrl } from "@/lib/api/urls";

/**
 * Artwork resolution per entity. Movies and collections are served from the artwork cache;
 * shows, albums, artists and audiobooks carry provider URLs on the entity and fall back to the
 * cache when those are missing.
 */
export const moviePoster = (movieId: string) => artworkUrl("movie", movieId, "poster");
export const movieBackdrop = (movieId: string) => artworkUrl("movie", movieId, "backdrop");

export const showPoster = (show: { id: string; posterUrl?: string | null }) => show.posterUrl ?? artworkUrl("show", show.id, "poster");
export const showBackdrop = (show: { id: string; backdropUrl?: string | null }) => show.backdropUrl ?? artworkUrl("show", show.id, "backdrop");

export const episodeThumb = (episodeId: string) => artworkUrl("episode", episodeId, "thumbnail");

export const albumCover = (album: { id: string; coverUrl?: string | null }) => album.coverUrl ?? artworkUrl("album", album.id, "cover");
export const artistImage = (artist: { id: string; imageUrl?: string | null }) => artist.imageUrl ?? artworkUrl("artist", artist.id, "poster");
export const audiobookCover = (book: { id: string; coverUrl?: string | null }) => book.coverUrl ?? artworkUrl("audiobook", book.id, "cover");

export const collectionPoster = (collection: { tmdbCollectionId: number; posterUrl?: string | null }) =>
  collection.posterUrl ?? artworkUrl("collection", collection.tmdbCollectionId, "poster");
export const collectionBackdrop = (collection: { tmdbCollectionId: number; backdropUrl?: string | null }) =>
  collection.backdropUrl ?? artworkUrl("collection", collection.tmdbCollectionId, "backdrop");

export const personProfile = (person: { profileUrl?: string | null }) => person.profileUrl ?? undefined;
