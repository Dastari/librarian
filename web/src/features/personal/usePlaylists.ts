import { useApolloClient, useMutation } from "@apollo/client/react";
import type { ApolloClient } from "@apollo/client";
import { useCallback, useMemo } from "react";

import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import {
  EntityListEntryCreateDocument, EntityPersonalListCreateDocument, PlaylistListDocument, PlaylistMembersDocument,
  type PersonalListKind, type PlaylistSummaryFragment,
} from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { assertSuccess } from "@/lib/graphql/errors";

import { appendPositions } from "./positions";
import { entryTarget, targetFields, targetKey, type PlaylistTarget } from "./types";

export type Playlist = PlaylistSummaryFragment;

/** The signed-in person's lists, each with how many entries it holds and the first few for its picture. */
export function usePlaylists() {
  const { user } = useSession();
  const list = useCompleteConnection(PlaylistListDocument, { where: { userId: { eq: user?.id ?? "" } }, orderBy: [{ name: "ASC" }] }, (data) => data.personalLists, { skip: !user });
  return list;
}

export const playlistCount = (playlist: Playlist) => playlist.entries.pageInfo.totalCount ?? 0;

/** Everything a list holds, as the targets it points at with where each sits. */
async function members(client: ApolloClient, listId: string) {
  const rows: { id: string; position: number; target: PlaylistTarget | null }[] = [];
  for (let offset = 0; ; offset += 100) {
    const { data } = await client.query({ query: PlaylistMembersDocument, variables: { listId, page: { limit: 100, offset } }, fetchPolicy: "network-only" });
    if (!data) throw new Error("The playlist could not be read");
    rows.push(...data.listEntries.edges.map(({ node }) => ({ id: node.id, position: node.position, target: entryTarget(node) })));
    if (!data.listEntries.pageInfo.hasNextPage) return rows;
  }
}

/** A playlist's entries as targets, in order. Read off-cache so playing never uses a stale order. */
export async function fetchPlaylistTargets(client: ApolloClient, listId: string): Promise<PlaylistTarget[]> {
  return (await members(client, listId)).flatMap((row) => row.target ? [row.target] : []);
}

export interface AddResult {
  /** Entries written. */
  added: number;
  /** Targets the playlist already held, or that were listed twice. */
  alreadyThere: number;
}

/**
 * Creating playlists and adding to them. `createPlaylist(name, targets)` is also what "Save queue
 * as playlist" calls with the queue's entries in order; `addToPlaylist` appends and skips what a
 * playlist already holds. Writes go through the generated mutations, one at a time and in order,
 * so a failure part-way leaves the entries before it in place and says how many made it.
 */
export function usePlaylistWrites() {
  const { user } = useSession();
  const client = useApolloClient();
  const [createList] = useMutation(EntityPersonalListCreateDocument);
  const [createEntry] = useMutation(EntityListEntryCreateDocument);

  const addToPlaylist = useCallback(async (listId: string, targets: readonly PlaylistTarget[]): Promise<AddResult> => {
    if (!user) throw new Error("Sign in to use playlists");
    const held = await members(client, listId);
    const seen = new Set(held.flatMap((row) => row.target ? [targetKey(row.target)] : []));
    const fresh = targets.filter((target) => { const key = targetKey(target); if (seen.has(key)) return false; seen.add(key); return true; });
    const positions = appendPositions(held, fresh.length);
    let added = 0;
    for (const [index, target] of fresh.entries()) {
      try {
        const { data } = await createEntry({ variables: { input: { userId: user.id, listId, position: positions[index]!, ...targetFields(target) } } });
        assertSuccess(data?.createListEntry, "Could not add the item");
        added += 1;
      } catch (error) {
        throw new Error(`${added ? `Added ${added} of ${fresh.length}. ` : ""}${error instanceof Error ? error.message : "Could not add the item"}`);
      }
    }
    return { added, alreadyThere: targets.length - fresh.length };
  }, [client, createEntry, user]);

  const createPlaylist = useCallback(async (name: string, targets: readonly PlaylistTarget[] = [], kind: PersonalListKind = "MANUAL") => {
    if (!user) throw new Error("Sign in to use playlists");
    const { data } = await createList({ variables: { input: { userId: user.id, name: name.trim(), kind, savedViewId: null, description: null } } });
    assertSuccess(data?.createPersonalList, "Could not create the playlist");
    const created = data?.createPersonalList?.personalList;
    if (!created) throw new Error("The playlist was not returned");
    const result = targets.length ? await addToPlaylist(created.id, targets) : { added: 0, alreadyThere: 0 };
    return { id: created.id, name: created.name, ...result };
  }, [addToPlaylist, createList, user]);

  return useMemo(() => ({ addToPlaylist, createPlaylist }), [addToPlaylist, createPlaylist]);
}

/** Just the "make a playlist from an ordered list of targets" half, for callers that need nothing else. */
export function useCreatePlaylistFromTargets() {
  return usePlaylistWrites().createPlaylist;
}
