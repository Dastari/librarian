import { useMutation } from "@apollo/client/react";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, filters, IdentityCell, RelativeTime, TextCell, type DataTableSearch, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { IconCheck, IconRefresh } from "glass-ux/icons";
import { Segmented } from "glass-ux/segmented";
import { parseAsStringEnum, useQueryState } from "nuqs";
import { useCallback, useMemo, useState } from "react";

import { EntityChip } from "@/components/entities/EntityChip";
import { Button, Dialog } from "@/components/ui";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { EntityLibraryListDocument, EntityNotificationUpdateDocument, EntityNotificationUpdateManyDocument } from "@/graphql/generated/graphql";
import { useMediaQuery } from "@/hooks/useMediaQuery";
import { usePrintMode } from "@/hooks/usePrintMode";
import { tableUrlFilters, useTableUrlState } from "@/hooks/useTableUrlState";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { ENTITY_KINDS, type EntityKindName } from "@/lib/entities/kinds";
import { humanize } from "@/lib/entities/linked";
import { notificationDecision, notificationLevel, notificationSubjects, NOTIFICATION_LEVELS } from "@/lib/entities/notification";
import { parseTimestamp } from "@/lib/format";
import { cn } from "@/lib/utils";

import { ActivityDetail } from "./ActivityDetail";
import { ACTIVITY_CATEGORIES, ACTIVITY_PRESETS, useActivity, type ActivityPreset, type ActivityRow } from "./useActivity";
import { useNotificationActions } from "./useNotificationActions";

const PRESET_LABEL: Record<ActivityPreset, string> = { unread: "Unread", action: "Needs action", all: "Everything" };
const NotificationGlyph = ENTITY_KINDS.notification.icon;

function columnsFor(libraries: Array<{ id: string; name: string }>) {
  const column = createDataTableColumns<ActivityRow>();
  return column.columns([
    column.accessor((row) => row.title, {
      id: "title", header: "Notification", width: 28, priority: 1, cardRole: "title", sortable: true, pin: true,
      cell: ({ row }) => {
        const level = notificationLevel(row.original.notificationType);
        const subject = notificationSubjects(row.original)[0];
        const Glyph = subject ? ENTITY_KINDS[subject.kind as EntityKindName]?.icon ?? NotificationGlyph : NotificationGlyph;
        return <IdentityCell icon={<Glyph />} title={row.original.title} subtitle={[humanize(row.original.category), subject?.label].filter(Boolean).join(" · ")} status={row.original.readAt ? undefined : level.status} statusLabel={row.original.readAt ? undefined : `Unread · ${level.label}`} />;
      },
    }),
    column.accessor((row) => row.notificationType, {
      id: "level", header: "Level", width: 9, priority: 2, cardRole: "status", sortable: true,
      filter: filters.options({ field: "notificationType", nullable: false, operators: ["in"], options: NOTIFICATION_LEVELS.map((value) => ({ value, label: notificationLevel(value).label })) }),
      cell: ({ row }) => { const level = notificationLevel(row.original.notificationType); return <Badge size="sm" marker="dot" tone={level.tone}>{level.label}</Badge>; },
    }),
    column.display({
      id: "decision", header: "Decision", width: 10, priority: 3,
      cell: ({ row }) => { const decision = notificationDecision(row.original); return decision ? <Badge size="sm" tone={decision.tone}>{decision.label}</Badge> : <TextCell muted>—</TextCell>; },
    }),
    column.display({
      id: "subject", header: "About", width: 16, priority: 2, cardRole: "subtitle",
      cell: ({ row }) => { const subject = notificationSubjects(row.original)[0]; return subject ? <EntityChip entity={subject} /> : <TextCell muted>—</TextCell>; },
    }),
    column.accessor((row) => row.message, { id: "message", header: "Details", width: 26, priority: 5, cardRole: "hidden", cell: ({ row }) => <TextCell muted>{row.original.message}</TextCell> }),
    column.accessor((row) => row.category, {
      id: "category", header: "Category", width: 9, priority: 4, sortable: true,
      filter: filters.options({ field: "category", nullable: false, operators: ["in"], options: ACTIVITY_CATEGORIES.map((value) => ({ value, label: humanize(value) })) }),
      cell: ({ row }) => <TextCell muted>{humanize(row.original.category)}</TextCell>,
    }),
    column.accessor((row) => row.libraryId ?? "", {
      id: "library", header: "Library", width: 9, priority: 5, cardRole: "hidden",
      filter: filters.options({ field: "libraryId", nullable: false, operators: ["in"], options: libraries.map((library) => ({ value: library.id, label: library.name })) }),
      cell: ({ row }) => <TextCell muted>{row.original.library?.name ?? "—"}</TextCell>,
    }),
    column.accessor((row) => row.createdAt, { id: "when", header: "When", width: 8, priority: 2, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.createdAt)} /> }),
  ]);
}

const search: DataTableSearch<ActivityRow> = { fields: [], placeholder: "Search activity", toCondition: (query) => ({ title: { contains: query } }) };
const urlSchema = {
  sortableColumns: ["title", "level", "category", "when"],
  filters: { level: tableUrlFilters.options(NOTIFICATION_LEVELS, ["in"]), category: tableUrlFilters.options([...ACTIVITY_CATEGORIES], ["in"]), library: tableUrlFilters.text() },
};
const defaultSorting = [{ id: "when", desc: true }];
const hiddenColumns = ["category", "library", "message"];
const noun = { one: "notification", other: "notifications" };
const getRowId = (row: ActivityRow) => row.id;
const getRowLabel = (row: ActivityRow) => row.title;

/**
 * Activity: notifications in one bounded table that loads as it scrolls. Each row names
 * what it is about as an entity chip, decisions are actions (approve, dismiss), and opening
 * a row reads it and shows it in full beside the table (docs/entity-tables.md).
 */
export function ActivityPage() {
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const printing = usePrintMode();
  const wide = useMediaQuery("(min-width: 1280px)");
  const [preset, setPreset] = useQueryState("show", parseAsStringEnum<ActivityPreset>([...ACTIVITY_PRESETS]).withDefault("unread").withOptions({ history: "push" }));
  const libraries = useCompleteConnection(EntityLibraryListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.libraries);
  const libraryOptions = useMemo(() => libraries.rows.map((library) => ({ id: library.id, name: library.name })), [libraries.rows]);
  const columns = useMemo(() => columnsFor(libraryOptions), [libraryOptions]);
  const url = useTableUrlState({ namespace: "activity", columns, ...urlSchema, defaultSorting });
  const state = useMemo<DataTableState>(() => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: {}, ...url.state }), [url.state]);
  const onStateChange = (next: DataTableState, change: DataTableStateChange) => url.onStateChange(next, change);

  const activity = useActivity(state, preset);
  const refetch = activity.refetch;
  const onChanged = useCallback(() => refetch(), [refetch]);
  const actions = useNotificationActions(onChanged, isAdmin);
  const [update] = useMutation(EntityNotificationUpdateDocument);
  const [updateMany] = useMutation(EntityNotificationUpdateManyDocument);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = activity.list.rows.find((row) => row.id === selectedId) ?? null;
  const open = (row: ActivityRow) => {
    setSelectedId(row.id);
    if (!row.readAt) void update({ variables: { id: row.id, input: { readAt: new Date().toISOString() } } }).then(onChanged);
  };
  const markAllRead = async () => {
    await updateMany({ variables: { where: { readAt: { isNull: true } }, input: { readAt: new Date().toISOString() } } });
    onChanged();
  };

  return (
    <div data-print-flow className="page-gutter flex min-h-0 flex-1 flex-col gap-4 overflow-hidden py-4 md:py-6 [@media(max-height:35rem)]:gap-2 [@media(max-height:35rem)]:py-2">
      <h1 className="sr-only">Activity</h1>
      <div className="flex shrink-0 flex-wrap items-center gap-x-6 gap-y-3">
        <Segmented<ActivityPreset>
          aria-label="Show activity"
          value={preset}
          onValueChange={(value) => void setPreset(value === "unread" ? null : value)}
          options={ACTIVITY_PRESETS.map((value) => ({ value, label: activity.counts[value] !== undefined ? `${PRESET_LABEL[value]} ${activity.counts[value]}` : PRESET_LABEL[value] }))}
        />
        <div className="ms-auto flex items-center gap-2">
          <Button variant="secondary" isDisabled={!activity.counts.unread} onPress={() => void markAllRead()}><IconCheck /> Mark all read</Button>
        </div>
      </div>
      <div data-print-flow className={cn("grid min-h-0 flex-1 grid-rows-[minmax(0,1fr)] gap-6 overflow-hidden", selected && wide && "xl:grid-cols-[minmax(0,3fr)_minmax(22rem,2fr)]")}>
        <div className="flex min-h-0 min-w-0 flex-col">
          <DataTable<ActivityRow>
            id={`librarian.${user?.id ?? "session"}.activity`}
            label="Activity"
            queryKey={JSON.stringify([user?.id, activity.variables])}
            height="fill"
            columns={columns}
            defaultHiddenColumns={hiddenColumns}
            rows={activity.list.rows}
            getRowId={getRowId}
            getRowLabel={getRowLabel}
            loading={activity.list.loading}
            refetching={activity.list.fetching && !activity.list.loading}
            rowCount={activity.list.totalCount}
            state={state}
            onStateChange={onStateChange}
            infinite={{ revision: activity.list.revision, hasMore: !printing && activity.list.hasMore, onLoadMore: activity.list.loadMore, loadingMore: activity.list.loadingMore, error: activity.list.moreError?.message }}
            search={search}
            actions={actions}
            selectable
            live
            noun={noun}
            onRowOpen={open}
            rowOpen="click"
            error={activity.list.error && activity.list.rows.length === 0 ? "Couldn't load activity. Retry to reconnect." : undefined}
            onRetry={refetch}
            emptyMessage={preset === "unread" ? "You're all caught up." : preset === "action" ? "Nothing needs a decision." : "No activity yet."}
            toolbar={<Button size="sm" variant="ghost" isIconOnly aria-label="Refresh activity" isPending={activity.list.fetching} onPress={refetch}><IconRefresh /></Button>}
          />
        </div>
        {selected && wide ? <div data-print-flow className="scrollbar-thin min-h-0 overflow-y-auto"><ActivityDetail row={selected} actions={actions} onClose={() => setSelectedId(null)} /></div> : null}
      </div>
      <Dialog isOpen={Boolean(selected) && !wide} onOpenChange={(value) => !value && setSelectedId(null)} title="Notification" size="md">
        {selected ? <ActivityDetail row={selected} actions={actions} onClose={() => setSelectedId(null)} /> : null}
      </Dialog>
    </div>
  );
}
