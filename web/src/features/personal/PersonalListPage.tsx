import { copyText } from "glass-ux/copy-button";
import { useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { useMemo, useRef, useState } from "react";
import { defineActions } from "glass-ux/actions";
import { createDataTableColumns, DataTable, TextCell } from "glass-ux/data-table";
import { IconArrowDown, IconArrowUp, IconBookmark, IconCopy, IconExternalLink, IconPlay, IconShuffle, IconTrash } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { EmptyState, ErrorState } from "@/components/ui";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { useSession } from "@/lib/auth/useSession";
import { useSelectableTableState } from "@/hooks/useTableSelection";
import { useQueueActions } from "@/features/player/queueActions";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { EntityListEntryDeleteDocument, EntityListEntryUpdateDocument, EntitySavedViewGetDocument, PersonalEntriesDocument, PlaylistGetDocument, type PersonalEntryFragment } from "@/graphql/generated/graphql";
import { formatRuntime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery } from "@/lib/graphql/live/hooks";
import { entryKindLabel, EntryIdentity, entryLength } from "./EntryIdentity";
import { useAlbumEntity } from "@/features/media/useAlbumEntity";
import { useAudiobookEntity } from "@/features/media/useAudiobookEntity";
import { useMovieEntity } from "@/features/media/useMovieEntity";
import { useShowEntity } from "@/features/media/useShowEntity";
import { ENTRY_OWN_ACTIONS, liftActions } from "./entryActions";
import { PlaylistIdentity } from "./PlaylistIdentity";
import { moved, positionForMove } from "./positions";
import { playlistDuration } from "./playlistTotals";
import { SmartResults } from "./SmartResults";
import { entryPresentation, entryTarget, type PlaylistTarget } from "./types";
import { playlistTitle, useAddToPlaylist } from "./useAddToPlaylist";
import { useHomeShelves } from "./usePersonalLibrary";
import { usePlaylistEntity } from "./usePlaylistEntity";
import { usePlaylistPlayback } from "./usePlaylistPlayback";

type Entry = PersonalEntryFragment;
const getEntryId = (entry: Entry) => entry.id;
const entryTitle = (entry: Entry) => entryPresentation(entry)?.title ?? "Unavailable item";
const noun = { one: "item", other: "items" };
/** Removing the title from its library is not something a playlist row should sit beside Remove from playlist with. */
const LIFT_SKIP: ReadonlySet<string> = new Set([...ENTRY_OWN_ACTIONS, "remove"]);
const column = createDataTableColumns<Entry>();
const columns = column.columns([
  column.accessor(entryTitle, { id: "title", header: "Title", width: 30, priority: 1, cardRole: "title", cell: ({ row }) => <EntryIdentity entry={row.original} /> }),
  column.accessor(entryKindLabel, { id: "kind", header: "Type", width: 9, priority: 2, cardRole: "subtitle", cell: ({ row }) => <TextCell muted>{entryKindLabel(row.original)}</TextCell> }),
  column.accessor(entryLength, { id: "length", header: "Length", width: 7, priority: 2, align: "end", grow: false, cell: ({ row }) => <TextCell muted>{entryLength(row.original)}</TextCell> }),
]);

/** One playlist: its identity with Play and Shuffle, then its entries in order, to reorder, remove and act on together. */
export function PersonalListPage({ listId }: { listId: string }) {
  const navigate = useNavigate();
  const playback = usePlaylistPlayback();
  const { user } = useSession();
  const list = useLiveQuery(PlaylistGetDocument, { variables: { id: listId } });
  const candidate = list.data?.personalList;
  const current = candidate?.id === listId ? candidate : null;
  const saved = useLiveQuery(EntitySavedViewGetDocument, { variables: { id: current?.savedViewId ?? "" }, skip: !current?.savedViewId });
  const entries = useCompleteConnection(PersonalEntriesDocument, { listId }, (data) => data.listEntries, { skip: current?.kind === "SMART" });
  const table = useSelectableTableState({ scope: [user?.id, listId], rows: entries.rows, columns, getRowId: getEntryId, ready: entries.complete && !entries.fetching });
  const home = useHomeShelves();
  const { actions: listActions, dialogs } = usePlaylistEntity<NonNullable<typeof current>>({ open: false });
  // A movie, show, album or book entry carries that title's own actions (Find releases, Refresh
  // metadata, its settings...) beside the playlist's.
  const movieEntity = useMovieEntity<NonNullable<Entry["movie"]>>();
  const showEntity = useShowEntity<NonNullable<Entry["show"]>>();
  const albumEntity = useAlbumEntity<NonNullable<Entry["album"]>>();
  const bookEntity = useAudiobookEntity<NonNullable<Entry["audiobook"]>>();
  const [remove] = useMutation(EntityListEntryDeleteDocument);
  const [move] = useMutation(EntityListEntryUpdateDocument);
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const run = async (action: () => Promise<unknown>) => {
    if (busy.current) return;
    busy.current = true; setPending(true);
    try { await action(); } catch (error) { toast.danger(errorMessage(error)); } finally { busy.current = false; setPending(false); }
  };
  const playlist = useAddToPlaylist<Entry>({
    targetsOf: (rows) => rows.flatMap((row) => { const target = entryTarget(row); return target ? [target] : []; }),
    titleOf: (rows) => playlistTitle(rows, entryTitle(rows[0]!), noun),
    bulk: true,
    priority: 4,
  });

  // One write between two neighbours; the server settles a closed gap (see positions.ts).
  const reorderTo = async (entry: Entry, to: number) => {
    const from = entries.rows.findIndex((row) => row.id === entry.id);
    if (from < 0 || to === from || to < 0 || to >= entries.rows.length) return;
    const ordered = moved(entries.rows, from, to);
    const position = positionForMove(ordered, to);
    const { data } = await move({ variables: { id: entry.id, input: { position } } });
    assertSuccess(data?.updateListEntry, "Could not reorder the playlist");
    await entries.refetch();
  };
  const targetsFrom = (rows: readonly Entry[]): PlaylistTarget[] => rows.flatMap((row) => { const target = entryTarget(row); return target ? [target] : []; });
  // Where an entry sits among the targets (entries pointing at nothing have none).
  const targetIndex = (entry: Entry) => targetsFrom(entries.rows.slice(0, entries.rows.findIndex((row) => row.id === entry.id))).length;
  const inOrder = (selected: readonly Entry[]) => { const ids = new Set(selected.map((row) => row.id)); return entries.rows.filter((row) => ids.has(row.id)); };
  // Play next / Add to queue: the selected entries in playlist order; an album, show or book entry expands to its parts.
  const queueActions = useQueueActions<Entry>((rows) => [{ kind: "targets", targets: targetsFrom(inOrder(rows)) }]);
  const locked = pending ? "Saving the playlist" : !entries.complete ? "Load the whole playlist to reorder" : false;
  const actions = useMemo(() => defineActions<Entry>("list-entry", [
    {
      id: "play", label: (rows) => (rows.length === 1 ? "Play from here" : `Play ${rows.length} selected`), icon: IconPlay, group: "open", priority: 1, primary: true, bulk: true,
      // From here keeps the whole playlist in the queue, so Previous goes back through what came before.
      run: (rows) => rows.length === 1 ? playback.play(targetsFrom(entries.rows), targetIndex(rows[0]!)) : playback.play(targetsFrom(inOrder(rows))),
    },
    {
      id: "shuffle", label: (rows) => (rows.length === 1 ? "Shuffle from here" : `Shuffle ${rows.length} selected`), icon: IconShuffle, group: "open", bulk: true,
      run: (rows) => rows.length === 1 ? playback.shuffle(targetsFrom(entries.rows), targetIndex(rows[0]!)) : playback.shuffle(targetsFrom(inOrder(rows))),
    },
    ...queueActions,
    { id: "open", label: (rows) => `Open ${entryKindLabel(rows[0]!).toLowerCase()}`, icon: IconExternalLink, group: "open", priority: 2, visible: (rows) => rows.length === 1 && Boolean(entryPresentation(rows[0]!)?.href), run: (rows, context) => { const href = entryPresentation(rows[0]!)?.href; if (href) context.navigate(href); } },
    playlist.action,
    ...liftActions<Entry, NonNullable<Entry["movie"]>>(movieEntity.actions, (entry) => entry.movie, LIFT_SKIP),
    ...liftActions<Entry, NonNullable<Entry["show"]>>(showEntity.actions, (entry) => entry.show, LIFT_SKIP),
    ...liftActions<Entry, NonNullable<Entry["album"]>>(albumEntity.actions, (entry) => entry.album, LIFT_SKIP),
    ...liftActions<Entry, NonNullable<Entry["audiobook"]>>(bookEntity.actions, (entry) => entry.audiobook, LIFT_SKIP),
    { id: "copy-titles", label: "Copy item titles", icon: IconCopy, group: "organise", bulk: true, run: async (rows) => { await copyText(rows.map(entryTitle).join("\n")); }, successMessage: () => "Item titles copied" },
    { id: "up", label: "Move up", icon: IconArrowUp, group: "organise", visible: (rows) => rows.length === 1 && entries.rows[0]?.id !== rows[0]!.id, disabledReason: () => locked, run: (rows) => run(() => reorderTo(rows[0]!, entries.rows.findIndex((row) => row.id === rows[0]!.id) - 1)) },
    { id: "down", label: "Move down", icon: IconArrowDown, group: "organise", visible: (rows) => rows.length === 1 && entries.rows.at(-1)?.id !== rows[0]!.id, disabledReason: () => locked, run: (rows) => run(() => reorderTo(rows[0]!, entries.rows.findIndex((row) => row.id === rows[0]!.id) + 1)) },
    {
      id: "remove", label: (rows) => (rows.length === 1 ? "Remove from playlist" : `Remove ${rows.length} from playlist`), icon: IconTrash, group: "danger", bulk: true,
      disabledReason: () => (pending ? "Saving the playlist" : false),
      confirm: (rows) => rows.length > 1 ? { title: `Remove ${rows.length} items from this playlist?`, body: "Media and files remain in the library.", confirmLabel: "Remove items", tone: "danger" } : null,
      run: async (rows) => {
        setPending(true);
        const failures: string[] = [];
        try {
          for (const entry of rows) {
            try { assertSuccess((await remove({ variables: { id: entry.id } })).data?.deleteListEntry, "Could not remove the item"); }
            catch (error) { failures.push(`${entryTitle(entry)}: ${errorMessage(error)}`); }
          }
        } finally { try { await entries.refetch(); } finally { setPending(false); } }
        if (failures.length) throw new Error(`Removed ${rows.length - failures.length} of ${rows.length} items. ${failures.join("; ")}`);
      },
      successMessage: (rows) => (rows.length === 1 ? "Removed from the playlist" : `Removed ${rows.length} items`),
    },
  // eslint-disable-next-line react-hooks/exhaustive-deps
  ]), [entries.rows, entries.complete, locked, pending, playback, playlist.action, queueActions, remove, move, movieEntity.actions, showEntity.actions, albumEntity.actions, bookEntity.actions]);

  if (list.error && !current) return <ErrorState error={list.error} onRetry={() => void list.refetch()} className="m-6" />;
  if (!current) return <div className="page-gutter py-6">{list.loading ? <p role="status">Loading playlist…</p> : <EmptyState icon={IconBookmark} title="Playlist not found" />}</div>;
  const pinned = home.rows.some((shelf) => shelf.listId === listId && shelf.visible);
  const totals = entries.complete ? playlistDuration(entries.rows) : null;
  const duration = totals?.seconds ? `${totals.partial ? "At least " : ""}${formatRuntime(totals.seconds)}` : undefined;
  return <div data-print-flow className="page-gutter flex min-h-0 min-w-0 flex-1 flex-col gap-5 overflow-hidden py-6">
    <PlaylistIdentity list={current} tier="header" heading="h1" actions={listActions} pinned={pinned} duration={duration} className="shrink-0" />
    {home.error ? <ErrorState error={home.error} onRetry={() => void home.refetch()} /> : null}
    {current.kind === "SMART" ? saved.data?.savedView ? <SmartResults view={saved.data.savedView} /> : saved.error ? <ErrorState error={saved.error} onRetry={() => void saved.refetch()} /> : <p role="status">Loading rules…</p> : <>
      {entries.error ? <ErrorState error={entries.error} onRetry={() => void entries.refetch()} /> : null}
      <DataTable<Entry> id={`personal:${user?.id ?? "session"}:${listId}`} queryKey={[user?.id, listId]} label={current.name} height="fill" columns={columns} rows={table.rows} getRowId={getEntryId} state={table.state} onStateChange={table.onStateChange} getRowLabel={entryTitle}
        loading={entries.loading} refetching={entries.fetching && !entries.loading}
        actions={actions} selectable renderBulkActions={(rows) => <TableBulkActions set={actions} rows={rows} noun={noun} />}
        onRowOpen={(entry) => { const href = entryPresentation(entry)?.href; if (href) void navigate({ href }); }} rowOpen="click"
        infinite={{ hasMore: false, onLoadMore: () => undefined }}
        reorder={{ disabled: locked, onReorder: async (change) => {
          // A rejection puts the row back and the table says why.
          if (busy.current) throw new Error("Saving the playlist");
          busy.current = true; setPending(true);
          try { await reorderTo(change.row, change.to); } finally { busy.current = false; setPending(false); }
        } }}
        noun={noun} emptyMessage="Nothing in this playlist yet. Choose Add to playlist on a movie, show, episode, album, track, audiobook or chapter." />
    </>}
    {playlist.dialog}
    {dialogs}
    {movieEntity.dialogs}
    {showEntity.dialogs}
    {albumEntity.dialogs}
    {bookEntity.dialogs}
  </div>;
}
