import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { defineActions } from "glass-ux/actions";
import { createDataTableColumns, NumericCell, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconArrowDown, IconArrowUp, IconEdit, IconPlus, IconTrash } from "glass-ux/icons";
import { IconList, IconPlugConnected, IconPower, IconRss } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button } from "@/components/ui";
import { useSession } from "@/lib/auth/useSession";
import { glyph } from "@/lib/entities/kinds";
import {
  CreateRssFeedFromSourceDocument,
  EntityRssFeedDeleteDocument,
  EntityRssFeedListDocument,
  EntityRssFeedUpdateDocument,
  EntitySourceDeleteDocument,
  EntitySourceListDocument,
  EntitySourceUpdateDocument,
  PollRssFeedDocument,
  TestSourceDocument,
  UpdateSourcePrioritiesDocument,
  type EntityRssFeedListQuery,
  type EntitySourceListQuery,
} from "@/graphql/generated/graphql";
import { parseTimestamp } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { FeedItemsDialog } from "./FeedItemsDialog";
import { SourceDialog, type SourceTarget } from "./SourceDialog";
import { SourceIdentity } from "./SourceIdentity";
import { healthState, type HealthState } from "./sourceHealth";

import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { runSettingsSelection } from "./runSettingsSelection";

import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsPage, SettingsSection } from "./SettingsSection";
import { useSettingsConnection } from "./useSettingsConnection";

type Source = EntitySourceListQuery["sources"]["edges"][number]["node"];
type Feed = EntityRssFeedListQuery["rssFeeds"]["edges"][number]["node"];

/** One row of the Sources table: an indexer or an RSS feed, with its health worked out. */
export type SourceRow =
  | { rowKey: string; kind: "source"; name: string; enabled: boolean; health: HealthState; source: Source }
  | { rowKey: string; kind: "feed"; name: string; enabled: boolean; health: HealthState; feed: Feed };

const FeedGlyph = glyph(IconRss, "rss");
const TestIcon = glyph(IconPlugConnected, "plug-connected");
const PowerIcon = glyph(IconPower, "power");
const ItemsIcon = glyph(IconList, "list");

const searches = (source: Source) => [source.supportsMovieSearch && "Movies", source.supportsTvSearch && "TV", source.supportsMusicSearch && "Music", source.supportsBookSearch && "Books"].filter(Boolean).join(", ") || (source.supportsSearch ? "General" : "None");

/** Indexers by priority, then feeds by name, each with the state its health fields give. */
export function sourceRows(sources: readonly Source[], feeds: readonly Feed[], now = Date.now()): SourceRow[] {
  return [
    ...sources.map((source): SourceRow => ({
      rowKey: `source:${source.id}`, kind: "source", name: source.name, enabled: source.enabled, source,
      health: healthState({ enabled: source.enabled, healthStatus: source.healthStatus, healthKind: source.healthKind, lastError: source.lastError, lastSuccessAt: source.lastSuccessAt, lastCheckedAt: source.lastCheckedAt, failures: source.errorCount }, now),
    })),
    ...feeds.map((feed): SourceRow => ({
      rowKey: `feed:${feed.id}`, kind: "feed", name: feed.name, enabled: feed.enabled, feed,
      health: healthState({ enabled: feed.enabled, healthStatus: feed.healthStatus, healthKind: feed.healthKind, lastError: feed.lastError, lastSuccessAt: feed.lastSuccessfulAt, lastCheckedAt: feed.lastPolledAt, failures: feed.consecutiveFailures }, now),
    })),
  ];
}

const column = createDataTableColumns<SourceRow>();
const columns = column.columns([
  column.accessor((row) => (row.kind === "source" ? row.source.priority : 1000), { id: "priority", header: "#", width: 4, priority: 2, align: "end", grow: false, cell: ({ row }) => (row.original.kind === "source" ? <NumericCell value={row.original.source.priority} /> : <TextCell muted>—</TextCell>) }),
  column.accessor("name", {
    header: "Source", width: 42, priority: 1, cardRole: "title",
    cell: ({ row }) => {
      const item = row.original;
      const subtitle = item.kind === "source"
        ? [item.source.definitionId, item.source.siteUrl].filter(Boolean).join(" · ")
        : [item.feed.urlDisplay, `every ${item.feed.pollIntervalMinutes} min`].filter(Boolean).join(" · ");
      return <SourceIdentity kind={item.kind} name={item.name} health={item.health} enabled={item.enabled} facts={subtitle} />;
    },
  }),
  column.display({ id: "kind", header: "Kind", width: 11, priority: 3, cell: ({ row }) => <TextCell muted>{row.original.kind === "feed" ? "RSS feed" : searches(row.original.source)}</TextCell> }),
  column.accessor((row) => (row.kind === "source" ? row.source.lastSuccessAt : row.feed.lastSuccessfulAt) ?? "", { id: "last", header: "Last worked", width: 10, priority: 4, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.kind === "source" ? row.original.source.lastSuccessAt : row.original.feed.lastSuccessfulAt)} fallback="Never" /> }),
]);

/** Torrent indexers and RSS feeds: add, check, order by priority, enable or remove. */
export function SourcesSettings() {
  const sourceList = useSettingsConnection(EntitySourceListDocument, { orderBy: [{ priority: "ASC" }] }, (data) => data.sources);
  const feedList = useSettingsConnection(EntityRssFeedListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.rssFeeds);
  const { rows: sources, refetch: refetchSources } = sourceList;
  const { rows: feeds, refetch: refetchFeeds } = feedList;
  const rows = useMemo(() => sourceRows(sources, feeds), [sources, feeds]);
  const refetch = async () => { await Promise.all([refetchSources(), refetchFeeds()]); };
  const [editing, setEditing] = useState<SourceTarget | null | "new">(null);
  const [browsing, setBrowsing] = useState<Feed | null>(null);
  const [testSource] = useMutation(TestSourceDocument);
  const [updateSource] = useMutation(EntitySourceUpdateDocument);
  const [deleteSource] = useMutation(EntitySourceDeleteDocument);
  const [pollFeed] = useMutation(PollRssFeedDocument);
  const [updateFeed] = useMutation(EntityRssFeedUpdateDocument);
  const [deleteFeed] = useMutation(EntityRssFeedDeleteDocument);
  const [makeFeed] = useMutation(CreateRssFeedFromSourceDocument);
  const { user } = useSession();
  const [reorder] = useMutation(UpdateSourcePrioritiesDocument);

  const test = async (row: SourceRow) => {
    try {
      if (row.kind === "source") {
        const { data: result } = await testSource({ variables: { id: row.source.id } });
        if (result?.testSource.success) toast.success(`${row.name}: working, ${result.testSource.releasesFound ?? 0} releases in ${result.testSource.elapsedMs ?? 0} ms`);
        else toast.warning(`${row.name}: ${result?.testSource.error ?? "test failed"}`);
      } else {
        const result = (await pollFeed({ variables: { id: row.feed.id } })).data?.pollRssFeed;
        if (result?.success) toast.success(`${row.name}: read, ${result.newItems} new ${result.newItems === 1 ? "release" : "releases"}`);
        else toast.warning(`${row.name}: ${result?.error ?? "could not be read"}`);
      }
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const sourceIds = (list: readonly SourceRow[]) => list.flatMap((row) => (row.kind === "source" ? [row.source.id] : []));
  const move = async (row: SourceRow, direction: -1 | 1) => {
    const ids = sourceIds(rows);
    const index = ids.indexOf(row.kind === "source" ? row.source.id : "");
    const target = index + direction;
    if (index < 0 || target < 0 || target >= ids.length) return;
    [ids[index], ids[target]] = [ids[target]!, ids[index]!];
    try {
      assertSuccess((await reorder({ variables: { input: { sourceIds: ids } } })).data?.updateSourcePriorities, "Could not reorder");
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const setEnabled = async (row: SourceRow, enabled: boolean) => {
    if (row.kind === "source") assertSuccess((await updateSource({ variables: { id: row.source.id, input: { enabled } } })).data?.updateSource, "Could not update");
    else assertSuccess((await updateFeed({ variables: { id: row.feed.id, input: { enabled } } })).data?.updateRssFeed, "Could not update");
  };
  const toggle = async (row: SourceRow) => {
    try {
      await setEnabled(row, !row.enabled);
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };
  const remove = async (row: SourceRow) => {
    if (row.kind === "source") assertSuccess((await deleteSource({ variables: { id: row.source.id } })).data?.deleteSource, "Could not remove");
    else assertSuccess((await deleteFeed({ variables: { id: row.feed.id } })).data?.deleteRssFeed, "Could not remove");
  };
  const feedFromAccount = async (row: SourceRow) => {
    if (row.kind !== "source") return;
    try {
      const result = assertSuccess((await makeFeed({ variables: { sourceId: row.source.id } })).data?.createRssFeedFromSource, "Could not create the feed");
      toast.success(`Feed added from ${row.name}: ${result.urlDisplay ?? "its personal RSS link"}`);
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const only = (rows: readonly SourceRow[]) => (rows.length === 1 ? rows[0]! : null);
  const actions = defineActions<SourceRow>("source", [
    { id: "edit", bulk: true, label: "Edit", icon: IconEdit, group: "open", priority: 2, visible: (selected) => selected.length === 1, run: (selected) => setEditing(selected[0]!.kind === "source" ? { kind: "source", source: selected[0]!.source } : { kind: "feed", feed: (selected[0] as Extract<SourceRow, { kind: "feed" }>).feed }) },
    { id: "items", label: "What it has listed", icon: ItemsIcon, group: "open", priority: 3, visible: (selected) => only(selected)?.kind === "feed", run: (selected) => { const row = selected[0]; if (row?.kind === "feed") setBrowsing(row.feed); } },
    { id: "test", label: (selected) => (selected.length === 1 ? (selected[0]!.kind === "feed" ? "Poll now" : "Test now") : `Check ${selected.length} sources`), icon: (TestIcon), group: "manage", priority: 1, bulk: true,
      run: async (selected) => {
        if (selected.length === 1) { await test(selected[0]!); return; }
        await runSettingsSelection(selected, "Check", (row) => row.name, async (row) => {
          if (row.kind === "source") { const result = (await testSource({ variables: { id: row.source.id } })).data?.testSource; if (!result?.success) throw new Error(result?.error ?? "Connection test failed"); }
          else { const result = (await pollFeed({ variables: { id: row.feed.id } })).data?.pollRssFeed; if (!result?.success) throw new Error(result?.error ?? "The feed could not be read"); }
        }, refetch);
        toast.success(`Checked ${selected.length} sources`);
      },
    },
    { id: "account-feed", label: "Create RSS feed from this account", icon: FeedGlyph, group: "manage", priority: 4,
      visible: (selected) => { const row = only(selected); return row?.kind === "source" && row.source.definitionId === "iptorrents"; },
      run: (selected) => feedFromAccount(selected[0]!) },
    { id: "toggle", bulk: true, label: (selected) => (selected[0]!.enabled ? "Disable" : "Enable"), icon: PowerIcon, group: "manage", priority: 2, visible: (selected) => selected.length === 1, run: (selected) => toggle(selected[0]!) },
    ...([true, false] as const).map((enabled) => ({
      id: enabled ? "enable-selected" : "disable-selected", label: enabled ? "Enable sources" : "Disable sources", icon: PowerIcon, group: "manage" as const, bulk: true,
      visible: (selected: readonly SourceRow[]) => selected.length > 1 && selected.some((row) => row.enabled !== enabled),
      run: (selected: readonly SourceRow[]) => runSettingsSelection(selected.filter((row) => row.enabled !== enabled), enabled ? "Enable" : "Disable", (row) => row.name, (row) => setEnabled(row, enabled), refetch),
      successMessage: () => enabled ? "Sources enabled" : "Sources disabled",
    })),
    { id: "up", bulk: true, label: "Move up", icon: IconArrowUp, group: "organise", visible: (selected) => { const row = only(selected); return row?.kind === "source" && sourceIds(rows)[0] !== row.source.id; }, run: (selected) => move(selected[0]!, -1) },
    { id: "down", bulk: true, label: "Move down", icon: IconArrowDown, group: "organise", visible: (selected) => { const row = only(selected); return row?.kind === "source" && sourceIds(rows).at(-1) !== row.source.id; }, run: (selected) => move(selected[0]!, 1) },
    {
      id: "delete", label: (selected) => selected.length === 1 ? "Remove" : `Remove ${selected.length} sources`, icon: IconTrash, group: "danger", bulk: true,
      confirm: (selected) => ({ title: selected.length === 1 ? `Remove ${selected[0]!.name}?` : `Remove ${selected.length} sources?`, body: "Stored credentials and feed addresses are deleted. Downloads already started keep running.", confirmLabel: "Remove", tone: "danger" }),
      run: (selected) => runSettingsSelection(selected, "Remove", (row) => row.name, remove, refetch),
      successMessage: (selected) => selected.length === 1 ? "Source removed" : `${selected.length} sources removed`,
    },
  ]);

  const editingTarget = editing === "new" ? null : editing;
  return (
    <SettingsPage>
      <SettingsSection
        title="Sources"
        description="Indexers are searched in priority order when looking for releases; RSS feeds are read on a schedule and kept as a catalogue of what is available. Each is checked at least once an hour."
        actions={<Button variant="primary" size="sm" onPress={() => setEditing("new")}><IconPlus /> Add source</Button>}
      >
        <SettingsLoadError error={sourceList.error ?? feedList.error} retry={refetch} hasRows={rows.length > 0} />
        <LoadedDataTable<SourceRow> complete={!sourceList.loading && !feedList.loading && !sourceList.error && !feedList.error} height="auto" selectable queryKey={user?.id}
          id={`librarian.${user?.id ?? "session"}.settings.sources`}
          label="Sources"
          columns={columns}
          rows={rows}
          getRowId={(row) => row.rowKey}
          getRowLabel={(row) => row.name}
          loading={(sourceList.loading || feedList.loading) && rows.length === 0}
          actions={actions} renderBulkActions={(selected) => <TableBulkActions set={actions} rows={selected} noun={{ one: "source", other: "sources" }} />}
          onRowOpen={(row) => setEditing(row.kind === "source" ? { kind: "source", source: row.source } : { kind: "feed", feed: row.feed })}
          rowOpen="click"
          reorder={{
            onReorder: async (change) => {
              assertSuccess((await reorder({ variables: { input: { sourceIds: sourceIds(change.rows) } } })).data?.updateSourcePriorities, "Could not reorder");
              await refetch();
            },
          }}
          noun={{ one: "source", other: "sources" }}
          emptyMessage="No sources yet. Add a torrent indexer to search for releases, or an RSS feed to find new ones as they appear."
          emptyAction={<Button variant="primary" onPress={() => setEditing("new")}><IconPlus /> Add source</Button>}
        />
      </SettingsSection>
      <SourceDialog target={editingTarget} isOpen={editing !== null} onOpenChange={(open) => !open && setEditing(null)} onSaved={() => void refetch()} />
      <FeedItemsDialog feedId={browsing?.id ?? null} feedName={browsing?.name ?? ""} onOpenChange={(open) => !open && setBrowsing(null)} />
    </SettingsPage>
  );
}
