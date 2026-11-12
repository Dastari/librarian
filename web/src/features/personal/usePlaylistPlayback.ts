import { useApolloClient } from "@apollo/client/react";
import { useCallback } from "react";

import { usePlayer } from "@/features/player/usePlayer";

import { playlistPlayItems, shufflePlayItems, startPlaylistPlayback } from "./playlistPlayback";
import { fetchPlaylistTargets } from "./usePlaylists";
import type { PlaylistTarget } from "./types";

/**
 * Playing playlists and lists of targets from anywhere. `play`/`shuffle` make what can play the play
 * queue, in order (or shuffled), and start it; they reject with a sentence for a toast when nothing
 * has a file. `play(targets, from)` starts at the target at `from` ("Play from here") with the whole
 * list still in the queue around it. `*Playlist` read the playlist's entries first. Albums, shows and
 * audiobooks among the targets expand to their children (see playlistPlayback.ts). Audio and video
 * mix in one queue; the queue director switches between the dock and the video player.
 */
export function usePlaylistPlayback() {
  const client = useApolloClient();
  const player = usePlayer();

  const start = useCallback(async (targets: readonly PlaylistTarget[], shuffle: boolean, from = 0) => {
    const { items, skipped, starts } = await playlistPlayItems(client, targets);
    if (!items.length) throw new Error(targets.length ? "Nothing here has a file to play yet" : "This playlist is empty");
    // An entry with no file starts at the next one that has.
    const at = starts.slice(from).find((value): value is number => value !== null);
    if (at === undefined) throw new Error("Nothing from here has a file to play yet");
    if (shuffle) {
      // A shuffle from an entry shuffles that entry and what follows it.
      const ordered = shufflePlayItems(items.slice(at));
      startPlaylistPlayback(player, ordered);
      return { played: ordered.length, skipped };
    }
    startPlaylistPlayback(player, items, at);
    return { played: items.length - at, skipped };
  }, [client, player]);

  return {
    play: useCallback((targets: readonly PlaylistTarget[], from = 0) => start(targets, false, from), [start]),
    shuffle: useCallback((targets: readonly PlaylistTarget[], from = 0) => start(targets, true, from), [start]),
    playPlaylist: useCallback(async (listId: string) => start(await fetchPlaylistTargets(client, listId), false), [client, start]),
    shufflePlaylist: useCallback(async (listId: string) => start(await fetchPlaylistTargets(client, listId), true), [client, start]),
  };
}
