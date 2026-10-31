import type { ActionSet } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { EntityIdentity } from "glass-ux/entity";
import { IconClose } from "glass-ux/icons";
import { Progress } from "glass-ux/progress";

import { Button, KeyValueList, Panel } from "@/components/ui";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { torrentImport, torrentStatus, torrentTarget } from "@/lib/entities/torrent";
import { formatBytes, formatDateTime, formatSpeed } from "@/lib/format";

import type { DownloadRow } from "./useDownloads";

const TorrentGlyph = ENTITY_KINDS.torrent.icon;

/** A download's identity, its actions, the import outcome and the files inside it. */
export function TorrentDetail({ row, actions, onClose }: { row: DownloadRow; actions: ActionSet<DownloadRow>; onClose: () => void }) {
  const { record, live } = row;
  const state = torrentStatus(live?.state ?? record.state);
  const imported = torrentImport(record.postProcessStatus);
  const target = torrentTarget(record);
  const progress = live?.progress ?? record.progress;
  const TargetGlyph = target ? ENTITY_KINDS[target.kind as keyof typeof ENTITY_KINDS]?.icon : undefined;

  return (
    <Panel className="self-start xl:sticky xl:top-0">
      <div className="flex items-start gap-2">
        <EntityIdentity
          className="min-w-0 flex-1"
          tier="compact"
          heading="h2"
          kind={ENTITY_KINDS.torrent.noun}
          name={record.name}
          icon={<TorrentGlyph />}
          facts={[state.label, formatBytes(record.totalBytes), record.library?.name ?? "No library"]}
          actions={actions}
          target={row}
        />
        <Button variant="ghost" size="sm" isIconOnly aria-label="Close download details" onPress={onClose}><IconClose /></Button>
      </div>
      <div className="mt-4 flex flex-col gap-2">
        <Progress value={progress} max={1} size="sm" aria-label="Download progress" />
        <div className="flex flex-wrap items-center gap-2">
          <Badge size="sm" marker="dot" tone={imported.tone}>{imported.label}</Badge>
          {target ? <EntityIdentity tier="chip" size="sm" kind={ENTITY_KINDS[target.kind as keyof typeof ENTITY_KINDS]?.noun} name={target.label} href={target.href} icon={TargetGlyph ? <TargetGlyph /> : undefined} preview={false} /> : null}
        </div>
      </div>
      <KeyValueList
        className="mt-4"
        items={[
          { label: "Progress", value: `${Math.round(progress * 100)}% · ${formatBytes(live?.downloaded ?? record.downloadedBytes)} of ${formatBytes(record.totalBytes)}` },
          { label: "Speed", value: live ? `↓ ${formatSpeed(live.downloadSpeed)} · ↑ ${formatSpeed(live.uploadSpeed)}` : "Not loaded in the client" },
          { label: "Peers", value: live?.peers },
          { label: "Uploaded", value: formatBytes(record.uploadedBytesTotal) },
          { label: "Tracker minimum", value: record.minimumRatio ? `Ratio ${record.minimumRatio.toFixed(2)}${record.minimumSeedTimeMinutes ? ` · ${Math.round(record.minimumSeedTimeMinutes / 60)} h seeding` : ""}` : undefined },
          { label: "Save path", value: live?.savePath ?? record.savePath, mono: true },
          { label: "Info hash", value: record.infoHash, mono: true },
          { label: "Added", value: formatDateTime(record.addedAt) },
          { label: "Completed", value: record.completedAt ? formatDateTime(record.completedAt) : undefined },
          { label: "Season", value: record.season !== null && record.season !== undefined ? (record.season === 0 ? "Specials pack" : `Season ${record.season} pack`) : undefined },
          { label: "Import error", value: record.postProcessError ? <span className="text-danger">{record.postProcessError}</span> : undefined },
        ]}
      />
      {live?.files.length ? (
        <>
          <p className="text-overline mb-2 mt-5 text-muted">Files</p>
          <ul className="scrollbar-thin max-h-80 overflow-y-auto text-body-sm">
            {live.files.map((file) => (
              <li key={file.index} className="flex items-center gap-3 border-b border-separator py-1.5 last:border-b-0">
                <span title={file.path} className="min-w-0 flex-1 break-all font-mono text-label text-foreground">{file.path.split(/[\\/]/).pop()}</span>
                <span className="text-numeric shrink-0 text-label-sm text-muted">{formatBytes(file.size)}</span>
                <Progress value={file.progress} max={1} size="sm" aria-label={`Progress of ${file.path}`} className="w-16 shrink-0" />
              </li>
            ))}
          </ul>
        </>
      ) : null}
    </Panel>
  );
}
