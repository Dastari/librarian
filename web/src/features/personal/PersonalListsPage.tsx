import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { IconBookmark, IconPlus } from "glass-ux/icons";
import { Surface } from "glass-ux/surface";
import { toast } from "glass-ux/toast";
import { errorMessage } from "@/lib/graphql/errors";
import { Button, EmptyState, ErrorState, PageHeader } from "@/components/ui";
import { ListEditor } from "./ListEditor";
import { usePersonalLists } from "./usePersonalLibrary";

export function PersonalListsPage() {
  const lists = usePersonalLists();
  const [editing, setEditing] = useState(false);
  const [creating, setCreating] = useState(false);
  return <div className="page-gutter flex min-w-0 flex-col gap-6 py-6">
    <PageHeader title="My lists" actions={<Button variant="primary" onPress={() => setEditing(true)}><IconPlus />New list</Button>} />
    {lists.error ? <ErrorState error={lists.error} onRetry={() => void lists.refetch()} /> : null}
    {!lists.loading && lists.rows.length === 0 ? <EmptyState icon={IconBookmark} title="Your personal library" description="Save titles for later, make playlists, or collect titles using saved filters." /> : null}
    {!lists.rows.some((list) => list.kind === "WATCH_LATER") ? <Button isPending={creating} isDisabled={!lists.complete} onPress={async () => { setCreating(true); try { await lists.createList("Watch Later", "WATCH_LATER"); } catch (error) { toast.danger(errorMessage(error)); } finally { setCreating(false); } }}>Create Watch Later</Button> : null}
    <div className="grid min-w-0 gap-4 sm:grid-cols-2 xl:grid-cols-3">
      {lists.rows.map((list) => <Surface key={list.id} material="card" radius="tile" padding="tile">
        <Link to="/lists/$listId" params={{ listId: list.id }} className="nav-focus block min-w-0 rounded">
          <p className="break-words text-title-md text-foreground">{list.name}</p>
          <p className="mt-1 text-body-sm text-muted">{list.kind === "SMART" ? "Smart collection" : list.kind === "WATCH_LATER" ? "Watch Later" : "Manual list / playlist"}</p>
        </Link>
      </Surface>)}
    </div>
    {editing ? <ListEditor open={editing} onOpenChange={setEditing} /> : null}
  </div>;
}
