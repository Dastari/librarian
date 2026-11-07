import { useCallback, useMemo, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { defineActions } from "glass-ux/actions";
import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconExternalLink, IconPlus } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { errorMessage } from "@/lib/graphql/errors";
import { Button, ErrorState, ViewToggle, type ViewMode } from "@/components/ui";
import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { TableToolbar } from "@/components/ui/TableToolbar";
import { useSession } from "@/lib/auth/useSession";
import { parseTimestamp } from "@/lib/format";
import { usePref } from "@/lib/prefs";
import { ListEditor } from "./ListEditor";
import { usePersonalLists } from "./usePersonalLibrary";

type ListRow = ReturnType<typeof usePersonalLists>["rows"][number];

const column = createDataTableColumns<ListRow>();
const columns = column.columns([
  column.accessor("name", { header: "List", width: 28, priority: 1, cardRole: "title", sortable: true, cell: ({ row }) => <TextCell>{row.original.name}</TextCell> }),
  column.display({ id: "kind", header: "Kind", width: 16, priority: 1, cardRole: "subtitle", cell: ({ row }) => <TextCell muted>{row.original.kind === "SMART" ? "Smart collection" : row.original.kind === "WATCH_LATER" ? "Watch Later" : "Manual list / playlist"}</TextCell> }),
  column.accessor("updatedAt", { header: "Updated", width: 9, priority: 2, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.updatedAt)} /> }),
]);
const noun = { one: "list", other: "lists" };

/** Someone's lists, as cards or rows, with making a new one in the table's own toolbar. */
export function PersonalListsPage() {
  const lists = usePersonalLists();
  const { user } = useSession();
  const navigate = useNavigate();
  const [editing, setEditing] = useState(false);
  const [creating, setCreating] = useState(false);
  const [view, setView] = usePref<ViewMode>("lists.view", "card");
  const open = useCallback((row: ListRow) => void navigate({ to: "/lists/$listId", params: { listId: row.id } }), [navigate]);
  const actions = useMemo(() => defineActions<ListRow>("personal-list", [
    { id: "open", label: "Open list", icon: IconExternalLink, group: "open", priority: 1, visible: (rows) => rows.length === 1, run: (rows) => open(rows[0]!) },
  ]), [open]);
  return <div className="page-gutter flex min-h-0 min-w-0 flex-1 flex-col gap-4 overflow-clip py-4 md:py-6">
    <h1 className="sr-only">My lists</h1>
    {lists.error ? <ErrorState error={lists.error} onRetry={() => void lists.refetch()} /> : null}
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      <LoadedDataTable<ListRow>
        id={`librarian.${user?.id ?? "session"}.lists`}
        label="My lists"
        height="fill"
        view={view === "card" ? "cards" : "auto"}
        cardMinWidth={18}
        columns={columns}
        rows={lists.rows}
        complete={lists.complete}
        getRowId={(row) => row.id}
        getRowLabel={(row) => row.name}
        loading={lists.loading}
        noun={noun}
        selectable={false}
        actions={actions}
        onRowOpen={open}
        rowOpen="click"
        search={{ fields: ["name"], placeholder: "Filter lists" }}
        emptyMessage="Save titles for later, make playlists, or collect titles using saved filters."
        toolbar={(
          <TableToolbar>
            {!lists.rows.some((list) => list.kind === "WATCH_LATER") ? <Button size="sm" isPending={creating} isDisabled={!lists.complete} onPress={async () => { setCreating(true); try { await lists.createList("Watch Later", "WATCH_LATER"); } catch (error) { toast.danger(errorMessage(error)); } finally { setCreating(false); } }}>Create Watch Later</Button> : null}
            <ViewToggle value={view} onChange={setView} className="shrink-0" />
            <Button size="sm" variant="primary" onPress={() => setEditing(true)}><IconPlus size="md" /> New list</Button>
          </TableToolbar>
        )}
      />
    </div>
    {editing ? <ListEditor open={editing} onOpenChange={setEditing} /> : null}
  </div>;
}
