import { ActionBar, type ActionSet } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { Gauge } from "glass-ux/charts";
import { CopyButton } from "glass-ux/copy-button";
import { IconCalendar, IconCheck, IconClock, IconFolder, IconHash, IconLayers } from "glass-ux/icons";
import { Metric } from "glass-ux/metric";
import { Notice } from "glass-ux/notice";
import { Progress } from "glass-ux/progress";
import { TruncatedText } from "glass-ux/truncated-text";

import { EntityChip } from "@/components/entities/EntityChip";
import { DetailGrid, DetailTile, EntityDetailDialog, type DetailSection } from "@/components/entities/EntityDetailDialog";
import { torrentImport, torrentTarget } from "@/lib/entities/torrent";
import { formatBytes, formatDateTime, formatSpeed } from "@/lib/format";

import { TorrentFiles, useTorrentFiles } from "./TorrentFiles";
import { TorrentIdentity } from "./TorrentIdentity";
import type { DownloadRow } from "./useDownloads";
import type { RateHistory } from "./useRateHistory";

type Rates = RateHistory["byHash"] extends ReadonlyMap<string, infer Series> ? Series : never;

const fraction = (value: number) => Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0;
const RANGE = "Last two minutes";

/** With the files listed below, the notice needn't list them too: "19 file(s) did not match any library item". */
function noticeText(error: string, filesListed: boolean) {
  const lead = error.split(": ")[0]!;
  return filesListed && /^\d+ file\(s\)/.test(lead) ? `${lead}. See the list below.` : error;
}

function Copyable({ value, label }: { value: string; label: string }) {
  return (
    <span className="flex min-w-0 items-center gap-1">
      <TruncatedText middle numeric className="min-w-0 flex-1 font-mono text-label-sm">{value}</TruncatedText>
      <CopyButton value={value} label={label} size="sm" variant="ghost" />
    </span>
  );
}

function TorrentFacts({ row }: { row: DownloadRow }) {
  const { record, live } = row;
  return (
    <div className="flex min-w-0 flex-col gap-3">
      <DetailGrid label="History">
        <DetailTile icon={<IconLayers />} label="Library" value={record.library?.name ?? "Not linked"} detail={record.season != null ? (record.season === 0 ? "Specials pack" : `Season ${record.season} pack`) : undefined} />
        <DetailTile icon={<IconCalendar />} label="Added" value={formatDateTime(record.addedAt)} />
        {record.completedAt ? <DetailTile icon={<IconCheck />} label="Completed" value={formatDateTime(record.completedAt)} /> : null}
        {record.minimumRatio ? <DetailTile icon={<IconClock />} label="Tracker minimum" value={`Ratio ${record.minimumRatio.toFixed(2)}`} detail={record.minimumSeedTimeMinutes ? `${Math.round(record.minimumSeedTimeMinutes / 60)} h seeding` : undefined} /> : null}
      </DetailGrid>
      <DetailTile icon={<IconFolder />} label="Save path" value={<Copyable value={live?.savePath ?? record.savePath} label="Copy save path" />} />
      <DetailTile icon={<IconHash />} label="Info hash" value={<Copyable value={record.infoHash} label="Copy info hash" />} />
      {record.postProcessError ? <p className="whitespace-pre-wrap text-label-sm text-muted [overflow-wrap:anywhere]">{record.postProcessError}</p> : null}
    </div>
  );
}

/**
 * The transfer as it is happening: speed and peers over the last two minutes, sampled on this page
 * because the server keeps no history, and the share ratio against what the tracker asks for.
 * Unknown (a download the client hasn't loaded) reads as unknown rather than as zero.
 */
function TransferMetrics({ row, rates, ratio }: { row: DownloadRow; rates?: Rates; ratio: number }) {
  const { record, live } = row;
  const target = record.minimumRatio && record.minimumRatio > 0 ? record.minimumRatio : 1;
  return (
    <div role="group" aria-label="Transfer" className="grid grid-cols-2 gap-x-6 gap-y-3 border-y border-separator py-3 md:grid-cols-[1fr_1fr_1fr_15rem]">
      <div className="h-20 min-w-0"><Metric label="Download" value={live ? live.downloadSpeed ?? 0 : null} data={rates?.down} format={formatSpeed} range={RANGE} better="none" /></div>
      <div className="h-20 min-w-0"><Metric label="Upload" value={live ? live.uploadSpeed ?? 0 : null} data={rates?.up} format={formatSpeed} range={RANGE} tone="success" better="none" /></div>
      <div className="h-20 min-w-0"><Metric label="Peers" value={live ? live.peers ?? 0 : null} data={rates?.peers} range={RANGE} tone="neutral" better="none" /></div>
      <div className="flex h-20 min-w-0 items-center gap-3">
        <div className="size-20 shrink-0"><Gauge label="Share ratio" value={ratio} min={0} max={Math.max(target, ratio)} warningAt={null} dangerAt={null} format={(value) => value.toFixed(2)} /></div>
        <div className="min-w-0 text-label-sm">
          <p className="text-muted">Share ratio</p>
          <p className="text-numeric text-foreground">{formatBytes(record.uploadedBytesTotal)} uploaded</p>
        </div>
      </div>
    </div>
  );
}

/** A download in the shared detail dialog. Live values update in place while it is open. */
export function TorrentDetail({ row, rates, actions, open, onClose, finalFocus }: {
  row: DownloadRow;
  rates?: Rates;
  actions: ActionSet<DownloadRow>;
  open: boolean;
  onClose: () => void;
  finalFocus?: Parameters<typeof EntityDetailDialog>[0]["finalFocus"];
}) {
  const { record, live } = row;
  const imported = torrentImport(record.postProcessStatus);
  const target = torrentTarget(record);
  const progress = fraction(live?.progress ?? record.progress);
  const downloaded = live?.downloaded ?? record.downloadedBytes;
  const ratio = downloaded > 0 ? record.uploadedBytesTotal / downloaded : 0;
  const files = useTorrentFiles(row);

  const sections: DetailSection[] = [
    { key: "files", label: "Files", count: files.files.length || undefined, content: <TorrentFiles row={row} files={files} /> },
    { key: "details", label: "Details", content: <TorrentFacts row={row} /> },
  ];

  return (
    <EntityDetailDialog
      open={open}
      onOpenChange={(next) => { if (!next) onClose(); }}
      label="Download details"
      identity={<TorrentIdentity tier="header" row={row} className="min-w-0" />}
      actions={<ActionBar set={actions} targets={[row]} label={`Actions for ${record.name}`} maxButtons={2} />}
      finalFocus={finalFocus}
      badges={record.postProcessError && !target ? undefined : (
        <>
          {record.postProcessError ? null : <Badge size="sm" marker="dot" tone={imported.tone}>{imported.label}</Badge>}
          {target ? <EntityChip entity={target} /> : null}
        </>
      )}
      notice={record.postProcessError ? (
        <Notice tone={record.postProcessStatus === "failed" ? "danger" : "warning"} title={imported.label} className="min-w-0">
          <p className="line-clamp-2 whitespace-pre-wrap break-words [overflow-wrap:anywhere]">{noticeText(record.postProcessError, files.files.length > 0)}</p>
        </Notice>
      ) : undefined}
      summary={
        <div className="flex min-w-0 flex-col gap-3">
          {progress >= 1 ? null : (
            <div className="flex min-w-0 flex-col gap-1.5">
              <div className="flex items-baseline justify-between gap-3 text-label-sm">
                <span className="text-muted">{formatBytes(downloaded)} of {formatBytes(record.totalBytes)}</span>
                <span className="text-numeric text-foreground">{Math.round(progress * 100)}%</span>
              </div>
              <Progress value={progress} max={1} size="sm" showValue={false} aria-label="Download progress" />
            </div>
          )}
          <TransferMetrics row={row} rates={rates} ratio={ratio} />
        </div>
      }
      sections={sections}
    />
  );
}
