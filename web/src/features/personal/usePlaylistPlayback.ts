import { useApolloClient } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { useCallback } from "react";

import { usePlayer } from "@/features/player/usePlayer";

import { playlistPlayItems, shufflePlayItems, startPlaylistPlayback } from "./playlistPlayback";
import { fetchPlaylistTargets } from "./usePlaylists";
import type { PlaylistTarget } from "./types";

/**
 * Playing playlists and lists of targets from anywhere. `play`/`shuffle` put what can play in the
 * queue in order (or shuffled) and start it; they reject with a sentence for a toast when nothing
 * has a file. `*Playlist` read the playlist's entries first. Albums, shows and audiobooks among
 * the targets expand to their children (see playlistPlayback.ts).
 */
export function usePlaylistPlayback() {
  const client = useApolloClient();
  const player = usePlayer();
  const navigate = useNavigate();

  const start = useCallback(async (targets: readonly PlaylistTarget[], shuffle: boolean) => {
    const { items, skipped } = await playlistPlayItems(client, targets);
    if (!items.length) throw new Error(targets.length ? "Nothing here has a file to play yet" : "This playlist is empty");
    const ordered = shuffle ? shufflePlayItems(items) : items;
    startPlaylistPlayback(player, (mediaFileId) => void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId } }), ordered);
    return { played: ordered.length, skipped };
  }, [client, navigate, player]);

  return {
    play: useCallback((targets: readonly PlaylistTarget[]) => start(targets, false), [start]),
    shuffle: useCallback((targets: readonly PlaylistTarget[]) => start(targets, true), [start]),
    playPlaylist: useCallback(async (listId: string) => start(await fetchPlaylistTargets(client, listId), false), [client, start]),
    shufflePlaylist: useCallback(async (listId: string) => start(await fetchPlaylistTargets(client, listId), true), [client, start]),
  };
}
