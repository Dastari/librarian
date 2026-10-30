import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { useMemo } from "react";

import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";

import { EntityTorrentListDocument, LiveTorrentsDocument, type EntityTorrentListQuery, type LiveTorrentsQuery } from "@/graphql/generated/graphql";

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
  const live = useQuery(LiveTorrentsDocument);
  const hashes = useMemo(() => live.data?.liveTorrents.map((torrent) => torrent.infoHash) ?? [], [live.data]);
  const records = useCompleteConnection(EntityTorrentListDocument, { where: { infoHash: { inList: hashes } }, orderBy: [{ addedAt: "DESC" }] }, (data) => data.torrents, { skip: hashes.length === 0 });

  const refresh = () => {
    void live.refetch();
    void records.refetch();
  };

  const rows = useMemo<TorrentRow[]>(() => {
    const byHash = new Map(records.rows.map((record) => [record.infoHash.toLowerCase(), record]));
    return (live.data?.liveTorrents ?? []).map((torrent) => ({ live: torrent, record: byHash.get(torrent.infoHash.toLowerCase()) ?? null }));
  }, [live.data, records.rows]);

  return { rows, loading: live.loading && !live.data, error: live.error ?? records.error, recordError: records.error, refetch: refresh };
}
