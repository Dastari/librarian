import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { Checkbox } from "glass-ux/checkbox";
import { IconArrowDown, IconArrowUp, IconDownload, IconLink, IconPlayerPause, IconPlayerPlay, IconPlus, IconRefresh, IconTrash, IconWand } from "@tabler/icons-react";
import { useMemo, useState } from "react";
import { parseAsStringEnum, useQueryState } from "nuqs";

import { Button, ConfirmDialog, DataTable, type DataTableColumn, type DataTableRowAction, Dialog, EmptyState, ErrorState, GlassSegmented, PageHeader, StatTile, StatusChip } from "@/components/ui";
import { useMediaQuery } from "@/hooks/useMediaQuery";
import { PauseTorrentDocument, ProcessSourceDocument, RemoveTorrentDocument, ResumeTorrentDocument } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes, formatSpeed } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { torrentState } from "@/lib/status";
import { cn } from "@/lib/utils";

import { AddTorrentDialog } from "./AddTorrentDialog";
import { LinkTorrentDialog } from "./LinkTorrentDialog";
import { TorrentDetail } from "./TorrentDetail";
import { useLiveTorrents, type TorrentRow } from "./useLiveTorrents";

const FILTERS = ["all", "active", "seeding", "paused", "done"] as const;
type Filter = (typeof FILTERS)[number];

function matchesFilter(row: TorrentRow, filter: Filter): boolean {
  const state = row.live.state.toLowerCase();
  switch (filter) {
    case "active":
      return state === "downloading" || state === "checking" || state === "initializing" || state === "queued";
    case "seeding":
      return state === "seeding";
    case "paused":
      return state === "paused";
    case "done":
      return row.live.progress >= 1 || state === "completed";
    default:
      return true;
  }
}

export function DownloadsPage() {
  const isAdmin = useIsAdmin();
  const torrents = useLiveTorrents();
  const [filter, setFilter] = useQueryState("show", parseAsStringEnum<Filter>([...FILTERS]).withDefault("all").withOptions({ history: "push" }));
  const wide = useMediaQuery("(min-width: 1280px)");
  const [adding, setAdding] = useState(false);
  const [linking, setLinking] = useState<TorrentRow | null>(null);
  const [removing, setRemoving] = useState<TorrentRow | null>(null);
  const [deleteFiles, setDeleteFiles] = useState(false);
  const [selected, setSelected] = useState<TorrentRow | null>(null);

  const [pause] = useMutation(PauseTorrentDocument);
  const [resume] = useMutation(ResumeTorrentDocument);
  const [remove, { loading: removingBusy }] = useMutation(RemoveTorrentDocument);
  const [process] = useMutation(ProcessSourceDocument);

  const rows = useMemo(() => torrents.rows.filter((row) => matchesFilter(row, filter)), [torrents.rows, filter]);
  const totals = useMemo(() => torrents.rows.reduce((acc, row) => ({ down: acc.down + row.live.downloadSpeed, up: acc.up + row.live.uploadSpeed, active: acc.active + (row.live.state.toLowerCase() === "downloading" ? 1 : 0), seeding: acc.seeding + (row.live.state.toLowerCase() === "seeding" ? 1 : 0) }), { down: 0, up: 0, active: 0, seeding: 0 }), [torrents.rows]);

  const act = async (label: string, task: () => Promise<{ success: boolean; error?: string | null } | null | undefined>) => {
    try {
      assertSuccess(await task(), `${label} failed`);
      torrents.refetch();
    } catch (error) {
      toast.danger(errorMessage(error, `${label} failed`));
    }
  };

  const actions = useMemo<Array<DataTableRowAction<TorrentRow>>>(
    () =>
      isAdmin
        ? [
            { key: "pause", label: "Pause", icon: <IconPlayerPause size={16} />, hidden: (row) => row.live.state.toLowerCase() === "paused", onAction: (row) => act("Pause", async () => (await pause({ variables: { id: row.live.id } })).data?.pauseTorrent) },
            { key: "resume", label: "Resume", icon: <IconPlayerPlay size={16} />, hidden: (row) => row.live.state.toLowerCase() !== "paused", onAction: (row) => act("Resume", async () => (await resume({ variables: { id: row.live.id } })).data?.resumeTorrent) },
            { key: "link", label: "Link to library…", icon: <IconLink size={16} />, onAction: (row) => setLinking(row) },
            {
              key: "process",
              label: "Import now",
              icon: <IconWand size={16} />,
              hidden: (row) => row.live.progress < 1 || !row.record,
              onAction: (row) =>
                act("Import", async () => {
                  const result = (await process({ variables: { sourceId: row.record!.id, sourceType: "torrent" } })).data?.processSource;
                  if (result?.success) toast.success(`${result.filesProcessed} files imported${result.filesFailed ? `, ${result.filesFailed} failed` : ""}`);
                  return result;
                }),
            },
            { key: "remove", label: "Remove", icon: <IconTrash size={16} />, destructive: true, onAction: (row) => { setDeleteFiles(false); setRemoving(row); } },
          ]
        : [],
    [isAdmin, pause, process, resume],
  );

  const columns = useMemo<Array<DataTableColumn<TorrentRow>>>(
    () => [
      {
        id: "name",
        header: "Download",
        sortable: true,
        value: (row) => row.live.name,
        wrap: true,
        cell: (row) => {
          const state = torrentState(row.live.state);
          return (
            <span className="flex min-w-0 flex-col gap-1.5">
              <span className="flex items-center gap-2">
                <span className={cn("size-2 shrink-0 rounded-full", state.dot, row.live.state.toLowerCase() === "downloading" && "animate-pulse-soft")} />
                <span className="break-words text-body-sm text-foreground">{row.live.name}</span>
              </span>
              <span className="h-1 w-full overflow-hidden rounded-full bg-surface-tertiary">
                <span className={cn("block h-full rounded-full", row.live.progress >= 1 ? "bg-success" : "bg-info")} style={{ width: `${Math.min(100, row.live.progress * 100)}%` }} />
              </span>
              <span className="break-words text-label-sm text-muted">
                {state.label} · {Math.round(row.live.progress * 100)}% · {formatBytes(row.live.downloaded)} of {formatBytes(row.live.size)}
                {row.record?.season !== null && row.record?.season !== undefined ? ` · S${String(row.record.season).padStart(2, "0")} pack` : ""}
                {row.record?.postProcessStatus ? ` · ${row.record.postProcessStatus}` : ""}
              </span>
            </span>
          );
        },
      },
      { id: "down", sortable: true, value: (row) => row.live.downloadSpeed, header: "Down", size: 110, align: "end", numeric: true, cell: (row) => <span className={row.live.downloadSpeed ? "text-info" : "text-muted"}>{formatSpeed(row.live.downloadSpeed)}</span> },
      { id: "up", sortable: true, value: (row) => row.live.uploadSpeed, header: "Up", size: 110, align: "end", numeric: true, hideBelow: "sm", cell: (row) => <span className={row.live.uploadSpeed ? "text-success" : "text-muted"}>{formatSpeed(row.live.uploadSpeed)}</span> },
      { id: "peers", sortable: true, value: (row) => row.live.peers, header: "Peers", size: 80, align: "end", numeric: true, hideBelow: "md", cell: (row) => row.live.peers },
      { id: "ratio", sortable: true, value: (row) => row.live.downloaded ? row.live.uploaded / row.live.downloaded : 0, header: "Ratio", size: 80, align: "end", numeric: true, hideBelow: "lg", cell: (row) => (row.live.downloaded ? (row.live.uploaded / row.live.downloaded).toFixed(2) : "0.00") },
      { id: "linked", header: "Library", size: 130, hideBelow: "md", cell: (row) => (row.record?.libraryId ? <StatusChip minimal status={{ label: row.record.movieId ? "Movie" : row.record.showId ? "Show" : row.record.albumId ? "Album" : row.record.audiobookId ? "Audiobook" : "Linked", tone: "success", dot: "bg-success" }} /> : <span className="text-muted">Unlinked</span>) },
    ],
    [],
  );

  return (
    <div data-print-flow className="page-gutter flex min-h-0 flex-1 flex-col gap-4 overflow-hidden py-4 md:gap-6 md:py-6 [@media(max-height:35rem)]:gap-2 [@media(max-height:35rem)]:py-2">
      <PageHeader
        title="Downloads"
        titleVisibility="accessible"
        meta={<span className="inline-flex flex-wrap items-center gap-3"><span className="inline-flex items-center gap-1 text-info"><IconArrowDown size={14} /> {formatSpeed(totals.down)}</span><span className="inline-flex items-center gap-1 text-success"><IconArrowUp size={14} /> {formatSpeed(totals.up)}</span><span className="text-muted sm:hidden [@media(max-height:35rem)]:inline">{totals.active} downloading · {totals.seeding} seeding</span></span>}
        actions={
          <>
            <Button variant="ghost" isIconOnly aria-label="Refresh" onPress={torrents.refetch}>
              <IconRefresh size={18} />
            </Button>
            {isAdmin ? (
              <Button variant="primary" onPress={() => setAdding(true)}>
                <IconPlus size={16} /> Add torrent
              </Button>
            ) : null}
          </>
        }
      />
      <div className="hidden shrink-0 grid-cols-2 gap-3 sm:grid lg:grid-cols-4 [@media(max-height:35rem)]:hidden">
        <StatTile label="Downloading" value={totals.active} icon={IconArrowDown} tone="accent" />
        <StatTile label="Seeding" value={totals.seeding} icon={IconArrowUp} tone="success" />
        <StatTile label="Download speed" value={formatSpeed(totals.down)} />
        <StatTile label="Upload speed" value={formatSpeed(totals.up)} />
      </div>
      <GlassSegmented<Filter> className="shrink-0" ariaLabel="Download filter" value={filter} onChange={setFilter} items={FILTERS.map((option) => ({ key: option, label: option.charAt(0).toUpperCase() + option.slice(1) }))} />
      {torrents.recordError && torrents.rows.length > 0 ? <ErrorState compact error={torrents.recordError} onRetry={torrents.refetch} /> : null}
      <div data-print-flow className={cn("grid min-h-0 flex-1 grid-rows-[minmax(0,1fr)] gap-6 overflow-hidden", selected && "xl:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]")}>
        <DataTable<TorrentRow>
          height="fill"
          columns={columns}
          rows={rows}
          getRowId={(row) => row.live.infoHash}
          isLoading={torrents.loading}
          dataSource="complete"
          rowActions={actions}
          noun="downloads"
          onRowClick={(row) => setSelected(row)}
          error={torrents.error && rows.length === 0 ? <ErrorState error={torrents.error} onRetry={torrents.refetch} /> : undefined}
          emptyState={<EmptyState icon={IconDownload} title={filter === "all" ? "No downloads" : `Nothing ${filter}`} description={filter === "all" ? "Add a torrent or magnet link, or find a release from a movie or show page." : undefined} action={isAdmin && filter === "all" ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size={16} /> Add torrent</Button> : undefined} />}
        />
        {selected && wide ? <div data-print-flow className="scrollbar-thin min-h-0 overflow-y-auto"><TorrentDetail row={torrents.rows.find((row) => row.live.infoHash === selected.live.infoHash) ?? selected} onClose={() => setSelected(null)} /></div> : null}
      </div>
      <Dialog isOpen={Boolean(selected) && !wide} onOpenChange={(open) => !open && setSelected(null)} title="Download details" size="md">
        {selected ? <TorrentDetail row={torrents.rows.find((row) => row.live.infoHash === selected.live.infoHash) ?? selected} onClose={() => setSelected(null)} /> : null}
      </Dialog>

      <AddTorrentDialog isOpen={adding} onOpenChange={setAdding} onAdded={torrents.refetch} />
      <LinkTorrentDialog row={linking} onClose={() => setLinking(null)} onLinked={torrents.refetch} />
      <ConfirmDialog
        isOpen={Boolean(removing)}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={`Remove ${removing?.live.name ?? "download"}?`}
        description="Stops the transfer and forgets the torrent. Files already imported into a library are never affected."
        confirmLabel={deleteFiles ? "Remove and delete files" : "Remove"}
        destructive
        isPending={removingBusy}
        onConfirm={async () => {
          if (!removing) return;
          await act("Remove", async () => (await remove({ variables: { id: removing.live.id, deleteFiles } })).data?.removeTorrent);
          setRemoving(null);
        }}
      >
        <Checkbox checked={deleteFiles} onCheckedChange={setDeleteFiles} disabled={removingBusy} label="Also delete the downloaded files" className="mt-3" />
      </ConfirmDialog>
    </div>
  );
}
