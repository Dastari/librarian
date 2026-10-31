import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, filters, IdentityCell, NumericCell, RelativeTime, SparklineCell, TextCell, type DataTableSearch, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { EntityIdentity } from "glass-ux/entity";
import { IconPlus, IconRefresh } from "glass-ux/icons";
import { LevelGauge, type GaugeLevel } from "glass-ux/level-gauge";
import { MeterCell } from "glass-ux/meter-cell";
import { Metric } from "glass-ux/metric";
import { Segmented } from "glass-ux/segmented";
import { parseAsStringEnum, useQueryState } from "nuqs";
import { useCallback, useMemo, useState } from "react";

import { Button, Dialog } from "@/components/ui";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { EntityLibraryListDocument } from "@/graphql/generated/graphql";
import { tableUrlFilters, useTableUrlState } from "@/hooks/useTableUrlState";
import { useMediaQuery } from "@/hooks/useMediaQuery";
import { usePrintMode } from "@/hooks/usePrintMode";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { ratioLevel, torrentImport, torrentStatus, torrentTarget, TORRENT_IMPORT_STATES, TORRENT_STATES, type RatioLevel } from "@/lib/entities/torrent";
import { formatBytes, formatSpeed } from "@/lib/format";
import { cn } from "@/lib/utils";

import { AddTorrentDialog } from "./AddTorrentDialog";
import { LinkTorrentDialog } from "./LinkTorrentDialog";
import { TorrentDetail } from "./TorrentDetail";
import { DOWNLOAD_PRESETS, useDownloads, type DownloadPreset, type DownloadRow } from "./useDownloads";
import { useRateHistory, type RateSample } from "./useRateHistory";
import { useTorrentActions } from "./useTorrentActions";

const PRESET_LABEL: Record<DownloadPreset, string> = { all: "All", downloading: "Downloading", seeding: "Seeding", paused: "Paused", failed: "Needs attention" };
const RATIO_LEVELS: readonly GaugeLevel<RatioLevel>[] = [
  { value: "short", label: "Short", tone: "warning" },
  { value: "close", label: "Close", tone: "info" },
  { value: "met", label: "Met", tone: "success" },
];
const MB = 1024 * 1024;
const TorrentGlyph = ENTITY_KINDS.torrent.icon;

const progressOf = (row: DownloadRow) => row.live?.progress ?? row.record.progress;
const stateOf = (row: DownloadRow) => row.live?.state ?? row.record.state;
const ratioOf = (row: DownloadRow) => (row.record.downloadedBytes > 0 ? row.record.uploadedBytesTotal / row.record.downloadedBytes : 0);

function columnsFor(rates: ReturnType<typeof useRateHistory>, libraries: Array<{ id: string; name: string }>) {
  const column = createDataTableColumns<DownloadRow>();
  return column.columns([
    column.accessor((row) => row.record.name, {
      id: "name", header: "Download", width: 26, priority: 1, cardRole: "title", sortable: true, pin: true,
      cell: ({ row }) => {
        const state = torrentStatus(stateOf(row.original));
        const target = torrentTarget(row.original.record);
        return <IdentityCell icon={<TorrentGlyph />} title={row.original.record.name} subtitle={[state.label, target?.label ?? "Not linked", formatBytes(row.original.record.totalBytes)].join(" · ")} status={state.status} statusLabel={state.label} />;
      },
    }),
    column.accessor((row) => stateOf(row), {
      id: "state", header: "State", width: 8, priority: 4, cardRole: "hidden", sortable: true,
      filter: filters.options({ field: "state", nullable: false, operators: ["in"], options: TORRENT_STATES.map((value) => ({ value, label: torrentStatus(value).label })) }),
      cell: ({ row }) => <TextCell muted>{torrentStatus(stateOf(row.original)).label}</TextCell>,
    }),
    column.accessor(progressOf, {
      id: "progress", header: "Progress", width: 11, priority: 2, sortable: true, align: "end",
      cell: ({ row }) => {
        const progress = progressOf(row.original);
        return <MeterCell label="Progress" value={progress} max={1} tone="data" warningAt={2} dangerAt={2} valueText={progress >= 1 ? formatBytes(row.original.record.totalBytes) : `${Math.round(progress * 100)}% of ${formatBytes(row.original.record.totalBytes)}`} />;
      },
    }),
    column.display({
      id: "down", header: "Down", width: 9, priority: 3, align: "end", flash: false,
      cell: ({ row }) => {
        const series = rates.byHash.get(row.original.record.infoHash.toLowerCase())?.down ?? [];
        return row.original.live ? <SparklineCell values={series.map((value) => value / MB)} label="Download speed, last two minutes" unit="MB/s" value={(row.original.live.downloadSpeed ?? 0) / MB} tone={row.original.live.downloadSpeed ? "info" : "neutral"} /> : <TextCell muted>—</TextCell>;
      },
    }),
    column.display({
      id: "up", header: "Up", width: 9, priority: 3, align: "end", flash: false,
      cell: ({ row }) => {
        const series = rates.byHash.get(row.original.record.infoHash.toLowerCase())?.up ?? [];
        return row.original.live ? <SparklineCell values={series.map((value) => value / MB)} label="Upload speed, last two minutes" unit="MB/s" value={(row.original.live.uploadSpeed ?? 0) / MB} tone={row.original.live.uploadSpeed ? "success" : "neutral"} /> : <TextCell muted>—</TextCell>;
      },
    }),
    column.display({
      id: "peers", header: "Peers", width: 5, priority: 4, align: "end", grow: false,
      cell: ({ row }) => row.original.live ? <NumericCell value={row.original.live.peers} /> : <TextCell muted>—</TextCell>,
    }),
    column.accessor(ratioOf, {
      id: "ratio", header: "Share ratio", width: 11, priority: 3,
      cell: ({ row }) => {
        const ratio = ratioOf(row.original);
        const target = row.original.record.minimumRatio;
        return (
          <span className="flex min-w-0 items-center gap-2" title={target ? `Tracker minimum ${target.toFixed(2)}` : "No tracker minimum; 1.00 is the goal"}>
            <span className="text-numeric shrink-0 tabular-nums">{ratio.toFixed(2)}</span>
            <LevelGauge label="Share ratio" levels={RATIO_LEVELS} value={ratioLevel(ratio, target)} />
          </span>
        );
      },
    }),
    column.display({
      id: "target", header: "For", width: 14, priority: 2, cardRole: "subtitle",
      cell: ({ row }) => {
        const target = torrentTarget(row.original.record);
        if (!target) return <TextCell muted>Not linked</TextCell>;
        const Glyph = ENTITY_KINDS[target.kind as keyof typeof ENTITY_KINDS]?.icon;
        return <EntityIdentity tier="chip" size="sm" kind={ENTITY_KINDS[target.kind as keyof typeof ENTITY_KINDS]?.noun} name={target.label} href={target.href} icon={Glyph ? <Glyph /> : undefined} preview={false} />;
      },
    }),
    column.accessor((row) => row.record.postProcessStatus ?? "waiting", {
      id: "import", header: "Import", width: 9, priority: 2, cardRole: "status",
      filter: filters.options({ field: "postProcessStatus", nullable: false, operators: ["in"], options: [...TORRENT_IMPORT_STATES, "waiting"].map((value) => ({ value, label: torrentImport(value === "waiting" ? null : value).label })) }),
      cell: ({ row }) => { const state = torrentImport(row.original.record.postProcessStatus); return <Badge size="sm" marker="dot" tone={state.tone}>{state.label}</Badge>; },
    }),
    column.accessor((row) => row.record.libraryId ?? "none", {
      id: "library", header: "Library", width: 9, priority: 5, cardRole: "hidden",
      filter: filters.options({ field: "libraryId", nullable: false, operators: ["in"], options: [{ value: "none", label: "Not linked" }, ...libraries.map((library) => ({ value: library.id, label: library.name }))] }),
      cell: ({ row }) => <TextCell muted>{row.original.record.library?.name ?? "Not linked"}</TextCell>,
    }),
    column.accessor((row) => row.record.totalBytes, { id: "size", header: "Size", width: 7, priority: 5, align: "end", sortable: true, sortDescFirst: true, cell: ({ row }) => <TextCell muted>{formatBytes(row.original.record.totalBytes)}</TextCell> }),
    column.accessor((row) => row.record.addedAt, { id: "added", header: "Added", width: 8, priority: 4, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={row.original.record.addedAt} /> }),
    column.accessor((row) => row.record.infoHash, { id: "hash", header: "Info hash", width: 16, priority: 5, cardRole: "hidden", cell: ({ row }) => <TextCell identifier>{row.original.record.infoHash}</TextCell> }),
  ]);
}

const search: DataTableSearch<DownloadRow> = { fields: [], placeholder: "Search downloads", toCondition: (query) => ({ name: { contains: query } }) };
const urlSchema = {
  sortableColumns: ["name", "progress", "size", "added", "state"],
  filters: { state: tableUrlFilters.options(TORRENT_STATES, ["in"]), import: tableUrlFilters.options([...TORRENT_IMPORT_STATES, "waiting"], ["in"]), library: tableUrlFilters.text() },
};
const defaultSorting = [{ id: "added", desc: true }];
const hiddenColumns = ["state", "library", "size", "hash"];
const noun = { one: "download", other: "downloads" };
const getRowId = (row: DownloadRow) => row.id;
const getRowLabel = (row: DownloadRow) => row.record.name;

/**
 * Downloads: every stored torrent in one bounded table that loads as it scrolls, with the
 * client's live speeds over it. Presets (with their counts) are server filters; live totals
 * sit beside them as metrics rather than a strip of status cards (docs/entity-tables.md).
 */
export function DownloadsPage() {
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const printing = usePrintMode();
  const wide = useMediaQuery("(min-width: 1280px)");
  const [preset, setPreset] = useQueryState("show", parseAsStringEnum<DownloadPreset>([...DOWNLOAD_PRESETS]).withDefault("all").withOptions({ history: "push" }));
  const libraries = useCompleteConnection(EntityLibraryListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.libraries);
  const libraryOptions = useMemo(() => libraries.rows.map((library) => ({ id: library.id, name: library.name })), [libraries.rows]);

  const baseColumns = useMemo(() => columnsFor({ byHash: new Map(), total: { down: [], up: [] } }, libraryOptions), [libraryOptions]);
  const url = useTableUrlState({ namespace: "downloads", columns: baseColumns, ...urlSchema, defaultSorting });
  const state = useMemo<DataTableState>(() => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: {}, ...url.state }), [url.state]);
  const onStateChange = (next: DataTableState, change: DataTableStateChange) => url.onStateChange(next, change);

  const downloads = useDownloads(state, preset);
  const samples = useMemo(() => new Map<string, RateSample>(downloads.live.map((torrent) => [torrent.infoHash.toLowerCase(), { down: torrent.downloadSpeed ?? 0, up: torrent.uploadSpeed ?? 0 }])), [downloads.live]);
  const rates = useRateHistory(samples);
  const columns = useMemo(() => columnsFor(rates, libraryOptions), [rates, libraryOptions]);
  const totals = useMemo(() => downloads.live.reduce((sum, torrent) => ({ down: sum.down + (torrent.downloadSpeed ?? 0), up: sum.up + (torrent.uploadSpeed ?? 0) }), { down: 0, up: 0 }), [downloads.live]);

  const [adding, setAdding] = useState(false);
  const [linking, setLinking] = useState<DownloadRow | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = downloads.rows.find((row) => row.id === selectedId) ?? null;
  const refetch = downloads.refetch;
  const onChanged = useCallback(() => refetch(), [refetch]);
  const actions = useTorrentActions({ onLink: setLinking, onChanged }, isAdmin);

  return (
    <div data-print-flow className="page-gutter flex min-h-0 flex-1 flex-col gap-4 overflow-hidden py-4 md:py-6 [@media(max-height:35rem)]:gap-2 [@media(max-height:35rem)]:py-2">
      <h1 className="sr-only">Downloads</h1>
      <div className="flex shrink-0 flex-wrap items-center gap-x-6 gap-y-3">
        <Segmented<DownloadPreset>
          aria-label="Show downloads"
          value={preset}
          onValueChange={(value) => void setPreset(value === "all" ? null : value)}
          options={DOWNLOAD_PRESETS.map((value) => ({ value, label: downloads.counts[value] !== undefined ? `${PRESET_LABEL[value]} ${downloads.counts[value]}` : PRESET_LABEL[value] }))}
        />
        <div className="ms-auto flex min-w-0 items-center gap-3">
          <div className="h-12 w-60 max-lg:hidden"><Metric label="Download" value={totals.down} data={rates.total.down} format={formatSpeed} range="Last two minutes" tone="info" better="none" /></div>
          <div className="h-12 w-60 max-lg:hidden"><Metric label="Upload" value={totals.up} data={rates.total.up} format={formatSpeed} range="Last two minutes" tone="success" better="none" /></div>
          {isAdmin ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus /> Add torrent</Button> : null}
        </div>
      </div>
      <div data-print-flow className={cn("grid min-h-0 flex-1 grid-rows-[minmax(0,1fr)] gap-6 overflow-hidden", selected && wide && "xl:grid-cols-[minmax(0,3fr)_minmax(22rem,2fr)]")}>
        <div className="flex min-h-0 min-w-0 flex-col">
          <DataTable<DownloadRow>
            id={`librarian.${user?.id ?? "session"}.downloads`}
            label="Downloads"
            queryKey={JSON.stringify([user?.id, downloads.variables])}
            height="fill"
            columns={columns}
            defaultHiddenColumns={hiddenColumns}
            rows={downloads.rows}
            getRowId={getRowId}
            getRowLabel={getRowLabel}
            loading={downloads.list.loading}
            refetching={downloads.list.fetching && !downloads.list.loading}
            rowCount={downloads.list.totalCount}
            state={state}
            onStateChange={onStateChange}
            infinite={{ revision: downloads.list.revision, hasMore: !printing && downloads.list.hasMore, onLoadMore: downloads.list.loadMore, loadingMore: downloads.list.loadingMore, error: downloads.list.moreError?.message }}
            search={search}
            actions={actions}
            selectable={isAdmin}
            live
            noun={noun}
            onRowOpen={(row) => setSelectedId(row.id)}
            rowOpen="click"
            error={downloads.list.error && downloads.rows.length === 0 ? "Couldn't load downloads. Retry to reconnect." : undefined}
            onRetry={refetch}
            emptyMessage={preset === "all" ? "No downloads. Add a torrent or magnet link, or find a release from a movie or show page." : `Nothing ${PRESET_LABEL[preset].toLowerCase()}`}
            toolbar={<Button size="sm" variant="ghost" isIconOnly aria-label="Refresh downloads" isPending={downloads.list.fetching} onPress={refetch}><IconRefresh /></Button>}
          />
        </div>
        {selected && wide ? <div data-print-flow className="scrollbar-thin min-h-0 overflow-y-auto"><TorrentDetail row={selected} actions={actions} onClose={() => setSelectedId(null)} /></div> : null}
      </div>
      <Dialog isOpen={Boolean(selected) && !wide} onOpenChange={(open) => !open && setSelectedId(null)} title="Download details" size="md">
        {selected ? <TorrentDetail row={selected} actions={actions} onClose={() => setSelectedId(null)} /> : null}
      </Dialog>

      <AddTorrentDialog isOpen={adding} onOpenChange={setAdding} onAdded={refetch} />
      <LinkTorrentDialog row={linking} onClose={() => setLinking(null)} onLinked={refetch} />
    </div>
  );
}
