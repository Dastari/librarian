import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity } from "glass-ux/entity";

import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { torrentStatus } from "@/lib/entities/torrent";
import { formatBytes } from "@/lib/format";

import type { DownloadRow } from "./useDownloads";

const TorrentGlyph = ENTITY_KINDS.torrent.icon;

/** One identity for a download: a row in the table, the header of its dialog. */
export function TorrentIdentity({ row, actions, tier = "row", className }: {
  row: DownloadRow;
  actions?: ActionSet<DownloadRow>;
  tier?: "row" | "header";
  className?: string;
}) {
  const rawState = (row.live?.state ?? row.record.state).toLowerCase();
  const state = torrentStatus(rawState);
  return (
    <EntityIdentity
      tier={tier}
      heading="h2"
      className={className ?? "min-w-0 flex-1"}
      kind={ENTITY_KINDS.torrent.noun}
      name={row.record.name}
      icon={<TorrentGlyph />}
      state={{ id: rawState, label: state.label, tone: rawState === "error" ? "danger" : rawState === "seeding" || rawState === "completed" ? "success" : rawState === "downloading" || rawState === "checking" || rawState === "initializing" ? "info" : "neutral", paused: rawState === "paused" }}
      facts={[...(tier === "row" ? [state.label] : []), formatBytes(row.record.totalBytes), row.record.library?.name ?? "No library"]}
      target={row}
      actions={actions}
    />
  );
}
