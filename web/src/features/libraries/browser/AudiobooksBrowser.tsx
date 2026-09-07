
import { useNavigate } from "@tanstack/react-router";
import { IconBooks, IconPlus } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, DataTable, type DataTableColumn, EmptyState, ErrorState, PosterCard } from "@/components/ui";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryAudiobooksDocument, type AudiobookOrderByInput, type AudiobookWhereInput, type LibraryAudiobooksQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { audiobookCover } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes, formatRuntime } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { BrowserLetterRail, BrowserToolbar } from "./BrowserToolbar";
import { ProgressBadge } from "./ShowsBrowser";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type BookRow = LibraryAudiobooksQuery["audiobooks"]["edges"][number]["node"];
const getRowId = (row: BookRow) => row.id;

export function AudiobooksBrowser({ libraryId }: { libraryId: string }) {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const filters = useBrowserFilters(`audiobooks.${libraryId}`);
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
    getTitle: (row) => row.title,
    hasMore: list.hasMore,
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "title", desc: false }]),
  });

  const columns = useMemo<Array<DataTableColumn<BookRow>>>(
    () => [
      {
        id: "title",
        header: "Audiobook",
        sortable: true,
        cell: (row) => (
          <span className="flex items-center gap-3">
            <img src={audiobookCover(row)} alt="" className="h-12 w-8 shrink-0 rounded-sm object-cover" loading="lazy" />
            <span className="min-w-0">
              <span className="block truncate text-body-sm text-foreground">{row.title}</span>
              <span className="block truncate text-label-sm text-muted">{[row.authorName, row.narratorName ? `read by ${row.narratorName}` : null].filter(Boolean).join(" · ")}</span>
            </span>
          </span>
        ),
      },
      { id: "chapters", header: "Chapters", size: 100, align: "end", cell: (row) => <ProgressBadge have={row.downloaded.pageInfo.totalCount ?? 0} total={row.chapterCount ?? 0} /> },
      { id: "totalDurationSecs", header: "Length", size: 100, align: "end", numeric: true, sortable: true, hideBelow: "md", cell: (row) => formatRuntime(row.totalDurationSecs) },
      { id: "sizeBytes", header: "Size", size: 100, align: "end", numeric: true, sortable: true, hideBelow: "lg", cell: (row) => formatBytes(row.sizeBytes) },
      { id: "publisher", header: "Publisher", size: 160, hideBelow: "xl", cell: (row) => <span className="text-muted">{row.publisher ?? "—"}</span> },
    ],
    [],
  );

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <BrowserToolbar
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={jump.active}
        onLetterChange={jump.jump}
        availability={filters.availability}
        onAvailabilityChange={filters.setAvailability}
        view={filters.view}
        onViewChange={filters.setView}
        total={list.totalCount}
        placeholder="Filter audiobooks"
        trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size={16} /> Add</Button> : null}
      />
      <div className="flex min-h-0 flex-1 gap-4">
        <DataTable<BookRow>
          className="min-w-0 flex-1"
          view={filters.view}
          columns={columns}
          rows={list.rows}
          getRowId={getRowId}
          isLoading={list.loading}
          totalCount={list.totalCount}
          noun="audiobooks"
          sorting={table.sorting}
          onSortingChange={table.setSorting}
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={<EmptyState icon={IconBooks} title="No audiobooks yet" description="Scan the library folder or add a book from Open Library." action={isAdmin ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size={16} /> Add audiobook</Button> : undefined} />}
          onRowClick={(row) => void navigate({ to: "/audiobooks/$audiobookId", params: { audiobookId: row.id } })}
          renderCard={(row) => (
            <PosterCard
              title={row.title}
              meta={row.authorName ?? undefined}
              image={audiobookCover(row)}
              tint={LIBRARY_TYPES.audiobooks.tintVar}
              to="/audiobooks/$audiobookId"
              params={{ audiobookId: row.id }}
              badge={row.chapterCount ? <ProgressBadge have={row.downloaded.pageInfo.totalCount ?? 0} total={row.chapterCount} className="text-foreground" /> : undefined}
            />
          )}
        />
        <BrowserLetterRail letter={jump.active} onLetterChange={jump.jump} />
      </div>
      <ProviderSearchDialog kind="audiobook" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </div>
  );
}
