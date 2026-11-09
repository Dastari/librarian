import { useMutation } from "@apollo/client/react";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, filters, IdentityCell, RelativeTime, TextCell, type DataTableSearch, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { IconHistory, IconTrash } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { useCallback, useMemo, useState } from "react";

import { Button, ConfirmDialog } from "@/components/ui";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { TableToolbar } from "@/components/ui/TableToolbar";
import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import { ClearPlayHistoryDocument, EntityUserListDocument, ImportPlayHistoryDocument, PlayHistoryPageDocument, type PlayHistoryOrderByInput, type PlayHistoryPageQuery, type PlayHistoryWhereInput } from "@/graphql/generated/graphql";
import { useTableSelection } from "@/hooks/useTableSelection";
import { tableUrlFilters, useTableUrlState } from "@/hooks/useTableUrlState";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { parseTimestamp } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { methodLabel } from "./nowPlaying";
import { formatPlayed } from "./statistics";

export type HistoryRow = PlayHistoryPageQuery["playHistories"]["edges"][number]["node"];

const KINDS = ["movie", "episode", "track", "audiobook"] as const;
const KIND_LABEL: Record<string, string> = { movie: "Movie", episode: "Episode", track: "Track", audiobook: "Audiobook" };
const KIND_GLYPH = { movie: ENTITY_KINDS.movie.icon, episode: ENTITY_KINDS.episode.icon, track: ENTITY_KINDS.track.icon, audiobook: ENTITY_KINDS.audiobook.icon } as const;

/** "Severance · S1 · E3", "Radiohead · OK Computer", "Arrival · 2016". */
export function historyLine(row: Pick<HistoryRow, "contentType" | "parentTitle" | "subtitle">): string {
  return [row.parentTitle, row.subtitle].filter(Boolean).join(" · ") || KIND_LABEL[row.contentType] || "";
}

function optionValues(raw: unknown): string[] {
  if (raw && typeof raw === "object" && "values" in raw && Array.isArray(raw.values)) return raw.values.filter((value): value is string => typeof value === "string");
  return [];
}

/** The table's state as a server query. */
export function historyVariables(state: DataTableState): { where: PlayHistoryWhereInput; orderBy: PlayHistoryOrderByInput[] } {
  const conditions: PlayHistoryWhereInput[] = [];
  for (const { id, value } of state.filters) {
    const values = optionValues(value);
    if (!values.length) continue;
    if (id === "kind") conditions.push({ contentType: { inList: values } });
    else if (id === "person") conditions.push({ userId: { inList: values } });
  }
  const query = state.search?.trim();
  if (query) conditions.push({ title: { contains: query } });
  const orderBy = state.sorting.flatMap(({ id, desc }): PlayHistoryOrderByInput[] => {
    const direction = desc ? "DESC" : "ASC";
    switch (id) {
      case "when": return [{ startedAt: direction }];
      case "title": return [{ title: direction }];
      case "played": return [{ secondsPlayed: direction }];
      default: return [];
    }
  });
  return { where: conditions.length ? { and: conditions } : {}, orderBy: orderBy.length ? orderBy : [{ startedAt: "DESC" }] };
}

function columnsFor(people: ReadonlyMap<string, string> | null) {
  const column = createDataTableColumns<HistoryRow>();
  return column.columns([
    column.accessor((row) => row.title, {
      id: "title", header: "Played", width: 30, priority: 1, cardRole: "title", sortable: true, pin: true,
      cell: ({ row }) => {
        const Glyph = KIND_GLYPH[row.original.contentType as keyof typeof KIND_GLYPH] ?? ENTITY_KINDS.movie.icon;
        return <IdentityCell icon={<Glyph />} title={row.original.title} subtitle={historyLine(row.original)} />;
      },
    }),
    column.accessor((row) => row.contentType, {
      id: "kind", header: "Kind", width: 9, priority: 4,
      filter: filters.options({ field: "contentType", nullable: false, operators: ["in"], options: KINDS.map((value) => ({ value, label: KIND_LABEL[value]! })) }),
      cell: ({ row }) => <TextCell muted>{KIND_LABEL[row.original.contentType] ?? row.original.contentType}</TextCell>,
    }),
    ...(people ? [column.accessor((row) => row.userId, {
      id: "person", header: "Person", width: 10, priority: 2,
      filter: filters.options({ field: "userId", nullable: false, operators: ["in"], options: [...people].map(([value, label]) => ({ value, label })) }),
      cell: ({ row }) => <TextCell>{people.get(row.original.userId) ?? "Removed user"}</TextCell>,
    })] : []),
    column.accessor((row) => row.secondsPlayed, {
      id: "played", header: "Time", width: 8, priority: 2, sortable: true, sortDescFirst: true,
      cell: ({ row }) => <TextCell muted={row.original.imported}>{row.original.imported ? "Imported" : formatPlayed(row.original.secondsPlayed)}</TextCell>,
    }),
    column.display({
      id: "finished", header: "Finished", width: 8, priority: 3, cardRole: "status",
      cell: ({ row }) => (row.original.completed ? <Badge size="sm" marker="dot" tone="success">Finished</Badge> : <TextCell muted>—</TextCell>),
    }),
    column.accessor((row) => row.deviceName ?? "", { id: "device", header: "Device", width: 12, priority: 3, cell: ({ row }) => <TextCell muted={!row.original.deviceName}>{row.original.deviceName ?? "—"}</TextCell> }),
    column.accessor((row) => row.playMethod ?? "", { id: "method", header: "Method", width: 9, priority: 5, cell: ({ row }) => <TextCell muted>{row.original.playMethod ? methodLabel(row.original.playMethod) : "—"}</TextCell> }),
    column.accessor((row) => row.startedAt, { id: "when", header: "When", width: 9, priority: 2, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.startedAt)} /> }),
  ]);
}

const search: DataTableSearch<HistoryRow> = { fields: [], placeholder: "Search history", toCondition: (query) => ({ title: { contains: query } }) };
const urlSchema = { sortableColumns: ["title", "played", "when"], filters: { kind: tableUrlFilters.options([...KINDS], ["in"]) } };
const defaultSorting = [{ id: "when", desc: true }];
const hiddenColumns = ["kind", "method"];
const noun = { one: "play", other: "plays" };
const getRowId = (row: HistoryRow) => row.id;
const getRowLabel = (row: HistoryRow) => row.title;

/**
 * Every play, newest first: what, how long was really played, on which device and whether it was
 * finished. Yours, or everyone's for an administrator. Plays can be removed one by one, and you can
 * clear your whole history.
 */
export function HistoryPage() {
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const users = useQuery(EntityUserListDocument, { variables: { page: { limit: 100, offset: 0 } }, skip: !isAdmin });
  const people = useMemo(() => (isAdmin ? new Map((users.data?.users.edges ?? []).map(({ node }) => [node.id, node.displayName || node.username])) : null), [isAdmin, users.data]);
  const columns = useMemo(() => columnsFor(people), [people]);
  const url = useTableUrlState({ namespace: "history", columns, ...urlSchema, defaultSorting });
  const baseState = useMemo<DataTableState>(() => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: {}, ...url.state }), [url.state]);
  const variables = useMemo(() => historyVariables(baseState), [baseState]);
  const list = useSettingsConnection(PlayHistoryPageDocument, variables, (data) => data.playHistories, { infinite: true, batchSize: 50, live: true });
  const selectedRows = useTableSelection({ scope: [user?.id, url.state], rows: list.rows, ready: !list.loading && !list.fetching });
  const state = { ...baseState, selection: selectedRows.selection };
  const onStateChange = (next: DataTableState, change: DataTableStateChange) => (change === "selection" ? selectedRows.onSelectionChange(next.selection) : url.onStateChange(next, change));
  const refetch = list.refetch;
  const refresh = useCallback(() => void refetch().catch(() => {}), [refetch]);

  const [clear, { loading: clearing }] = useMutation(ClearPlayHistoryDocument);
  const [runImport, { loading: importing }] = useMutation(ImportPlayHistoryDocument);
  const [confirmClear, setConfirmClear] = useState(false);
  const actions = useMemo(() => defineActions<HistoryRow>("play", [
    {
      id: "remove", label: (rows) => (rows.length === 1 ? "Remove from history" : `Remove ${rows.length} plays`), icon: IconTrash, group: "danger", priority: 9, bulk: true,
      visible: (rows) => isAdmin || rows.every((row) => row.userId === user?.id),
      confirm: (rows) => ({ title: rows.length === 1 ? `Remove ${rows[0]!.title} from history?` : `Remove ${rows.length} plays?`, body: "They no longer count in statistics. This cannot be undone.", confirmLabel: "Remove", tone: "danger" }),
      run: async (rows) => {
        assertSuccess((await clear({ variables: { where: { id: { inList: rows.map((row) => row.id) } } } })).data?.deletePlayHistories, "The plays could not be removed");
        refresh();
      },
      successMessage: (rows) => (rows.length === 1 ? "Removed from history" : `${rows.length} plays removed`),
    },
  ]), [clear, isAdmin, refresh, user?.id]);

  const onClear = async () => {
    if (!user) return;
    try {
      const result = assertSuccess((await clear({ variables: { where: { userId: { eq: user.id } } } })).data?.deletePlayHistories, "Your history could not be cleared");
      toast.success(`Cleared ${result.affectedCount ?? 0} plays`);
      setConfirmClear(false);
      refresh();
    } catch (error) {
      setConfirmClear(false);
      toast.danger(errorMessage(error));
    }
  };
  const onImport = async () => {
    try {
      const result = assertSuccess((await runImport({ variables: { allUsers: false } })).data?.importPlayHistory, "Watch progress could not be imported");
      toast.success(result.imported > 0 ? `Imported ${result.imported} watched ${result.imported === 1 ? "item" : "items"}` : "Nothing new to import");
      refresh();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <DataTable<HistoryRow>
        id={`librarian.${user?.id ?? "session"}.history`}
        label="Play history"
        queryKey={JSON.stringify([user?.id, variables])}
        height="fill"
        columns={columns}
        defaultHiddenColumns={hiddenColumns}
        rows={list.rows}
        getRowId={getRowId}
        getRowLabel={getRowLabel}
        loading={list.loading}
        refetching={list.fetching && !list.loading}
        rowCount={list.totalCount ?? 0}
        state={state}
        onStateChange={onStateChange}
        infinite={{ revision: list.revision, hasMore: list.hasMore, onLoadMore: list.loadMore, loadingMore: list.loadingMore, error: list.moreError?.message }}
        search={search}
        actions={actions}
        renderBulkActions={(rows) => <TableBulkActions set={actions} rows={rows} noun={noun} />}
        selectable
        live
        noun={noun}
        error={list.error && list.rows.length === 0 ? "Couldn't load the history. Retry to reconnect." : undefined}
        onRetry={refresh}
        emptyMessage="Nothing played yet. Plays are recorded from here on, on every device."
        toolbar={
          <TableToolbar>
            <Button size="sm" variant="secondary" isPending={importing} onPress={() => void onImport()}><IconHistory /> Import watched</Button>
            <Button size="sm" variant="danger-soft" isDisabled={list.rows.length === 0} onPress={() => setConfirmClear(true)}><IconTrash /> Clear my history</Button>
          </TableToolbar>
        }
      />
      <ConfirmDialog
        isOpen={confirmClear}
        onOpenChange={setConfirmClear}
        title="Clear your play history?"
        description="Every play of yours is deleted and your statistics start again from nothing. Watch progress and watched marks stay. This cannot be undone."
        confirmLabel="Clear history"
        destructive
        isPending={clearing}
        onConfirm={onClear}
      />
    </div>
  );
}
