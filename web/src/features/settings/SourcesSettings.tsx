import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { IconArrowDown, IconArrowUp, IconPlugConnected, IconPlus, IconTrash } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, ConfirmDialog, DataTable, type DataTableColumn, type DataTableRowAction, EmptyState, Panel, StatusChip } from "@/components/ui";
import { EntitySourceDeleteDocument, EntitySourceListDocument, EntitySourceUpdateDocument, TestSourceDocument, UpdateSourcePrioritiesDocument, type EntitySourceListQuery } from "@/graphql/generated/graphql";
import { formatRelative } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { SourceDialog } from "./SourceDialog";

type Source = EntitySourceListQuery["sources"]["edges"][number]["node"];

/** Torrent indexers and feeds: add, test, order by priority, enable or remove. */
export function SourcesSettings() {
  const { data, previousData, loading, refetch } = useQuery(EntitySourceListDocument, { variables: { orderBy: [{ priority: "ASC" }], page: { limit: 100, offset: 0 } } });
  const sources = useMemo(() => (data ?? previousData)?.sources.edges.map((edge) => edge.node) ?? [], [data, previousData]);
  const [editing, setEditing] = useState<Source | null | "new">(null);
  const [removing, setRemoving] = useState<Source | null>(null);
  const [testSource] = useMutation(TestSourceDocument);
  const [updateSource] = useMutation(EntitySourceUpdateDocument);
  const [deleteSource, { loading: deleting }] = useMutation(EntitySourceDeleteDocument);
  const [reorder] = useMutation(UpdateSourcePrioritiesDocument);

  const test = async (source: Source) => {
    try {
      const { data: result } = await testSource({ variables: { id: source.id } });
      if (result?.testSource.success) toast.success(`${source.name}: ${result.testSource.releasesFound ?? 0} releases in ${result.testSource.elapsedMs ?? 0} ms`);
      else toast.warning(`${source.name}: ${result?.testSource.error ?? "test failed"}`);
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const move = async (source: Source, direction: -1 | 1) => {
    const ids = sources.map((item) => item.id);
    const index = ids.indexOf(source.id);
    const target = index + direction;
    if (target < 0 || target >= ids.length) return;
    [ids[index], ids[target]] = [ids[target]!, ids[index]!];
    try {
      assertSuccess((await reorder({ variables: { input: { sourceIds: ids } } })).data?.updateSourcePriorities, "Could not reorder");
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const toggle = async (source: Source) => {
    try {
      assertSuccess((await updateSource({ variables: { id: source.id, input: { enabled: !source.enabled } } })).data?.updateSource, "Could not update");
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const actions: Array<DataTableRowAction<Source>> = [
    { key: "test", label: "Test connection", icon: <IconPlugConnected size={16} />, onAction: test },
    { key: "toggle", label: (source) => (source.enabled ? "Disable" : "Enable"), onAction: toggle },
    { key: "up", label: "Move up", icon: <IconArrowUp size={16} />, onAction: (source) => move(source, -1) },
    { key: "down", label: "Move down", icon: <IconArrowDown size={16} />, onAction: (source) => move(source, 1) },
    { key: "delete", label: "Remove", icon: <IconTrash size={16} />, destructive: true, onAction: (source) => setRemoving(source) },
  ];

  const columns: Array<DataTableColumn<Source>> = [
    { id: "priority", header: "#", size: 64, align: "end", numeric: true, cell: (source) => source.priority },
    {
      id: "name",
      header: "Source",
      cell: (source) => (
        <span className="min-w-0">
          <span className="block truncate text-body-sm text-foreground">{source.name}</span>
          <span className="block truncate text-label-sm text-muted">{[source.definitionId, source.sourceType.replace(/_/g, " "), source.siteUrl].filter(Boolean).join(" · ")}</span>
        </span>
      ),
    },
    { id: "capabilities", header: "Searches", size: 200, hideBelow: "md", cell: (source) => <span className="text-muted">{[source.supportsMovieSearch && "Movies", source.supportsTvSearch && "TV", source.supportsMusicSearch && "Music", source.supportsBookSearch && "Books"].filter(Boolean).join(", ") || (source.supportsSearch ? "General" : "None")}</span> },
    { id: "status", header: "Status", size: 150, cell: (source) => (source.enabled ? (source.lastError ? <StatusChip status={{ label: `Error ×${source.errorCount}`, tone: "danger", dot: "bg-danger" }} /> : <StatusChip status={{ label: "Enabled", tone: "success", dot: "bg-success" }} />) : <StatusChip status={{ label: "Disabled", tone: "default", dot: "bg-muted" }} />) },
    { id: "last", header: "Last success", size: 140, hideBelow: "lg", cell: (source) => <span className="text-muted">{source.lastSuccessAt ? formatRelative(source.lastSuccessAt) : "Never"}</span> },
  ];

  return (
    <div className="flex flex-col gap-6">
      <Panel
        title="Sources"
        description="Searched in priority order when looking for releases."
        flush
        actions={
          <Button variant="primary" size="sm" onPress={() => setEditing("new")}>
            <IconPlus size={16} /> Add source
          </Button>
        }
      >
        <DataTable<Source>
          className="px-4 pb-4" frame={false}
          columns={columns}
          rows={sources}
          getRowId={(source) => source.id}
          isLoading={loading && sources.length === 0}
          rowActions={actions}
          onRowClick={(source) => setEditing(source)}
          noun="sources"
          emptyState={<EmptyState compact icon={IconPlugConnected} title="No sources yet" description="Add a torrent indexer to search for releases automatically." action={<Button variant="primary" onPress={() => setEditing("new")}><IconPlus size={16} /> Add source</Button>} />}
        />
      </Panel>
      <SourceDialog source={editing === "new" ? null : editing} isOpen={editing !== null} onOpenChange={(open) => !open && setEditing(null)} onSaved={() => void refetch()} />
      <ConfirmDialog
        isOpen={Boolean(removing)}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={`Remove ${removing?.name}?`}
        description="Stored credentials are deleted. Downloads already started keep running."
        confirmLabel="Remove"
        destructive
        isPending={deleting}
        onConfirm={async () => {
          if (!removing) return;
          try {
            assertSuccess((await deleteSource({ variables: { id: removing.id } })).data?.deleteSource, "Could not remove");
            toast.success("Source removed");
            setRemoving(null);
            void refetch();
          } catch (error) {
            toast.danger(errorMessage(error));
          }
        }}
      />
    </div>
  );
}
