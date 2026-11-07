import { ActionBar, type ActionSet } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { Button } from "glass-ux/button";
import { CopyButton } from "glass-ux/copy-button";
import { Dialog } from "glass-ux/dialog";
import { FactList } from "glass-ux/fact-list";
import { IconActivity, IconArrowDown, IconArrowUp, IconClose, IconDisk, IconFile, IconInfo, IconPercent, IconSearch, IconUsers } from "glass-ux/icons";
import { Notice } from "glass-ux/notice";
import { Progress } from "glass-ux/progress";
import { ScrollArea } from "glass-ux/scroll-area";
import { TruncatedText } from "glass-ux/truncated-text";
import { VerticalTabs } from "glass-ux/vertical-tabs";
import type { ComponentProps, ReactNode } from "react";

import { torrentImport } from "@/lib/entities/torrent";
import { formatBytes, formatDateTime, formatSpeed } from "@/lib/format";

import { LiveStat, Stat } from "./TransferStat";
import { TorrentFiles, useTorrentFiles } from "./TorrentFiles";
import { IdentificationActivity, IdentifiedResults, useTorrentIdentification } from "./TorrentIdentification";
import { TorrentIdentity } from "./TorrentIdentity";
import type { DownloadRow } from "./useDownloads";
import type { RateHistory } from "./useRateHistory";

type Rates = RateHistory["byHash"] extends ReadonlyMap<string, infer Series> ? Series : never;

const fraction = (value: number) => Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0;

/** With the files listed below, the notice needn't list them too: "19 file(s) did not match any library item". */
function noticeText(error: string, filesListed: boolean) {
  const lead = error.split(": ")[0]!;
  return filesListed && /^\d+ file\(s\)/.test(lead) ? `${lead}. See the Files section.` : error;
}

function Copyable({ value, label }: { value: string; label: string }) {
  return (
    <span className="flex min-w-0 items-center gap-1">
      <TruncatedText middle numeric className="min-w-0 flex-1 font-mono text-label-sm">{value}</TruncatedText>
      <CopyButton value={value} label={label} size="sm" variant="ghost" />
    </span>
  );
}

function FactGroup({ title, children, className }: { title: string; children: ReactNode; className?: string }) {
  return (
    <section className={`torrent-facts flex min-w-0 flex-col gap-2 ${className ?? ""}`}>
      <h3 className="text-title-sm text-foreground">{title}</h3>
      {children}
    </section>
  );
}

function TorrentFacts({ row, ratio }: { row: DownloadRow; ratio: number }) {
  const { record, live } = row;
  const imported = torrentImport(record.postProcessStatus);
  const history = [
    { label: "Library", value: record.library?.name ?? "Not linked" },
    ...(record.season != null ? [{ label: "Pack", value: record.season === 0 ? "Specials" : `Season ${record.season}` }] : []),
    { label: "Added", value: formatDateTime(record.addedAt) },
    ...(record.completedAt ? [{ label: "Completed", value: formatDateTime(record.completedAt) }] : []),
    { label: "Import", value: imported.label },
  ];
  const sharing = [
    { label: "Share ratio", value: ratio.toFixed(2) },
    { label: "Uploaded", value: formatBytes(record.uploadedBytesTotal) },
    ...(record.minimumRatio ? [{ label: "Tracker minimum", value: `Ratio ${record.minimumRatio.toFixed(2)}${record.minimumSeedTimeMinutes ? `, ${Math.round(record.minimumSeedTimeMinutes / 60)} h seeding` : ""}` }] : []),
  ];
  return (
    <ScrollArea aria-label="Details" className="h-full min-h-0">
      <div className="grid max-w-4xl grid-cols-[repeat(auto-fit,minmax(18rem,1fr))] gap-x-10 gap-y-6">
        <FactGroup title="History"><FactList facts={history} /></FactGroup>
        <FactGroup title="Sharing"><FactList facts={sharing} /></FactGroup>
        <FactGroup title="On disk" className="col-span-full">
          <FactList facts={[
            { label: "Save path", value: <Copyable value={live?.savePath ?? record.savePath} label="Copy save path" /> },
            { label: "Info hash", value: <Copyable value={record.infoHash} label="Copy info hash" /> },
          ]} />
        </FactGroup>
        {record.postProcessError ? <p className="col-span-full whitespace-pre-wrap text-label-sm text-muted [overflow-wrap:anywhere]">{record.postProcessError}</p> : null}
      </div>
    </ScrollArea>
  );
}

/**
 * The one line under a download's name: its size and share ratio, then the transfer as it is
 * happening, sampled on this page because the server keeps no history. Unknown (a download the
 * client hasn't loaded) reads as unknown rather than as zero.
 */
function TorrentStats({ row, rates, ratio }: { row: DownloadRow; rates?: Rates; ratio: number }) {
  const { record, live } = row;
  return (
    <div role="group" aria-label="Transfer" className="flex min-w-0 flex-wrap items-center gap-x-4 gap-y-0.5 text-body-sm text-foreground">
      <Stat icon={<IconDisk size="sm" />} label="Size"><span className="text-numeric">{formatBytes(record.totalBytes)}</span></Stat>
      <Stat icon={<IconPercent size="sm" />} label="Share ratio"><span className="text-numeric">{ratio.toFixed(2)}</span></Stat>
      <LiveStat icon={<IconArrowDown size="sm" />} label="Download" value={live ? live.downloadSpeed ?? 0 : null} data={rates?.down} format={formatSpeed} />
      <LiveStat icon={<IconArrowUp size="sm" />} label="Upload" value={live ? live.uploadSpeed ?? 0 : null} data={rates?.up} format={formatSpeed} tone="success" />
      <LiveStat icon={<IconUsers size="sm" />} label="Peers" value={live ? live.peers ?? 0 : null} data={rates?.peers} tone="neutral" />
    </div>
  );
}

/** A download's dialog. Live values update in place while it is open. */
export function TorrentDetail({ row, rates, actions, open, onClose, finalFocus }: {
  row: DownloadRow;
  rates?: Rates;
  actions: ActionSet<DownloadRow>;
  open: boolean;
  onClose: () => void;
  finalFocus?: ComponentProps<typeof Dialog.Popup>["finalFocus"];
}) {
  const { record, live } = row;
  const imported = torrentImport(record.postProcessStatus);
  const progress = fraction(live?.progress ?? record.progress);
  const downloaded = live?.downloaded ?? record.downloadedBytes;
  const ratio = downloaded > 0 ? record.uploadedBytesTotal / downloaded : 0;
  const files = useTorrentFiles(row);
  const identification = useTorrentIdentification(record.infoHash);
  const needsChoice = identification.items.filter((item) => item.status === "needs_choice").length;
  // Once something has been identified, the results say what happened to each thing in the download.
  const identifiedInstead = identification.items.length > 0;
  const failure = !identifiedInstead && record.postProcessError && record.postProcessStatus !== "needs_review";
  const showStatus = !identifiedInstead && !failure;

  const sections = [
    { value: "identified", label: "Identified", icon: <IconSearch />, count: identification.items.length || undefined, marker: needsChoice > 0 ? "error" as const : undefined },
    { value: "activity", label: "Activity", icon: <IconActivity /> },
    { value: "files", label: "Files", icon: <IconFile />, count: files.files.length || undefined },
    { value: "details", label: "Details", icon: <IconInfo /> },
  ];

  return (
    <Dialog.Root open={open} onOpenChange={(next) => { if (!next) onClose(); }}>
      <Dialog.Popup size="full" kind="settings" aria-label="Download details" finalFocus={finalFocus} className="torrent-detail mx-auto max-w-[88rem]">
        <div className="flex min-w-0 shrink-0 items-center gap-3 px-(--inset-panel) pb-4 pt-(--inset-panel) max-sm:flex-wrap">
          <div className="min-w-0 flex-1 max-sm:order-last max-sm:basis-full"><TorrentIdentity tier="header" row={row} facts={<TorrentStats row={row} rates={rates} ratio={ratio} />} className="min-w-0" /></div>
          <div className="shrink-0 max-sm:ms-auto"><ActionBar set={actions} targets={[row]} label={`Actions for ${record.name}`} maxButtons={0} /></div>
          <Dialog.Close render={<Button variant="ghost" shape="circle" size="sm" icon={<IconClose />} aria-label="Close" className="shrink-0" />} />
        </div>
        <Dialog.Body flush className="flex min-h-0 flex-1 flex-col gap-3 px-(--inset-panel) pb-(--inset-panel)">
          {showStatus ? <div className="flex min-w-0 shrink-0 flex-wrap items-center gap-2"><Badge size="sm" marker="dot" tone={imported.tone}>{imported.label}</Badge></div> : null}
          {needsChoice > 0 ? (
            <Notice tone="warning" title="Needs a decision" className="min-w-0 shrink-0">
              <p>{needsChoice} {needsChoice === 1 ? "thing" : "things"} in this download could be more than one title. See Identified.</p>
            </Notice>
          ) : failure ? (
            <Notice tone={record.postProcessStatus === "failed" ? "danger" : "warning"} title={imported.label} className="min-w-0 shrink-0">
              <p className="line-clamp-2 whitespace-pre-wrap break-words [overflow-wrap:anywhere]">{noticeText(record.postProcessError!, files.files.length > 0)}</p>
            </Notice>
          ) : null}
          {progress >= 1 ? null : (
            <div className="flex min-w-0 shrink-0 flex-col gap-1.5">
              <div className="flex items-baseline justify-between gap-3 text-label-sm">
                <span className="text-muted">{formatBytes(downloaded)} of {formatBytes(record.totalBytes)}</span>
                <span className="text-numeric text-foreground">{Math.round(progress * 100)}%</span>
              </div>
              <Progress value={progress} max={1} size="sm" showValue={false} aria-label="Download progress" />
            </div>
          )}
          <VerticalTabs.Root aria-label="Download sections" items={sections} defaultValue="identified" className="min-h-0 flex-1">
            <VerticalTabs.Panel value="identified" keepMounted className="h-full min-h-0"><IdentifiedResults infoHash={record.infoHash} kind={record.identificationKind} state={identification} onChanged={files.refetch} /></VerticalTabs.Panel>
            <VerticalTabs.Panel value="activity" className="h-full min-h-0"><IdentificationActivity state={identification} /></VerticalTabs.Panel>
            <VerticalTabs.Panel value="files" className="h-full min-h-0"><TorrentFiles row={row} files={files} items={identification.items} /></VerticalTabs.Panel>
            <VerticalTabs.Panel value="details" className="h-full min-h-0"><TorrentFacts row={row} ratio={ratio} /></VerticalTabs.Panel>
          </VerticalTabs.Root>
        </Dialog.Body>
      </Dialog.Popup>
    </Dialog.Root>
  );
}
