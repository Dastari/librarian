import { useApolloClient, useMutation } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { toast } from "glass-ux/toast";
import { Link, useNavigate } from "@tanstack/react-router";
import { ActionBar, defineActions, type ActionSet } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import { createDataTableColumns, filters, NumericCell, TextCell } from "glass-ux/data-table";
import { IconBookmark, IconInfo, IconPlay } from "glass-ux/icons";
import { createContext, useCallback, useContext, useMemo, useState } from "react";

import { ConfirmDialog, EmptyState, ErrorState, KeyValueList, MetaLine, Panel, SkeletonHero, StatusChip } from "@/components/ui";
import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { AcquisitionChip } from "@/features/acquisition/AcquisitionChip";
import { AcquisitionDialog } from "@/features/acquisition/AcquisitionDialog";
import { autoDownloadMeta } from "@/features/acquisition/mode";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { usePlayer } from "@/features/player/usePlayer";
import type { PlayItem } from "@/features/player/store";
import { AudiobookDetailDocument, AudiobookChaptersPageDocument, EntityAudiobookDeleteDocument, EntityChapterUpdateDocument, type AudiobookDetailQuery } from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { audiobookCover } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatBytes, formatClock, formatDate, formatRuntime } from "@/lib/format";
import { activeContentStatusQueries } from "@/lib/graphql/content-status";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { statusMeta } from "@/lib/status";

import { MediaActions } from "./MediaActions";
import { DetailShell } from "./DetailShell";
import { useAudioPages } from "./useAudioPages";
import { useMediaProgress } from "./useMediaProgress";

type ChapterRow = NonNullable<AudiobookDetailQuery["audiobook"]>["chapters"]["edges"][number]["node"];

type ChapterCellProps = { row: { original: ChapterRow } };
const AudiobookTableContext = createContext<{
  actions: ActionSet<ChapterRow>;
  statuses: ReturnType<typeof useContentStatuses>;
  progressByFile: ReturnType<typeof useMediaProgress>["progressByFile"];
  currentItemId: string | undefined;
} | null>(null);

function useAudiobookTable() {
  const context = useContext(AudiobookTableContext);
  if (!context) throw new Error("Audiobook cells require AudiobookTableContext");
  return context;
}

function ChapterTitleCell({ row: { original: chapter } }: ChapterCellProps) {
  const { currentItemId } = useAudiobookTable();
  return <TextCell><span className={currentItemId === chapter.id ? "text-brand" : undefined}>{chapter.title ?? `Chapter ${chapter.chapterNumber}`}</span></TextCell>;
}

function ChapterNumberCell({ row }: ChapterCellProps) {
  return <NumericCell value={row.original.chapterNumber} />;
}

function ChapterStatusCell({ row: { original: chapter } }: ChapterCellProps) {
  const { progressByFile, statuses } = useAudiobookTable();
  return chapter.mediaFileId && progressByFile.get(chapter.mediaFileId)?.isWatched
    ? <StatusChip status={{ label: "Finished", tone: "success", dot: "bg-success" }} />
    : <StatusChip status={statusMeta(chapter.mediaFileId ? "AVAILABLE" : statuses.get(chapter.id))} />;
}

function ChapterFormatCell({ row: { original: chapter } }: ChapterCellProps) {
  return <TextCell muted>{chapter.mediaFile ? [chapter.mediaFile.audioCodec?.toUpperCase(), formatBytes(chapter.mediaFile.size)].filter(Boolean).join(" · ") : "—"}</TextCell>;
}

function ChapterDurationCell({ row: { original: chapter } }: ChapterCellProps) {
  return <TextCell identifier>{formatClock(chapter.durationSecs ?? chapter.mediaFile?.duration)}</TextCell>;
}

function ChapterActionsCell({ row }: ChapterCellProps) {
  const { actions } = useAudiobookTable();
  return <ActionBar set={actions} targets={[row.original]} label={row.original.title ?? `Chapter ${row.original.chapterNumber}`} maxButtons={1} />;
}

// Keep component types stable for flexRender; context also propagates progress
// and player changes when the table memoizes an unchanged chapter row.
const chapterColumn = createDataTableColumns<ChapterRow>();
const chapterColumns = chapterColumn.columns([
  chapterColumn.accessor("title", { header: "Chapter", sortable: true, filter: filters.text({ field: "title" }), cardRole: "title", priority: 1, width: 20, minWidth: 10, hideable: false, cell: ChapterTitleCell }),
  chapterColumn.accessor("chapterNumber", { header: "Number", sortable: true, filter: filters.number({ field: "chapterNumber", server: "int" }), priority: 2, width: 5, minWidth: 4, align: "end", grow: false, cell: ChapterNumberCell }),
  chapterColumn.display({ id: "status", header: "Status", cardRole: "status", priority: 2, width: 9, minWidth: 7, cell: ChapterStatusCell }),
  chapterColumn.display({ id: "format", header: "Format", priority: 4, width: 12, minWidth: 8, cell: ChapterFormatCell }),
  chapterColumn.display({ id: "duration", header: "Length", priority: 3, width: 6, minWidth: 5, align: "end", grow: false, cell: ChapterDurationCell }),
  chapterColumn.display({ id: "chapterActions", header: "Actions", priority: 1, width: 12, minWidth: 10, grow: false, hideable: false, cell: ChapterActionsCell }),
]);

export function AudiobookPage({ audiobookId }: { audiobookId: string }) {
  const navigate = useNavigate();
  const client = useApolloClient();
  const player = usePlayer();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const { data, previousData, loading, error, refetch } = useQuery(AudiobookDetailDocument, { variables: { id: audiobookId }, context: { liveManaged: true } });
  const detail = (data ?? previousData)?.audiobook;
  const book = detail?.id === audiobookId ? detail : null;
  const loadPage = useCallback(async (offset: number) => {
    const result = await client.query({ query: AudiobookChaptersPageDocument, variables: { id: audiobookId, offset }, fetchPolicy: "no-cache" });
    if (!result.data?.audiobook) throw new Error("Audiobook chapters unavailable");
    return result.data.audiobook.chapters;
  }, [client, audiobookId]);
  const pages = useAudioPages(audiobookId, book?.chapters, loadPage);
  const chapters = useMemo(() => [...pages.rows].sort((a, b) => a.chapterNumber - b.chapterNumber), [pages.rows]);
  const ids = useMemo(() => chapters.map((chapter) => chapter.id), [chapters]);
  const statuses = useContentStatuses("CHAPTER", ids);
  const fileIds = useMemo(() => chapters.map((chapter) => chapter.mediaFileId).filter((id): id is string => Boolean(id)), [chapters]);
  const progress = useMediaProgress(fileIds, user?.id);
  const { progressByFile } = progress;
  const [deleteBook, { loading: deleting }] = useMutation(EntityAudiobookDeleteDocument);
  const [updateChapter] = useMutation(EntityChapterUpdateDocument, { refetchQueries: () => activeContentStatusQueries(client) });
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [searching, setSearching] = useState(false);
  const [acquisition, setAcquisition] = useState(false);

  const toggleWanted = useCallback(async (chapter: ChapterRow) => {
    if (!isAdmin) return;
    const result = assertSuccess((await updateChapter({ variables: { id: chapter.id, input: { wanted: !chapter.wanted } } })).data?.updateChapter, "Could not update chapter");
    if (result.chapter) pages.updateRow({ ...chapter, ...result.chapter });
    await refetch();
  }, [isAdmin, updateChapter, refetch, pages.updateRow]);

  const queue = useMemo<PlayItem[]>(() => book ? chapters
    .filter((chapter) => chapter.mediaFileId)
    .map((chapter) => ({
      mediaFileId: chapter.mediaFileId!,
      title: book.title,
      subtitle: chapter.title ?? `Chapter ${chapter.chapterNumber}`,
      artwork: audiobookCover(book),
      entity: { kind: "chapter", id: chapter.id },
      audiobookId: book.id,
      href: `/audiobooks/${book.id}`,
      duration: chapter.durationSecs ?? undefined,
    })) : [], [chapters, book]);
  const resumeIndex = Math.max(0, queue.findIndex((item) => !progressByFile.get(item.mediaFileId)?.isWatched));
  const resumeItem = queue[resumeIndex];
  const resumeChapter = chapters.find((chapter) => chapter.id === resumeItem?.entity.id);
  const savedResume = resumeItem ? progressByFile.get(resumeItem.mediaFileId) : undefined;
  const resumeLabel = resumeIndex > 0 || (savedResume && !savedResume.isWatched && savedResume.currentPosition > 0)
    ? `Resume chapter ${resumeChapter?.chapterNumber ?? resumeIndex + 1}` : "Listen";
  const progressUnavailable = progress.error ? "Retry playback progress before listening" : progress.loading ? "Loading playback progress" : undefined;
  const playFrom = useCallback((chapter?: ChapterRow) => {
    if (progressUnavailable || (!chapter && !pages.complete)) return;
    const playable = pages.complete ? queue : queue.filter((item) => item.entity.id === chapter?.id);
    const index = chapter ? playable.findIndex((item) => item.entity.id === chapter.id) : resumeIndex;
    if (playable.length === 0 || index < 0) return;
    // Watched chapters always restart, including explicit row selection and later queue items.
    const items = playable.map((item, position) => {
      const saved = progressByFile.get(item.mediaFileId);
      return { ...item, startPosition: position === index && saved && !saved.isWatched ? saved.currentPosition : 0 };
    });
    player.playAudio(items, index);
  }, [progressUnavailable, pages.complete, queue, resumeIndex, progressByFile, player]);
  const actions = useMemo(() => defineActions<ChapterRow>("chapter", [
    { id: "play", label: ([chapter]) => {
        const saved = chapter?.mediaFileId ? progressByFile.get(chapter.mediaFileId) : undefined;
        return saved && !saved.isWatched && saved.currentPosition > 0 ? "Resume" : "Play";
      }, icon: IconPlay, group: "open", primary: true,
      visible: ([chapter]) => Boolean(chapter?.mediaFileId),
      disabledReason: () => progressUnavailable,
      run: ([chapter]) => { if (chapter?.mediaFileId) playFrom(chapter); } },
    { id: "wanted", label: ([chapter]) => chapter?.wanted ? "Wanted" : "Want", icon: IconBookmark, group: "manage", priority: 1,
      visible: ([chapter]) => isAdmin && Boolean(chapter && !chapter.mediaFileId),
      run: ([chapter]) => chapter && toggleWanted(chapter) },
  ]), [isAdmin, playFrom, progressByFile, progressUnavailable, toggleWanted]);

  if (error && !book) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!book) return loading ? <SkeletonHero /> : <EmptyState icon={IconInfo} title="Audiobook not found" description="This item may have been removed from the library." className="page-gutter my-8" />;

  const remove = async () => {
    try {
      const { data: result } = await deleteBook({ variables: { id: book.id } });
      assertSuccess(result?.deleteAudiobook, "Could not remove audiobook");
      toast.success("Audiobook removed");
      await navigate({ to: "/libraries/$libraryId/audiobooks", params: { libraryId: book.libraryId } });
    } catch (mutationError) {
      toast.danger(errorMessage(mutationError));
    }
  };

  return (
    <DetailShell
      backdrop={null}
      hero={{
        poster: audiobookCover(book),
        tint: LIBRARY_TYPES.audiobooks.tintVar,
        eyebrow: book.library ? (
          <Link to="/libraries/$libraryId/audiobooks" params={{ libraryId: book.library.id }} className="nav-focus rounded hover:underline">
            {book.library.name}
          </Link>
        ) : null,
        title: book.title,
        meta: (
          <>
            <MetaLine items={[book.authorName, book.narratorName ? `read by ${book.narratorName}` : null, formatRuntime(book.totalDurationSecs), `${book.chapters.pageInfo.totalCount ?? book.chapterCount ?? chapters.length} chapters`]} />
            <AcquisitionChip status={autoDownloadMeta(book.autoDownloadMode)} onPress={isAdmin ? () => setAcquisition(true) : undefined} />
          </>
        ),
        description: book.description,
        actions: (
          <MediaActions
            personal={book.userId === user?.id ? { kind: "audiobook", id: book.id } : undefined}
            id={book.id}
            label={book.title}
            play={queue.length > 0 ? { label: resumeLabel, run: () => playFrom(), disabledReason: !pages.complete ? "Load all chapters to listen to the book" : progressUnavailable } : undefined}
            find={isAdmin ? () => setSearching(true) : undefined}
            settings={isAdmin ? () => setAcquisition(true) : undefined}
            remove={isAdmin ? () => setConfirmDelete(true) : undefined}
          />
        ),
      }}
    >
      <AudiobookTableContext.Provider value={{ actions, statuses, progressByFile, currentItemId: player.audio?.entity.id }}>
        <LoadedDataTable
          id={`audiobook-chapters:${user?.id ?? "anonymous"}:${book.libraryId}:${book.id}`}
          label={`${book.title} chapters`} columns={chapterColumns} rows={chapters} getRowId={(chapter) => chapter.id} getRowLabel={(chapter) => chapter.title ?? `Chapter ${chapter.chapterNumber}`}
          height="auto" density="compact" noun={{ one: "chapter", other: "chapters" }} selectable={false} actions={actions}
          complete={pages.complete} search={{ fields: ["title"], placeholder: "Search chapters" }} refetching={loading || pages.loading || progress.loading}
          status={pages.error ? <span role="status">Could not load all chapters. <Button size="sm" variant="ghost" onClick={pages.retry}>Retry chapters</Button></span>
            : progress.error ? <span role="status">Could not load playback progress. <Button size="sm" variant="ghost" onClick={() => void progress.refetch().catch(() => {})}>Retry progress</Button></span>
            : pages.loading ? "Loading remaining chapters…" : progress.loading ? "Loading playback progress…"
            : error ? <span role="status">Could not refresh chapters. <Button size="sm" variant="ghost" onClick={() => void refetch()}>Retry refresh</Button></span> : undefined}
          emptyMessage="No chapters" onRowOpen={(chapter) => { if (chapter.mediaFileId) playFrom(chapter); }}
        />
      </AudiobookTableContext.Provider>
      <Panel title="Details" className="lg:max-w-2xl">
        <KeyValueList columns={2} items={[{ label: "Publisher", value: book.publisher }, { label: "Published", value: formatDate(book.publishedDate) }, { label: "Language", value: book.language }, { label: "ISBN", value: book.isbn, mono: true }, { label: "ASIN", value: book.asin, mono: true }, { label: "Size", value: formatBytes(book.sizeBytes) }, { label: "Folder", value: book.path, mono: true }]} />
      </Panel>
      <AcquisitionDialog isOpen={acquisition} onOpenChange={setAcquisition} target={{ kind: "audiobook", id: book.id, title: book.title, libraryId: book.libraryId, autoDownloadMode: book.autoDownloadMode, qualityProfileId: book.qualityProfileId }} onSaved={() => void refetch()} />
      <ReleaseSearchDialog isOpen={searching} onOpenChange={setSearching} query={[book.authorName, book.title].filter(Boolean).join(" ")} libraryId={book.libraryId} author={book.authorName} target={{ audiobookId: book.id }} />
      <ConfirmDialog isOpen={confirmDelete} onOpenChange={setConfirmDelete} title={`Remove ${book.title}?`} description="The audiobook and its chapters leave the catalogue. Files stay on disk." confirmLabel="Remove" destructive isPending={deleting} onConfirm={remove} />
    </DetailShell>
  );
}
