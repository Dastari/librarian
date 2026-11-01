import type { ActionSet } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { FactList } from "glass-ux/fact-list";
import { Notice } from "glass-ux/notice";
import { Progress } from "glass-ux/progress";
import { ScrollArea } from "glass-ux/scroll-area";
import { Panel, PanelBody } from "glass-ux/surface";
import { TruncatedText } from "glass-ux/truncated-text";
import { useId } from "react";

import { EntityChip } from "@/components/entities/EntityChip";
import { torrentImport, torrentTarget } from "@/lib/entities/torrent";
import { formatBytes, formatDateTime, formatSpeed } from "@/lib/format";

import { TorrentIdentity } from "./TorrentIdentity";
import type { DownloadRow } from "./useDownloads";

const fraction = (value: number) => Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0;

/** Fixed identity and actions; the panel's public body slot owns all detail scrolling. */
export function TorrentDetail({ row, actions, onClose, embedded = false }: { row: DownloadRow; actions: ActionSet<DownloadRow>; onClose: () => void; embedded?: boolean }) {
  const sectionId = useId();
  const { record, live } = row;
  const imported = torrentImport(record.postProcessStatus);
  const target = torrentTarget(record);
  const progress = fraction(live?.progress ?? record.progress);
  const downloaded = live?.downloaded ?? record.downloadedBytes;
  const ratio = downloaded > 0 ? record.uploadedBytesTotal / downloaded : 0;

  const header = <header className="shrink-0 p-panel pb-0">
    <TorrentIdentity row={row} actions={actions} onClose={onClose} />
  </header>;
  const content = (
    <div className={`flex min-w-0 flex-col gap-6${embedded ? " p-panel" : ""}`}>
      <section aria-label="Transfer progress" className="flex min-w-0 flex-col gap-2">
        <div className="flex items-baseline justify-between gap-3 text-label-sm">
          <span className="min-w-0 text-muted">{formatBytes(downloaded)} of {formatBytes(record.totalBytes)}</span>
          <span className="text-numeric shrink-0 text-foreground">{Math.round(progress * 100)}%</span>
        </div>
        <Progress value={progress} max={1} size="sm" showValue={false} aria-label="Download progress" />
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <Badge size="sm" marker="dot" tone={imported.tone}>{imported.label}</Badge>
          {target ? <EntityChip entity={target} /> : null}
        </div>
      </section>

      {record.postProcessError ? <Notice tone="danger" title="Import failed" className="min-w-0">
        <p className="whitespace-pre-wrap break-words [overflow-wrap:anywhere]">{record.postProcessError}</p>
      </Notice> : null}

      <section aria-labelledby={`${sectionId}-torrent-transfer-facts`} className="flex min-w-0 flex-col gap-3">
        <h3 id={`${sectionId}-torrent-transfer-facts`} className="text-overline text-muted">Transfer</h3>
        <FactList facts={[
          { label: "Download speed", value: live ? formatSpeed(live.downloadSpeed) : "Not loaded in the client" },
          { label: "Upload speed", value: live ? formatSpeed(live.uploadSpeed) : "Not loaded in the client" },
          { label: "Peers", value: live?.peers ?? "—" },
          { label: "Uploaded", value: formatBytes(record.uploadedBytesTotal) },
          { label: "Share ratio", value: ratio.toFixed(2) },
          ...(record.minimumRatio ? [{ label: "Tracker minimum", value: `Ratio ${record.minimumRatio.toFixed(2)}${record.minimumSeedTimeMinutes ? ` · ${Math.round(record.minimumSeedTimeMinutes / 60)} h seeding` : ""}` }] : []),
        ]} />
      </section>

      <section aria-labelledby={`${sectionId}-torrent-storage-facts`} className="flex min-w-0 flex-col gap-3">
        <h3 id={`${sectionId}-torrent-storage-facts`} className="text-overline text-muted">Storage and history</h3>
        <FactList facts={[
          { label: "Library", value: record.library?.name ?? "Not linked" },
          { label: "Save path", value: <span className="break-all font-mono text-label-sm">{live?.savePath ?? record.savePath}</span> },
          { label: "Info hash", value: <span className="break-all font-mono text-label-sm">{record.infoHash}</span> },
          { label: "Added", value: formatDateTime(record.addedAt) },
          ...(record.completedAt ? [{ label: "Completed", value: formatDateTime(record.completedAt) }] : []),
          ...(record.season != null ? [{ label: "Season", value: record.season === 0 ? "Specials pack" : `Season ${record.season} pack` }] : []),
        ]} />
      </section>

      <section aria-labelledby={`${sectionId}-torrent-files`} className="flex min-w-0 flex-col gap-3">
        <h3 id={`${sectionId}-torrent-files`} className="text-overline text-muted">Files{live ? ` · ${live.files.length}` : ""}</h3>
        {live?.files.length ? <ul aria-label="Torrent files" className="min-w-0 text-body-sm">
          {live.files.map((file) => <li key={file.index} className="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] items-center gap-x-3 gap-y-1 border-b border-separator py-3 last:border-b-0">
            <TruncatedText className="min-w-0 font-mono text-label-sm" title={file.path}>{file.path.split(/[\\/]/).pop() ?? file.path}</TruncatedText>
            <span className="text-numeric shrink-0 text-label-sm text-muted">{formatBytes(file.size)}</span>
            <Progress value={fraction(file.progress)} max={1} size="sm" showValue={false} aria-label={`Progress of ${file.path}`} className="col-span-2" />
          </li>)}
        </ul> : <p className="text-body-sm text-muted">{live ? "No files reported by the client." : "Load the torrent in the client to view its files."}</p>}
      </section>
    </div>
  );
  if (embedded) return (
    <section aria-label="Download details" className="flex h-full min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      {header}
      <ScrollArea aria-label="Download details content" className="min-h-0 min-w-0 flex-1">{content}</ScrollArea>
    </section>
  );
  return (
    <Panel scroll aria-label="Download details" className="h-full min-h-0 min-w-0 overflow-hidden">
      {header}
      <PanelBody aria-label="Download details content" className="min-w-0">{content}</PanelBody>
    </Panel>
  );
}
