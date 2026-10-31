import type { DataTableState } from "glass-ux/data-table";
import { useMemo } from "react";

import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import { DownloadCountsDocument, DownloadsTableDocument, LiveTorrentsDocument, type DownloadsTableQuery, type LiveTorrentsQuery, type TorrentOrderByInput, type TorrentWhereInput } from "@/graphql/generated/graphql";
import { TORRENT_STATE_PRESETS } from "@/lib/entities/torrent";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

export type DownloadRecord = DownloadsTableQuery["torrents"]["edges"][number]["node"];
export type LiveTorrent = LiveTorrentsQuery["liveTorrents"][number];

/** A stored torrent and, when the client has it loaded, its live state. */
export interface DownloadRow {
  id: string;
  record: DownloadRecord;
  live: LiveTorrent | null;
}

export const DOWNLOAD_PRESETS = ["all", "downloading", "seeding", "paused", "failed"] as const;
export type DownloadPreset = (typeof DOWNLOAD_PRESETS)[number];

const BATCH = 50;

function presetWhere(preset: DownloadPreset): TorrentWhereInput | null {
  switch (preset) {
    case "downloading": return { state: { inList: [...TORRENT_STATE_PRESETS.downloading] } };
    case "seeding": return { state: { eq: "seeding" } };
    case "paused": return { state: { eq: "paused" } };
    case "failed": return { or: [{ state: { eq: "error" } }, { postProcessStatus: { eq: "failed" } }] };
    default: return null;
  }
}

/** Option values as strings, from a glass-ux filter value ({ op: "in", values }). */
function optionValues(raw: unknown): string[] {
  if (raw && typeof raw === "object" && "values" in raw && Array.isArray(raw.values)) return raw.values.filter((value): value is string => typeof value === "string");
  return [];
}

/**
 * The table's state as a server query. Only the columns' declared filters cross this
 * boundary; anything else is a programming error rather than a silently ignored filter.
 */
export function downloadsVariables(state: DataTableState, preset: DownloadPreset): { where: TorrentWhereInput; orderBy: TorrentOrderByInput[] } {
  const conditions: TorrentWhereInput[] = [];
  const base = presetWhere(preset);
  if (base) conditions.push(base);
  for (const { id, value } of state.filters) {
    const values = optionValues(value);
    switch (id) {
      case "state": if (values.length) conditions.push({ state: { inList: values } }); break;
      case "import": {
        const waiting = values.includes("waiting");
        const stored = values.filter((item) => item !== "waiting");
        const or: TorrentWhereInput[] = [];
        if (stored.length) or.push({ postProcessStatus: { inList: stored } });
        if (waiting) or.push({ postProcessStatus: { isNull: true } });
        if (or.length) conditions.push({ or });
        break;
      }
      case "library": if (values.length) conditions.push(values.includes("none") ? { or: [{ libraryId: { isNull: true } }, ...(values.length > 1 ? [{ libraryId: { inList: values.filter((item) => item !== "none") } }] : [])] } : { libraryId: { inList: values } }); break;
      default: throw new Error(`Unsupported download filter: ${id}`);
    }
  }
  const query = state.search?.trim();
  if (query) conditions.push({ name: { contains: query } });

  const orderBy: TorrentOrderByInput[] = state.sorting.flatMap(({ id, desc }): TorrentOrderByInput[] => {
    const direction = desc ? "DESC" : "ASC";
    switch (id) {
      case "name": return [{ name: direction }];
      case "progress": return [{ progress: direction }];
      case "size": return [{ totalBytes: direction }];
      case "added": return [{ addedAt: direction }];
      case "state": return [{ state: direction }];
      default: return [];
    }
  });
  return { where: conditions.length ? { and: conditions } : {}, orderBy: orderBy.length ? orderBy : [{ addedAt: "DESC" }] };
}

/**
 * Downloads: stored torrents in server batches (filters, sorting and search run on the
 * server), joined with the client's live state by info hash. The live list is the client's
 * in-memory set and updates over the TorrentProgress subscription.
 */
export function useDownloads(state: DataTableState, preset: DownloadPreset) {
  const variables = useMemo(() => downloadsVariables(state, preset), [state, preset]);
  const list = useSettingsConnection(DownloadsTableDocument, variables, (data) => data.torrents, { infinite: true, batchSize: BATCH, live: true });
  const live = useQuery(LiveTorrentsDocument);
  const counts = useQuery(DownloadCountsDocument);

  const liveByHash = useMemo(() => new Map((live.data?.liveTorrents ?? []).map((torrent) => [torrent.infoHash.toLowerCase(), torrent])), [live.data]);
  const rows = useMemo<DownloadRow[]>(() => list.rows.map((record) => ({ id: record.id, record, live: liveByHash.get(record.infoHash.toLowerCase()) ?? null })), [list.rows, liveByHash]);
  const presetCounts: Partial<Record<DownloadPreset, number>> = counts.data ? {
    all: counts.data.all.pageInfo.totalCount ?? undefined,
    downloading: counts.data.downloading.pageInfo.totalCount ?? undefined,
    seeding: counts.data.seeding.pageInfo.totalCount ?? undefined,
    paused: counts.data.paused.pageInfo.totalCount ?? undefined,
    failed: counts.data.failed.pageInfo.totalCount ?? undefined,
  } : {};

  const refetch = () => {
    void list.refetch().catch(() => {});
    void live.refetch();
    void counts.refetch();
  };

  return { rows, list, live: live.data?.liveTorrents ?? [], liveByHash, counts: presetCounts, variables, refetch };
}
