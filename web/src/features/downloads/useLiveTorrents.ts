import { useApolloClient, useQuery, useSubscription } from "@apollo/client/react";
import { useMemo } from "react";

import { EntityTorrentListDocument, LiveTorrentsDocument, TorrentAddedDocument, TorrentCompletedDocument, TorrentProgressDocument, TorrentRemovedDocument, type EntityTorrentListQuery, type LiveTorrentsQuery } from "@/graphql/generated/graphql";

export type LiveTorrent = LiveTorrentsQuery["liveTorrents"][number];
export type TorrentRecord = EntityTorrentListQuery["torrents"]["edges"][number]["node"];

export interface TorrentRow {
  live: LiveTorrent;
  record: TorrentRecord | null;
}

/**
 * Live client state merged with the persisted torrent records (library linkage, post-processing
 * outcome). Progress arrives over the subscription and is written straight into the cache.
 */
export function useLiveTorrents() {
  const client = useApolloClient();
  const live = useQuery(LiveTorrentsDocument, { pollInterval: 15_000 });
  const records = useQuery(EntityTorrentListDocument, { variables: { page: { limit: 100, offset: 0 }, orderBy: [{ addedAt: "DESC" }] } });

  useSubscription(TorrentProgressDocument, {
    onData: ({ data }) => {
      const update = data.data?.torrentProgress;
      if (!update) return;
      client.cache.modify({
        id: client.cache.identify({ __typename: "LiveTorrent", infoHash: update.infoHash }),
        fields: {
          progress: () => update.progress,
          downloadSpeed: () => update.downloadSpeed,
          uploadSpeed: () => update.uploadSpeed,
          peers: () => update.peers,
          state: () => update.state,
        },
      });
    },
  });
  const refresh = () => {
    void live.refetch();
    void records.refetch();
  };
  useSubscription(TorrentAddedDocument, { onData: refresh });
  useSubscription(TorrentCompletedDocument, { onData: refresh });
  useSubscription(TorrentRemovedDocument, { onData: refresh });

  const rows = useMemo<TorrentRow[]>(() => {
    const byHash = new Map(records.data?.torrents.edges.map((edge) => [edge.node.infoHash.toLowerCase(), edge.node]) ?? []);
    return (live.data?.liveTorrents ?? []).map((torrent) => ({ live: torrent, record: byHash.get(torrent.infoHash.toLowerCase()) ?? null }));
  }, [live.data, records.data]);

  return { rows, loading: live.loading && !live.data, error: live.error, refetch: refresh };
}
