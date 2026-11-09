import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity } from "glass-ux/entity";
import type { ReactNode } from "react";

import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { torrentStatus } from "@/lib/entities/torrent";
import { formatBytes } from "@/lib/format";

import type { DownloadRow } from "./useDownloads";

const TorrentGlyph = ENTITY_KINDS.torrent.icon;

/** One identity for a download: a row in the table, the header of its dialog. */
export function TorrentIdentity({ row, actions, tier = "row", facts, className }: {
  row: DownloadRow;
  actions?: ActionSet<DownloadRow>;
  tier?: "row" | "header";
  /** Replaces the usual facts under the name. */
  facts?: ReactNode;
  className?: string;
}) {
  const rawState = (row.live?.state ?? row.record.state).toLowerCase();
  const state = torrentStatus(rawState);
  // Why it is stopped, in the client's words; a download share that is down outranks the rest.
  const detail = row.live?.statusDetail ?? null;
  const waitingForShare = detail?.startsWith("Waiting for the download share") ?? false;
  const shown = waitingForShare
    ? { id: "waiting-share", label: "Waiting for share", tone: "warning" as const, paused: true }
    : { id: rawState, label: state.label, tone: rawState === "error" ? "danger" as const : rawState === "seeding" || rawState === "completed" ? "success" as const : rawState === "downloading" || rawState === "checking" || rawState === "initializing" ? "info" as const : "neutral" as const, paused: rawState === "paused" };
  return (
    <EntityIdentity
      tier={tier}
      heading="h2"
      className={className ?? "min-w-0 flex-1"}
      kind={ENTITY_KINDS.torrent.noun}
      name={row.record.name}
      icon={<TorrentGlyph />}
      state={shown}
      facts={facts ?? (detail && (waitingForShare || rawState === "error") ? [detail] : [...(tier === "row" ? [shown.label] : []), formatBytes(row.record.totalBytes), row.record.library?.name ?? "No library"])}
      target={row}
      actions={actions}
    />
  );
}
