import { copyText } from "glass-ux/copy-button";
import { useNavigate } from "@tanstack/react-router";
import { useMemo, useRef } from "react";
import { defineActions } from "glass-ux/actions";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { useSelectableTableState } from "@/hooks/useTableSelection";
import { useSession } from "@/lib/auth/useSession";
import { useQueueActions } from "@/features/player/queueActions";
import { usePlayer } from "@/features/player/usePlayer";
import type { PlayItem } from "@/features/player/store";
import { createDataTableColumns, DataTable, IdentityCell, TextCell } from "glass-ux/data-table";
import { EmptyState, ErrorState, MediaRow, PosterCard, PosterGallery } from "@/components/ui";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { IconBookmark, IconCopy, IconExternalLink, IconPlay } from "glass-ux/icons";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { useInfiniteConnection } from "@/features/libraries/browser/useInfiniteConnection";
import {
  EntityLibraryListDocument, EntityMovieListDocument, EntityShowListDocument, EntityAlbumListDocument, EntityAudiobookListDocument, EntityTrackListDocument,
  type OrderDirection,
  type SavedViewFieldsFragment, type MovieWhereInput, type ShowWhereInput, type AlbumWhereInput, type AudiobookWhereInput, type TrackWhereInput,
} from "@/graphql/generated/graphql";

import { moviePoster, showPoster, albumCover, audiobookCover } from "@/lib/artwork";

type Row = { image?: string; square?: boolean; id: string; title: string; detail: string; href: string; playItem?: PlayItem };
const getRowId = (row: Row) => row.id;
const TrackGlyph = ENTITY_KINDS.track.icon;
const column = createDataTableColumns<Row>();
const trackColumns = column.columns([
  column.accessor("title", { header: "Track", sortable: false, filter: { none: "Saved rules determine the complete result filter." }, width: 26, priority: 1, cardRole: "title", cell: ({ row }) => <IdentityCell icon={<TrackGlyph />} title={row.original.title} subtitle={row.original.detail} /> }),
  column.accessor("detail", { header: "Artist", sortable: false, filter: { none: "Saved rules determine the complete result filter." }, width: 14, priority: 2, cell: ({ row }) => <TextCell muted>{row.original.detail}</TextCell> }),
]);

/** Compile the small saved-rule vocabulary into generated, caller-authorized queries. */
export function smartRule(view: SavedViewFieldsFragment, ownedLibraryIds: string[]) {
  if (view.version !== 1 || !ownedLibraryIds.length || (view.libraryId && !ownedLibraryIds.includes(view.libraryId))) throw new Error("Saved view has no authorized library scope");
  const common = { libraryId: view.libraryId ? { eq: view.libraryId } : { inList: ownedLibraryIds } };
  const owner = { userId: { eq: view.userId } };
  const text = view.titleContains ? { contains: view.titleContains } : undefined;
  const available = view.availability === "ANY" ? undefined : view.availability === "AVAILABLE";
  const direction: OrderDirection = view.descending ? "DESC" : "ASC";
  const dateSort = view.sort === "CREATED_AT" ? "createdAt" : view.sort === "UPDATED_AT" ? "updatedAt" : null;
  switch (view.contentType) {
    case "MOVIE": return { kind: view.contentType, where: { ...common, ...owner, title: text, hasFile: available === undefined ? undefined : { eq: available } } satisfies MovieWhereInput, orderBy: [{ [dateSort ?? "title"]: direction }] };
    case "SHOW": return { kind: view.contentType, where: { ...common, ...owner, name: text } satisfies ShowWhereInput, orderBy: [{ [dateSort ?? "name"]: direction }] };
    case "ALBUM": return { kind: view.contentType, where: { ...common, ...owner, name: text, hasFiles: available === undefined ? undefined : { eq: available } } satisfies AlbumWhereInput, orderBy: [{ [dateSort ?? "name"]: direction }] };
    case "AUDIOBOOK": return { kind: view.contentType, where: { ...common, ...owner, title: text, hasFiles: available === undefined ? undefined : { eq: available } } satisfies AudiobookWhereInput, orderBy: [{ [dateSort ?? "title"]: direction }] };
    case "TRACK": return { kind: view.contentType, where: { ...common, title: text, mediaFileId: available === undefined ? undefined : { isNull: !available } } satisfies TrackWhereInput, orderBy: [{ [dateSort ?? "title"]: direction }] };
  }
}

function Results({ rows, list, preview = false }: { preview?: boolean; rows: Row[]; list: { loading: boolean; fetching: boolean; error?: Error; refetch: () => Promise<unknown>; hasMore: boolean; loadMore: () => Promise<unknown>; queryKey: string; loadRevision: number; totalCount: number | undefined } }) {
  const navigate = useNavigate();
  const player = usePlayer();
  const { user } = useSession();
  const table = useSelectableTableState({ scope: [user?.id, list.queryKey], rows, getRowId, ready: !list.loading && !list.fetching });
  const playable = (selected: readonly Row[]) => selected.some((row) => !row.playItem) ? "Some selected tracks have no file. Deselect them to play or queue the selection." : false;
  const items = (selected: readonly Row[]) => {
    if (playable(selected)) throw new Error("Select playable tracks");
    const ids = new Set(selected.map((row) => row.id));
    return rows.filter((row) => ids.has(row.id)).flatMap((row) => row.playItem ? [row.playItem] : []);
  };
  // Play next / Add to queue: the selected tracks in the displayed order.
  const queueActions = useQueueActions<Row>((selected) => [items(selected)], { disabledReason: playable });
  const actions = useMemo(() => {
    return defineActions<Row>("smart-track", [
      { id: "open", label: "Open album", icon: IconExternalLink, group: "open", bulk: true, visible: (selected) => selected.length === 1, run: ([row], context) => { if (row) context.navigate(row.href); } },
      { id: "play", label: (selected) => selected.length === 1 ? "Play" : "Play selected tracks", icon: IconPlay, group: "open", bulk: true, disabledReason: playable, run: (selected) => { const queue = items(selected); if (!queue.length) throw new Error("No playable tracks selected"); player.playAudio(queue); } },
      ...queueActions,
      { id: "copy-titles", label: "Copy track titles", icon: IconCopy, group: "organise", bulk: true, run: async (selected) => { await copyText(selected.map((row) => row.title).join("\n")); }, successMessage: () => "Track titles copied" },
    ]);
  }, [rows, player, queueActions]);
  if (preview) return <div className="flex min-w-0 flex-col gap-3">{list.error ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : null}{list.loading && rows.length === 0 ? <p role="status">Loading collection…</p> : null}<MediaRow ariaLabel="Smart collection">{rows.slice(0, 20).map((row) => <PosterCard key={row.id} title={row.title} meta={row.detail} image={row.image} aspect={row.square ? "square" : "poster"} width="row" onPress={() => void navigate({ href: row.href })} />)}</MediaRow></div>;
  const infinite = { hasMore: list.hasMore, loadMore: list.loadMore };
  // Titles are shown as posters, like the library browsers; tracks have no artwork of their own.
  if (rows.some((row) => row.image)) {
    return <div data-print-flow className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 overflow-hidden">
      {list.error ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : null}
      <PosterGallery<Row> label="Smart collection" noun="items" rows={rows} getRowId={(row) => row.id} loading={list.loading} refetching={list.fetching} queryKey={list.queryKey} totalCount={list.totalCount} infinite={infinite}
        emptyState={<EmptyState icon={IconBookmark} title="Nothing matches these rules yet" />}
        renderCard={(row) => <PosterCard title={row.title} meta={row.detail} image={row.image} aspect={row.square ? "square" : "poster"} onPress={() => void navigate({ href: row.href })} />} />
    </div>;
  }
  return <div data-print-flow className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 overflow-hidden">
    {list.error ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : null}
    <DataTable<Row> id={`smart:${user?.id ?? "session"}:${list.queryKey}`} label="Smart collection" height="fill" columns={trackColumns} rows={rows} getRowId={getRowId} state={table.state} onStateChange={table.onStateChange} getRowLabel={(row) => row.title} noun={{ one: "track", other: "tracks" }} loading={list.loading} refetching={list.fetching && !list.loading} queryKey={[user?.id, list.queryKey]}
      actions={actions} selectable renderBulkActions={(selected) => <TableBulkActions set={actions} rows={selected} noun={{ one: "track", other: "tracks" }} />}
      rowCount={list.totalCount} onRowOpen={(row) => void navigate({ href: row.href })} rowOpen="click" emptyMessage="Nothing matches these rules yet"
      infinite={{ revision: list.loadRevision, hasMore: list.hasMore, onLoadMore: list.loadMore }} />
  </div>;
}

function MovieResults({ view, preview, libraryIds }: { view: SavedViewFieldsFragment; preview?: boolean; libraryIds: string[] }) {
  const rule = smartRule(view, libraryIds)!;
  const list = useInfiniteConnection(EntityMovieListDocument, { where: rule.where as MovieWhereInput, orderBy: rule.orderBy }, (data) => data.movies);
  return <Results preview={preview} list={list} rows={list.rows.map((row) => ({ id: row.id, image: moviePoster(row.id), title: row.title, detail: row.year ? String(row.year) : "Movie", href: `/movies/${row.id}` }))} />;
}
function ShowResults({ view, preview, libraryIds }: { view: SavedViewFieldsFragment; preview?: boolean; libraryIds: string[] }) {
  const rule = smartRule(view, libraryIds)!;
  const list = useInfiniteConnection(EntityShowListDocument, { where: rule.where as ShowWhereInput, orderBy: rule.orderBy }, (data) => data.shows);
  return <Results preview={preview} list={list} rows={list.rows.map((row) => ({ id: row.id, image: showPoster(row), title: row.name, detail: row.network ?? "Show", href: `/shows/${row.id}` }))} />;
}
function AlbumResults({ view, preview, libraryIds }: { view: SavedViewFieldsFragment; preview?: boolean; libraryIds: string[] }) {
  const rule = smartRule(view, libraryIds)!;
  const list = useInfiniteConnection(EntityAlbumListDocument, { where: rule.where as AlbumWhereInput, orderBy: rule.orderBy }, (data) => data.albums);
  return <Results preview={preview} list={list} rows={list.rows.map((row) => ({ id: row.id, image: albumCover(row), square: true, title: row.name, detail: row.year ? String(row.year) : "Album", href: `/albums/${row.id}` }))} />;
}
function AudiobookResults({ view, preview, libraryIds }: { view: SavedViewFieldsFragment; preview?: boolean; libraryIds: string[] }) {
  const rule = smartRule(view, libraryIds)!;
  const list = useInfiniteConnection(EntityAudiobookListDocument, { where: rule.where as AudiobookWhereInput, orderBy: rule.orderBy }, (data) => data.audiobooks);
  return <Results preview={preview} list={list} rows={list.rows.map((row) => ({ id: row.id, image: audiobookCover(row), title: row.title, detail: row.authorName ?? "Audiobook", href: `/audiobooks/${row.id}` }))} />;
}
function TrackResults({ view, preview, libraryIds }: { view: SavedViewFieldsFragment; preview?: boolean; libraryIds: string[] }) {
  const rule = smartRule(view, libraryIds)!;
  const list = useInfiniteConnection(EntityTrackListDocument, { where: rule.where as TrackWhereInput, orderBy: rule.orderBy }, (data) => data.tracks);
  return <Results preview={preview} list={list} rows={list.rows.map((row) => ({ id: row.id, title: row.title, detail: row.artistName ?? "Track", href: `/albums/${row.albumId}`, playItem: row.mediaFileId ? { mediaFileId: row.mediaFileId, title: row.title, subtitle: row.artistName ?? undefined, href: `/albums/${row.albumId}`, entity: { kind: "track" as const, id: row.id }, duration: row.durationSecs ?? undefined } : undefined }))} />;
}
export function SmartResults({ view, preview }: { view: SavedViewFieldsFragment; preview?: boolean }) {
  const libraries = useCompleteConnection(EntityLibraryListDocument, { where: { userId: { eq: view.userId } } }, (data) => data.libraries);
  const ready = useRef({ key: libraries.queryKey, complete: false });
  if (ready.current.key !== libraries.queryKey) ready.current = { key: libraries.queryKey, complete: false };
  if (libraries.complete) ready.current.complete = true;
  if (!ready.current.complete) return libraries.error ? <ErrorState error={libraries.error} onRetry={() => void libraries.refetch()} /> : <p role="status">Loading collection…</p>;
  const ids = libraries.rows.map((row) => row.id);
  if (!ids.length || (view.libraryId && !ids.includes(view.libraryId))) return <EmptyState icon={IconBookmark} title="No libraries available" />;
  const refreshError = libraries.error ? <ErrorState error={libraries.error} onRetry={() => void libraries.refetch()} /> : null;
  switch (view.contentType) {
    case "MOVIE": return <>{refreshError}<MovieResults view={view} preview={preview} libraryIds={ids} /></>;
    case "SHOW": return <>{refreshError}<ShowResults view={view} preview={preview} libraryIds={ids} /></>;
    case "ALBUM": return <>{refreshError}<AlbumResults view={view} preview={preview} libraryIds={ids} /></>;
    case "AUDIOBOOK": return <>{refreshError}<AudiobookResults view={view} preview={preview} libraryIds={ids} /></>;
    case "TRACK": return <>{refreshError}<TrackResults view={view} preview={preview} libraryIds={ids} /></>;
  }
}
