import { useCallback, useMemo, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconPlus } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { errorMessage } from "@/lib/graphql/errors";
import { Button, ErrorState, PosterCard, PosterInTable, ViewToggle, type ViewMode } from "@/components/ui";
import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { TableToolbar } from "@/components/ui/TableToolbar";
import { useSession } from "@/lib/auth/useSession";
import { parseTimestamp } from "@/lib/format";
import { sizedArtwork } from "@/lib/api/urls";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { usePref } from "@/lib/prefs";
import { ListEditor } from "./ListEditor";
import { PlaylistIdentity, playlistKindLabel } from "./PlaylistIdentity";
import { playlistCovers } from "./playlistArt";
import { usePlaylistEntity } from "./usePlaylistEntity";
import { usePersonalLists } from "./usePersonalLibrary";
import { playlistCount, usePlaylists, type Playlist } from "./usePlaylists";

const column = createDataTableColumns<Playlist>();
const columns = column.columns([
  column.accessor("name", { header: "Playlist", width: 30, priority: 1, cardRole: "title", sortable: true, cell: ({ row }) => <PlaylistIdentity list={row.original} /> }),
  column.accessor((list) => (list.kind === "SMART" ? 0 : playlistCount(list)), { id: "items", header: "Items", width: 7, priority: 2, align: "end", grow: false, sortable: true, cardRole: "subtitle", cell: ({ row }) => <TextCell muted>{row.original.kind === "SMART" ? "Rules" : playlistCount(row.original)}</TextCell> }),
  column.accessor((list) => playlistKindLabel(list.kind), { id: "kind", header: "Kind", width: 12, priority: 3, sortable: true, cell: ({ row }) => <TextCell muted>{playlistKindLabel(row.original.kind)}</TextCell> }),
  column.accessor("updatedAt", { header: "Updated", width: 9, priority: 2, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.updatedAt)} /> }),
]);
const noun = { one: "playlist", other: "playlists" };
const PlaylistGlyph = ENTITY_KINDS.playlist.icon;

/** Someone's playlists, smart collections and Watch Later, as rows or cards, with making a new one in the table's toolbar. */
export function PersonalListsPage() {
  const lists = usePlaylists();
  const personal = usePersonalLists();
  const { user } = useSession();
  const navigate = useNavigate();
  const [editing, setEditing] = useState(false);
  const [creating, setCreating] = useState(false);
  const [view, setView] = usePref<ViewMode>("lists.view", "card");
  const open = useCallback((row: Playlist) => void navigate({ to: "/lists/$listId", params: { listId: row.id } }), [navigate]);
  const { actions, dialogs } = usePlaylistEntity<Playlist>();
  const renderCard = useCallback((row: Playlist) => {
    const covers = playlistCovers(row.entries.edges.map((edge) => edge.node));
    return (
      <PosterCard title={row.name} meta={[playlistKindLabel(row.kind), row.kind === "SMART" ? "" : `${playlistCount(row)} items`].filter(Boolean).join(" · ")} image={sizedArtwork(covers[0], 320)} mosaic={covers} aspect="square" fallbackIcon={<PlaylistGlyph />} to="/lists/$listId" params={{ listId: row.id }} />
    );
  }, []);
  const hasWatchLater = useMemo(() => lists.rows.some((list) => list.kind === "WATCH_LATER"), [lists.rows]);
  return <div className="page-gutter flex min-h-0 min-w-0 flex-1 flex-col gap-4 overflow-clip py-4 md:py-6">
    <h1 className="sr-only">Playlists</h1>
    {lists.error ? <ErrorState error={lists.error} onRetry={() => void lists.refetch()} /> : null}
    {/* A poster gallery like a library's: each card is one stop, its ring padded clear of the clip. */}
    <div className="media-browser flex min-h-0 min-w-0 flex-1 flex-col" data-view={view}>
      <PosterInTable.Provider value={true}>
      <LoadedDataTable<Playlist>
        id={`librarian.${user?.id ?? "session"}.lists`}
        label="Playlists"
        height="fill"
        view={view === "card" ? "cards" : "auto"}
        cardMinWidth={12}
        renderCard={renderCard}
        columns={columns}
        rows={lists.rows}
        complete={lists.complete}
        getRowId={(row) => row.id}
        getRowLabel={(row) => row.name}
        loading={lists.loading}
        noun={noun}
        selectable
        actions={actions}
        onRowOpen={open}
        rowOpen="click"
        search={{ fields: ["name"], placeholder: "Filter playlists" }}
        emptyMessage="Make a playlist of songs, shows, films and books, save titles for later, or collect titles with saved filters."
        toolbar={(
          <TableToolbar>
            {!hasWatchLater ? <Button size="sm" isPending={creating} isDisabled={!lists.complete} onPress={async () => { setCreating(true); try { await personal.createList("Watch Later", "WATCH_LATER"); } catch (error) { toast.danger(errorMessage(error)); } finally { setCreating(false); } }}>Create Watch Later</Button> : null}
            <ViewToggle value={view} onChange={setView} className="shrink-0" />
            <Button size="sm" variant="primary" onPress={() => setEditing(true)}><IconPlus size="md" /> New playlist</Button>
          </TableToolbar>
        )}
      />
      </PosterInTable.Provider>
    </div>
    {editing ? <ListEditor open={editing} onOpenChange={setEditing} /> : null}
    {dialogs}
  </div>;
}
