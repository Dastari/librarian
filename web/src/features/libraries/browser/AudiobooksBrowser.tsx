import { createDataTableColumns, TextCell } from "glass-ux/data-table";
import { useNavigate } from "@tanstack/react-router";

import { IconPlus } from "glass-ux/icons";

import { useCallback, useMemo, useState } from "react";

import { Button, PosterCard } from "@/components/ui";
import { useAudiobookEntity } from "@/features/media/useAudiobookEntity";
import { AudiobookIdentity } from "@/features/media/AudiobookIdentity";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryAudiobooksDocument, type AudiobookOrderByInput, type AudiobookWhereInput, type LibraryAudiobooksQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { audiobookCover } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { ProgressBadge } from "./ShowsBrowser";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { formatBytes, formatRuntime } from "@/lib/format";
import { MediaBrowserTable } from "./MediaBrowserTable";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type BookRow = LibraryAudiobooksQuery["audiobooks"]["edges"][number]["node"];
const getRowId = (row: BookRow) => row.id;

const column = createDataTableColumns<BookRow>();
const columns = column.columns([
  column.accessor("title", { header: "Audiobook", width: 32, priority: 1, cardRole: "title", sortable: true, cell: ({ row }) => <AudiobookIdentity book={row.original} /> }),
  column.display({ id: "chapters", header: "Chapters", width: 8, priority: 2, align: "end", grow: false, cell: ({ row }) => <TextCell muted>{`${row.original.downloaded.pageInfo.totalCount ?? 0} of ${row.original.chapterCount ?? 0}`}</TextCell> }),
  column.accessor("totalDurationSecs", { header: "Length", width: 7, priority: 2, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <TextCell muted>{row.original.totalDurationSecs ? formatRuntime(row.original.totalDurationSecs) : ""}</TextCell> }),
  column.accessor("sizeBytes", { header: "Size", width: 7, priority: 3, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <TextCell muted>{formatBytes(row.original.sizeBytes ?? 0)}</TextCell> }),
]);

export function AudiobooksBrowser({ libraryId }: { libraryId: string }) {
  const isAdmin = useIsAdmin();
  const navigate = useNavigate();
  const filters = useBrowserFilters();
  const { actions, dialogs } = useAudiobookEntity<BookRow>();
  const table = useServerTable<AudiobookOrderByInput>({ persistKey: `audiobooks.${libraryId}`, defaultSorting: [{ id: "title", desc: false }], sortMap: { title: "sortTitle" } });
  const [adding, setAdding] = useState(false);

  const where = useMemo<AudiobookWhereInput>(() => {
    const clause: AudiobookWhereInput = { libraryId: { eq: libraryId } };
    const title = titleFilter(filters.debouncedQuery, null);
    if (title) clause.title = title;
    if (filters.availability === "available") clause.hasFiles = { eq: true };
    if (filters.availability === "wanted" || filters.availability === "missing") clause.hasFiles = { eq: false };
    return clause;
  }, [libraryId, filters.debouncedQuery, filters.availability]);

  const list = useInfiniteConnection(LibraryAudiobooksDocument, { where, orderBy: table.orderBy }, (data) => data.audiobooks);
  const jump = useLetterJump({
    rows: list.rows,
    getRowId,
    getTitle: (row) => row.sortTitle || row.title,
    hasMore: list.hasMore,
    loading: list.fetching || list.loadingMore,
    queryKey: JSON.stringify(where),
    isTitleSorted: table.sorting.length === 1 && table.sorting[0]?.id === "title" && !table.sorting[0].desc,
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "title", desc: false }]),
  });

  const renderPoster = useCallback((row: BookRow) => (
      <PosterCard
      title={row.title}
      meta={row.authorName ?? undefined}
      image={audiobookCover(row)}
      tint={LIBRARY_TYPES.audiobooks.tintVar}
      to="/audiobooks/$audiobookId"
      params={{ audiobookId: row.id }}
      badge={row.chapterCount ? <ProgressBadge have={row.downloaded.pageInfo.totalCount ?? 0} total={row.chapterCount} className="text-foreground" /> : undefined}
      />
  ), []);

  return (
    <>
    <MediaBrowserTable<BookRow>
      persistKey="audiobooks"
      label="Audiobooks"
      noun={{ one: "audiobook", other: "audiobooks" }}
      placeholder="Filter audiobooks"
      columns={columns}
      list={list}
      sorting={table.sorting}
      onSortingChange={table.setSorting}
      query={filters.query}
      onQueryChange={filters.setQuery}
      availability={filters.availability}
      onAvailabilityChange={filters.setAvailability}
      getRowId={getRowId}
      getRowLabel={(row) => row.title}
      renderPoster={renderPoster}
      actions={actions}
      onRowOpen={(row) => void navigate({ to: "/audiobooks/$audiobookId", params: { audiobookId: row.id } })}
      emptyMessage="No audiobooks yet. Scan the library folder or add a book."
      emptyAction={isAdmin ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size="md" /> Add audiobook</Button> : undefined}
      trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size="md" /> Add</Button> : null}
      letter={{ active: jump.active, jump: jump.jump, containerRef: jump.containerRef }}
    />
      {dialogs}
      <ProviderSearchDialog kind="audiobook" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </>
  );
}
