import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell, NumericCell, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconArrowDown, IconArrowUp, IconEdit, IconPlus, IconTrash } from "glass-ux/icons";
import { IconPlugConnected, IconPower } from "@tabler/icons-react";
import { useState } from "react";

import { Button } from "@/components/ui";
import { useSession } from "@/lib/auth/useSession";
import { ENTITY_KINDS, glyph } from "@/lib/entities/kinds";
import { EntitySourceDeleteDocument, EntitySourceListDocument, EntitySourceUpdateDocument, TestSourceDocument, UpdateSourcePrioritiesDocument, type EntitySourceListQuery } from "@/graphql/generated/graphql";
import { parseTimestamp } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { SourceDialog } from "./SourceDialog";

import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsSection } from "./SettingsSection";
import { useSettingsConnection } from "./useSettingsConnection";

type Source = EntitySourceListQuery["sources"]["edges"][number]["node"];

const SourceGlyph = ENTITY_KINDS.source.icon;
const TestIcon = glyph(IconPlugConnected, "plug-connected");
const PowerIcon = glyph(IconPower, "power");
const searches = (source: Source) => [source.supportsMovieSearch && "Movies", source.supportsTvSearch && "TV", source.supportsMusicSearch && "Music", source.supportsBookSearch && "Books"].filter(Boolean).join(", ") || (source.supportsSearch ? "General" : "None");
/** Enabled and healthy, enabled with errors, or switched off. */
function sourceState(source: Source): { label: string; tone: "success" | "danger" | "neutral"; status: "online" | "error" | "offline" } {
  if (!source.enabled) return { label: "Disabled", tone: "neutral", status: "offline" };
  return source.lastError ? { label: `Failing ×${source.errorCount}`, tone: "danger", status: "error" } : { label: "Enabled", tone: "success", status: "online" };
}
const column = createDataTableColumns<Source>();
const columns = column.columns([
  column.accessor("priority", { header: "#", width: 4, priority: 2, align: "end", grow: false, cell: ({ row }) => <NumericCell value={row.original.priority} /> }),
  column.accessor("name", {
    header: "Source", width: 24, priority: 1, cardRole: "title",
    cell: ({ row }) => { const state = sourceState(row.original); return <IdentityCell icon={<SourceGlyph />} title={row.original.name} subtitle={[row.original.definitionId, row.original.sourceType.replace(/_/g, " "), row.original.siteUrl].filter(Boolean).join(" · ")} status={state.status} statusLabel={state.label} />; },
  }),
  column.display({ id: "capabilities", header: "Searches", width: 12, priority: 3, cell: ({ row }) => <TextCell muted>{searches(row.original)}</TextCell> }),
  column.display({ id: "status", header: "Status", width: 9, priority: 2, cardRole: "status", cell: ({ row }) => { const state = sourceState(row.original); return <Badge size="sm" marker="dot" tone={state.tone}>{state.label}</Badge>; } }),
  column.accessor((source) => source.lastSuccessAt ?? "", { id: "last", header: "Last success", width: 9, priority: 4, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.lastSuccessAt)} fallback="Never" /> }),
]);

/** Torrent indexers and feeds: add, test, order by priority, enable or remove. */
export function SourcesSettings() {
  const list = useSettingsConnection(EntitySourceListDocument, { orderBy: [{ priority: "ASC" }] }, (data) => data.sources);
  const { rows: sources, loading, refetch } = list;
  const [editing, setEditing] = useState<Source | null | "new">(null);
  const [testSource] = useMutation(TestSourceDocument);
  const [updateSource] = useMutation(EntitySourceUpdateDocument);
  const [deleteSource] = useMutation(EntitySourceDeleteDocument);
  const { user } = useSession();
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
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const actions = defineActions<Source>("source", [
    { id: "edit", label: "Edit", icon: IconEdit, group: "open", priority: 2, visible: (rows) => rows.length === 1, run: (rows) => setEditing(rows[0]!) },
    { id: "test", label: "Test connection", icon: TestIcon, group: "manage", priority: 1, visible: (rows) => rows.length === 1, run: (rows) => test(rows[0]!) },
    { id: "toggle", label: (rows) => (rows[0]!.enabled ? "Disable" : "Enable"), icon: PowerIcon, group: "manage", priority: 2, visible: (rows) => rows.length === 1, run: (rows) => toggle(rows[0]!) },
    { id: "up", label: "Move up", icon: IconArrowUp, group: "organise", visible: (rows) => rows.length === 1 && sources[0]?.id !== rows[0]!.id, run: (rows) => move(rows[0]!, -1) },
    { id: "down", label: "Move down", icon: IconArrowDown, group: "organise", visible: (rows) => rows.length === 1 && sources.at(-1)?.id !== rows[0]!.id, run: (rows) => move(rows[0]!, 1) },
    {
      id: "delete", label: "Remove", icon: IconTrash, group: "danger", visible: (rows) => rows.length === 1,
      confirm: (rows) => ({ title: `Remove ${rows[0]!.name}?`, body: "Stored credentials are deleted. Downloads already started keep running.", confirmLabel: "Remove", tone: "danger" }),
      run: async (rows) => { assertSuccess((await deleteSource({ variables: { id: rows[0]!.id } })).data?.deleteSource, "Could not remove"); void refetch(); },
      successMessage: () => "Source removed",
    },
  ]);

  return (
    <div className="flex flex-col gap-6">
      <SettingsSection
        title="Sources"
        description="Searched in priority order when looking for releases. Drag a row to change its priority."
        actions={<Button variant="primary" size="sm" onPress={() => setEditing("new")}><IconPlus /> Add source</Button>}
      >
        <SettingsLoadError error={list.error} retry={list.refetch} hasRows={sources.length > 0} />
        <DataTable<Source> height="auto"
          id={`librarian.${user?.id ?? "session"}.settings.sources`}
          label="Sources"
          columns={columns}
          rows={sources}
          getRowId={(source) => source.id}
          getRowLabel={(source) => source.name}
          loading={loading && sources.length === 0}
          actions={actions}
          onRowOpen={(source) => setEditing(source)}
          rowOpen="click"
          reorder={{
            onReorder: async (change) => {
              assertSuccess((await reorder({ variables: { input: { sourceIds: change.rows.map((source) => source.id) } } })).data?.updateSourcePriorities, "Could not reorder");
              await refetch();
            },
          }}
          noun={{ one: "source", other: "sources" }}
          emptyMessage="No sources yet. Add a torrent indexer to search for releases automatically."
          emptyAction={<Button variant="primary" onPress={() => setEditing("new")}><IconPlus /> Add source</Button>}
        />
      </SettingsSection>
      <SourceDialog source={editing === "new" ? null : editing} isOpen={editing !== null} onOpenChange={(open) => !open && setEditing(null)} onSaved={() => void refetch()} />
    </div>
  );
}
