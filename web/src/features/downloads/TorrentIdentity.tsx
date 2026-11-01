import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity } from "glass-ux/entity";
import { IconClose } from "glass-ux/icons";

import { Button } from "@/components/ui";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { torrentStatus } from "@/lib/entities/torrent";
import { formatBytes } from "@/lib/format";

import type { DownloadRow } from "./useDownloads";

const TorrentGlyph = ENTITY_KINDS.torrent.icon;

/** One identity contract in the table and inspector; the row tier keeps its menu inline. */
export function TorrentIdentity({ row, actions, onClose }: {
  row: DownloadRow;
  actions?: ActionSet<DownloadRow>;
  onClose?: () => void;
}) {
  const rawState = (row.live?.state ?? row.record.state).toLowerCase();
  const state = torrentStatus(rawState);
  return (
    <div data-torrent-identity className="flex min-w-0 flex-1 flex-nowrap items-center gap-2">
      {onClose ? <h2 className="sr-only">{row.record.name}</h2> : null}
      <EntityIdentity
        tier="row"
        className="min-w-0 flex-1"
        kind={ENTITY_KINDS.torrent.noun}
        name={row.record.name}
        icon={<TorrentGlyph />}
        state={{ id: rawState, label: state.label, tone: rawState === "error" ? "danger" : rawState === "seeding" || rawState === "completed" ? "success" : rawState === "downloading" || rawState === "checking" || rawState === "initializing" ? "info" : "neutral", paused: rawState === "paused" }}
        facts={[state.label, formatBytes(row.record.totalBytes), row.record.library?.name ?? "No library"]}
        target={row}
        actions={actions}
      />
      {onClose ? <Button className="shrink-0" variant="ghost" size="sm" isIconOnly aria-label="Close download details" onPress={onClose}><IconClose /></Button> : null}
    </div>
  );
}
