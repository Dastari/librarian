import type { ApolloClient } from "@apollo/client";
import { useApolloClient } from "@apollo/client/react";
import type { ActionDefinition, ActionGroup } from "glass-ux/actions";
import { IconPlus, IconSkipForward } from "glass-ux/icons";
import { useMemo, useRef } from "react";

import { expandPlaylist, playlistPlayItems } from "@/features/personal/playlistPlayback";
import type { PlaylistTarget } from "@/features/personal/types";

import { playerStore, type PlayItem } from "./store";

/**
 * What a row stands for in the queue: items already in hand, one playable thing (a movie, show,
 * episode, album, track, audiobook or chapter, which expands to its playable children), several
 * of them in order, or a playlist.
 */
export type QueueSource = PlayItem[] | PlaylistTarget | { kind: "targets"; targets: readonly PlaylistTarget[] } | { kind: "playlist"; id: string };

/**
 * Turns sources into play items, in order. Entities and playlists expand through the playlist
 * helpers (`playlistPlayItems`, `expandPlaylist`), the one place that knows how each kind plays.
 */
export async function resolveQueueSources(client: ApolloClient, sources: readonly QueueSource[]): Promise<PlayItem[]> {
  const groups = await Promise.all(sources.map(async (source): Promise<PlayItem[]> => {
    if (Array.isArray(source)) return source;
    if (source.kind === "playlist") return expandPlaylist(client, source.id);
    return (await playlistPlayItems(client, "targets" in source ? source.targets : [source])).items;
  }));
  return groups.flat();
}

interface QueueActionOptions<T> {
  /** Which rows the actions apply to (default all). */
  visible?: (rows: readonly T[]) => boolean;
  disabledReason?: (rows: readonly T[]) => string | false | null | undefined;
  group?: ActionGroup;
}

/**
 * "Play next" and "Add to queue" for any entity that plays: `toSources` says what each selected
 * row is, in the order the rows should play. Both work on a selection of rows (bulk), a show,
 * album or book expands to its playable children, and with nothing playing they start playback.
 * Spread the result into the entity's `defineActions` list.
 */
export function useQueueActions<T>(toSources: (rows: readonly T[]) => QueueSource[], options: QueueActionOptions<T> = {}): ActionDefinition<T>[] {
  const client = useApolloClient();
  const group = options.group;
  const latest = useRef({ toSources, options });
  latest.current = { toSources, options };
  return useMemo(() => {
    const shared = {
      group: group ?? ("open" as ActionGroup),
      bulk: true as const,
      visible: (rows: readonly T[]) => latest.current.options.visible?.(rows) ?? true,
      disabledReason: (rows: readonly T[]) => latest.current.options.disabledReason?.(rows) ?? false,
    };
    const run = (apply: (items: PlayItem[]) => void) => async (rows: readonly T[]) => {
      const items = await resolveQueueSources(client, latest.current.toSources(rows));
      if (!items.length) throw new Error("Nothing here is ready to play");
      apply(items);
    };
    return [
      { ...shared, id: "play-next", label: "Play next", icon: IconSkipForward, run: run((items) => playerStore.playNext(items)), successMessage: () => "Playing next" },
      { ...shared, id: "enqueue", label: "Add to queue", icon: IconPlus, run: run((items) => playerStore.enqueue(items)), successMessage: () => "Added to queue" },
    ];
  }, [client, group]);
}
