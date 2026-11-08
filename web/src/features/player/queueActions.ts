import type { ApolloClient } from "@apollo/client";
import { useApolloClient } from "@apollo/client/react";
import type { ActionDefinition, ActionGroup } from "glass-ux/actions";
import { IconPlus, IconSkipForward } from "glass-ux/icons";
import { useMemo, useRef } from "react";

import { playlistAdapter } from "./playlistAdapter";
import { resolvePlayItems, type PlayableRef } from "./playables";
import { playerStore, type PlayItem } from "./store";

/** What a row stands for in the queue: items already in hand, a thing to expand, or a playlist. */
export type QueueSource = PlayItem[] | PlayableRef | { kind: "playlist"; id: string };

/** Turns sources into play items, in order. Entities are expanded with the GraphQL they are read through. */
export async function resolveQueueSources(client: ApolloClient, sources: readonly QueueSource[]): Promise<PlayItem[]> {
  const groups = await Promise.all(sources.map((source) => {
    if (Array.isArray(source)) return Promise.resolve(source);
    return source.kind === "playlist" ? playlistAdapter.expand(client, source.id) : resolvePlayItems(client, source);
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
