import type { ApolloClient } from "@apollo/client";

import {
  EntityListEntryCreateDocument,
  EntityPersonalListCreateDocument,
  PersonalEntriesDocument,
  type PersonalEntryFragment,
} from "@/graphql/generated/graphql";
import { assertSuccess } from "@/lib/graphql/errors";

import { audiobookItems, albumPlayItems, showPlayItems } from "./playables";
import { moviePlayItem, trackPlayItem } from "./playItems";
import type { PlayItem } from "./store";

/**
 * The play queue's only contact with playlists (personal lists). It is a seam: everything the
 * queue needs from playlists goes through `playlistAdapter`, so when playlists gain their own
 * helpers (expanding a list to play items, creating a list from ordered targets) only this file
 * changes.
 *
 * What it assumes of today's GraphQL, and nothing more:
 *
 * - **Expanding** reads `PersonalEntries` (entries ordered by `position`, then `id`) and turns each
 *   movie, show, album, audiobook and track entry into its play items, in order. A show, album or
 *   book entry expands to its playable children. Entries with nothing playable add nothing.
 * - **Creating** makes a `MANUAL` list with `createPersonalList`, then one `createListEntry` per
 *   queue item with `position` 0, 1, 2… in queue order, each with one target. `CreateListEntryInput`
 *   today takes only `movieId`, `showId`, `albumId`, `audiobookId` or `trackId`, so tracks and movies
 *   are saved and **episodes and chapters are skipped**, and reported as skipped. A list that
 *   gains `episodeId` and `chapterId` targets only needs them added to `TARGET`.
 */
export interface PlaylistAdapter {
  expand(client: ApolloClient, listId: string): Promise<PlayItem[]>;
  createFromItems(client: ApolloClient, input: { userId: string; name: string; items: readonly PlayItem[] }): Promise<{ listId: string; saved: number; skipped: number }>;
}

/** The list-entry target a queue item is saved as, or null when lists cannot hold it yet. */
const TARGET: Record<PlayItem["entity"]["kind"], ((item: PlayItem) => { movieId?: string; trackId?: string }) | null> = {
  track: (item) => ({ trackId: item.entity.id }),
  movie: (item) => ({ movieId: item.entity.id }),
  episode: null,
  chapter: null,
};

export const savableItems = (items: readonly PlayItem[]) => items.filter((item) => TARGET[item.entity.kind]);

async function entryItems(client: ApolloClient, entry: PersonalEntryFragment): Promise<PlayItem[]> {
  if (entry.track) return [trackPlayItem({ ...entry.track, album: entry.track.album ? { id: entry.track.album.id, name: "", coverUrl: entry.track.album.coverUrl } : null })].filter((item): item is PlayItem => item !== null);
  if (entry.movie) return [moviePlayItem(entry.movie)].filter((item): item is PlayItem => item !== null);
  if (entry.albumId) return albumPlayItems(client, entry.albumId);
  if (entry.audiobookId) return audiobookItems(client, entry.audiobookId);
  if (entry.showId) return showPlayItems(client, entry.showId);
  return [];
}

export const playlistAdapter: PlaylistAdapter = {
  async expand(client, listId) {
    const entries: PersonalEntryFragment[] = [];
    for (let hasMore = true; hasMore;) {
      const { data } = await client.query({ query: PersonalEntriesDocument, variables: { listId, page: { limit: 100, offset: entries.length } }, fetchPolicy: "no-cache" });
      const page = data?.listEntries;
      if (!page) throw new Error("Playlist unavailable");
      entries.push(...page.edges.map((edge) => edge.node));
      hasMore = page.pageInfo.hasNextPage && page.edges.length > 0;
    }
    const items: PlayItem[] = [];
    for (const entry of entries) items.push(...(await entryItems(client, entry)));
    return items;
  },

  async createFromItems(client, { userId, name, items }) {
    const savable = savableItems(items);
    if (!savable.length) throw new Error("Nothing in the queue can be saved to a playlist yet. Episodes and chapters are not supported.");
    const created = await client.mutate({ mutation: EntityPersonalListCreateDocument, variables: { input: { userId, name: name.trim(), kind: "MANUAL", description: null, savedViewId: null } } });
    assertSuccess(created.data?.createPersonalList, "Could not create the playlist");
    const list = created.data?.createPersonalList?.personalList;
    if (!list) throw new Error("The playlist was not returned");
    let position = 0;
    for (const item of savable) {
      const target = TARGET[item.entity.kind]!(item);
      const { data } = await client.mutate({ mutation: EntityListEntryCreateDocument, variables: { input: { userId, listId: list.id, position, ...target } } });
      assertSuccess(data?.createListEntry, "Could not save an item to the playlist");
      position += 1;
    }
    return { listId: list.id, saved: savable.length, skipped: items.length - savable.length };
  },
};
