import type { ApolloClient } from "@apollo/client";

import { ArtistDetailDocument, EntityAlbumGetDocument, EpisodeDetailDocument } from "@/graphql/generated/graphql";

import { albumPlayItems, audiobookItems, showPlayItems } from "./playables";
import type { PlayItem } from "./store";

/**
 * What plays when the queue runs out. The rule is deliberately plain, and looks only at the last
 * entry that played:
 *
 * - a **track** carries on through the rest of its album, then into the artist's next album (by
 *   release year, skipping albums with no files);
 * - a **chapter** carries on through the rest of its book;
 * - an **episode** carries on through the rest of its show, later seasons included;
 * - a **movie** has nothing after it, so playback stops.
 *
 * Whatever is found is added to the end of the queue like any other entries, so it can be
 * reordered or cleared. When nothing is found the queue is left alone and playback stops.
 */
export async function continueAfter(client: ApolloClient, last: PlayItem): Promise<PlayItem[]> {
  switch (last.entity.kind) {
    case "track": {
      const albumId = last.albumId;
      if (!albumId) return [];
      const rest = after(await albumPlayItems(client, albumId), last);
      return rest.length ? rest : nextAlbumItems(client, albumId);
    }
    case "chapter":
      return last.audiobookId ? after(await audiobookItems(client, last.audiobookId), last) : [];
    case "episode": {
      let showId = last.showId;
      if (!showId) {
        const { data } = await client.query({ query: EpisodeDetailDocument, variables: { id: last.entity.id }, fetchPolicy: "no-cache" });
        showId = data?.episode?.showId;
      }
      return showId ? after(await showPlayItems(client, showId), last) : [];
    }
    case "movie":
      return [];
  }
}

/** The entries after `last`, or none when `last` is no longer among them (the media changed under it). */
export function after(items: readonly PlayItem[], last: PlayItem): PlayItem[] {
  const at = items.findIndex((item) => item.entity.id === last.entity.id && item.mediaFileId === last.mediaFileId);
  return at < 0 ? [] : items.slice(at + 1);
}

/** The first of the artist's later albums that has anything to play. */
async function nextAlbumItems(client: ApolloClient, albumId: string): Promise<PlayItem[]> {
  const album = (await client.query({ query: EntityAlbumGetDocument, variables: { id: albumId }, fetchPolicy: "no-cache" })).data?.album;
  if (!album) return [];
  const artist = (await client.query({ query: ArtistDetailDocument, variables: { id: album.artistId, page: { limit: 200, offset: 0 } }, fetchPolicy: "no-cache" })).data?.artist;
  if (!artist) return [];
  const chronological = artist.albums.edges
    .map((edge) => edge.node)
    .sort((a, b) => (a.year ?? Number.MAX_SAFE_INTEGER) - (b.year ?? Number.MAX_SAFE_INTEGER) || (a.releaseDate ?? "").localeCompare(b.releaseDate ?? "") || a.name.localeCompare(b.name));
  const here = chronological.findIndex((candidate) => candidate.id === albumId);
  if (here < 0) return [];
  const later = chronological.slice(here + 1).filter((candidate) => candidate.hasFiles);
  for (const candidate of later.slice(0, 5)) {
    const items = await albumPlayItems(client, candidate.id);
    if (items.length) return items;
  }
  return [];
}
