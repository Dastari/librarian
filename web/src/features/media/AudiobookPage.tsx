import { copyText } from "glass-ux/copy-button";
import { useApolloClient, useMutation } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Link } from "@tanstack/react-router";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import { createDataTableColumns, filters, NumericCell, TextCell } from "glass-ux/data-table";
import { IconBookmark, IconCopy, IconInfo, IconPlay, IconPlus } from "glass-ux/icons";
import { createContext, useCallback, useContext, useMemo } from "react";

import { EmptyState, ErrorState, MetaLine, SkeletonHero, StatusChip } from "@/components/ui";
import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { AcquisitionChip } from "@/features/acquisition/AcquisitionChip";
import { autoDownloadMeta } from "@/features/acquisition/mode";
import { usePlayer } from "@/features/player/usePlayer";
import { playerStore, type PlayItem } from "@/features/player/store";
import { AudiobookDetailDocument, AudiobookChaptersPageDocument, AudiobookPartsDocument, EntityChapterUpdateDocument, EntityChapterUpdateManyDocument, type AudiobookDetailQuery } from "@/graphql/generated/graphql";
import { useContentStatuses, contentStatusMeta } from "@/hooks/useContentStatuses";
import { audiobookCover } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatClock, formatRuntime } from "@/lib/format";
import { activeContentStatusQueries } from "@/lib/graphql/content-status";
import { assertSuccess } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { statusMeta } from "@/lib/status";

import { FINISHED_AT, heardFraction, heardPosition, type FileLength } from "./chapterProgress";
import { MediaActions } from "./MediaActions";
import { useAudiobookEntity } from "./useAudiobookEntity";
import { noRowMenu, PlayProgressCell, RowActions, type RowPlayState } from "./RowPlayAction";
import { DetailShell } from "./DetailShell";
import { useAudioPages } from "./useAudioPages";
import { filesWithProgress, resetProgressAction, useResetProgress, type ProgressReset } from "./progressReset";
import { useMediaProgress } from "./useMediaProgress";

type ChapterRow = NonNullable<AudiobookDetailQuery["audiobook"]>["chapters"]["edges"][number]["node"];


type ChapterCellProps = { row: { original: ChapterRow } };
const AudiobookTableContext = createContext<{
  actions: ActionSet<ChapterRow>;
  statuses: ReturnType<typeof useContentStatuses>;
  currentItemId: string | undefined;
  fractionOf: (chapter: ChapterRow) => number;
  playStateOf: (chapter: ChapterRow) => RowPlayState;
  press: (chapter: ChapterRow) => void;
  want?: (chapter: ChapterRow) => Promise<unknown>;
  unavailable?: string;
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
  const { statuses } = useAudiobookTable();
  return <StatusChip status={chapter.mediaFileId ? statusMeta("AVAILABLE") : contentStatusMeta(statuses, chapter.id)} />;
}

function ChapterProgressCell({ row: { original: chapter } }: ChapterCellProps) {
  const { fractionOf } = useAudiobookTable();
  return <PlayProgressCell fraction={fractionOf(chapter)} label={`Listening progress for ${chapter.title ?? `chapter ${chapter.chapterNumber}`}`} available={Boolean(chapter.mediaFileId)} />;
}

function ChapterFormatCell({ row: { original: chapter } }: ChapterCellProps) {
  return <TextCell muted>{chapter.mediaFile?.audioCodec?.toUpperCase() ?? "—"}</TextCell>;
}

function ChapterDurationCell({ row: { original: chapter } }: ChapterCellProps) {
  return <TextCell identifier>{formatClock(chapter.durationSecs ?? chapter.mediaFile?.duration)}</TextCell>;
}

function ChapterActionsCell({ row: { original: chapter } }: ChapterCellProps) {
  const { actions, playStateOf, press, want, unavailable } = useAudiobookTable();
  return (
    <RowActions
      set={actions}
      target={chapter}
      label={chapter.title ?? `Chapter ${chapter.chapterNumber}`}
      play={chapter.mediaFileId ? { state: playStateOf(chapter), onPress: () => press(chapter), disabledReason: unavailable } : undefined}
      want={want && !chapter.mediaFileId ? { wanted: chapter.wanted, onPress: () => want(chapter) } : undefined}
    />
  );
}

// Keep component types stable for flexRender; context also propagates progress
// and player changes when the table memoizes an unchanged chapter row.
const chapterColumn = createDataTableColumns<ChapterRow>();
const chapterColumns = chapterColumn.columns([
  chapterColumn.accessor("title", { header: "Chapter", sortable: true, filter: filters.text({ field: "title" }), cardRole: "title", priority: 1, width: 20, minWidth: 10, hideable: false, cell: ChapterTitleCell }),
  chapterColumn.accessor("chapterNumber", { header: "Number", sortable: true, filter: filters.number({ field: "chapterNumber", server: "int" }), priority: 2, width: 5, minWidth: 4, align: "end", grow: false, cell: ChapterNumberCell }),
  chapterColumn.display({ id: "status", header: "Status", cardRole: "status", priority: 2, width: 9, minWidth: 7, cell: ChapterStatusCell }),
  chapterColumn.display({ id: "progress", header: "Progress", priority: 2, width: 11, minWidth: 8, cell: ChapterProgressCell }),
  chapterColumn.display({ id: "format", header: "Format", priority: 5, width: 6, minWidth: 5, cell: ChapterFormatCell }),
  chapterColumn.display({ id: "duration", header: "Length", priority: 3, width: 6, minWidth: 5, align: "end", grow: false, cell: ChapterDurationCell }),
  chapterColumn.display({ id: "chapterActions", header: "", priority: 1, width: 7, minWidth: 6, grow: false, hideable: false, cell: ChapterActionsCell }),
]);

export function AudiobookPage({ audiobookId }: { audiobookId: string }) {
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
  const [updateChapter] = useMutation(EntityChapterUpdateDocument, { refetchQueries: () => activeContentStatusQueries(client) });
  const [updateChapters] = useMutation(EntityChapterUpdateManyDocument, { refetchQueries: () => activeContentStatusQueries(client) });

  const toggleWanted = useCallback(async (chapter: ChapterRow) => {
    if (!isAdmin) return;
    const result = assertSuccess((await updateChapter({ variables: { id: chapter.id, input: { wanted: !chapter.wanted } } })).data?.updateChapter, "Could not update chapter");
    if (result.chapter) pages.updateRow({ ...chapter, ...result.chapter });
    await refetch();
  }, [isAdmin, updateChapter, refetch, pages.updateRow]);

  // One file holding the whole book: it plays once, and a chapter is a place in it.
  const wholeFile = chapters.length > 1 && Boolean(chapters[0]?.mediaFileId) && chapters.every((chapter) => chapter.mediaFileId === chapters[0]!.mediaFileId);
  // Files cut without regard to chapters: they play in order, and a chapter is a place in one of them.
  const inParts = book?.fileShape === "parts" && !wholeFile;
  const partsQuery = useQuery(AudiobookPartsDocument, { variables: { chapterIds: ids }, skip: !inParts || ids.length === 0 });
  const parts = useMemo(() => (partsQuery.data?.mediaFiles.edges ?? []).map((edge) => edge.node).sort((a, b) => a.path.localeCompare(b.path, undefined, { numeric: true })), [partsQuery.data]);
  const partIndex = useCallback((id: string | null | undefined) => parts.findIndex((part) => part.id === id), [parts]);
  // Parts are files of their own, and listening is saved against them, whether or not a chapter starts in one.
  const progressFileIds = useMemo(() => [...new Set([...fileIds, ...parts.map((part) => part.id)])], [fileIds, parts]);
  const progress = useMediaProgress(progressFileIds, user?.id);
  const { progressByFile } = progress;
  const chapterStart = useCallback((chapter: ChapterRow) => inParts ? chapter.fileOffsetSecs ?? 0 : chapter.startTimeSecs, [inParts]);
  const chapterEnd = useCallback((chapter: ChapterRow) => inParts ? (chapter.fileOffsetSecs ?? 0) + (chapter.durationSecs ?? 0) : chapter.endTimeSecs ?? chapter.startTimeSecs + (chapter.durationSecs ?? 0), [inParts]);
  const chapterAt = useCallback((position: number, fileId?: string | null) => {
    if (!inParts) return [...chapters].reverse().find((chapter) => chapter.startTimeSecs <= position);
    const current = partIndex(fileId);
    return [...chapters].reverse().find((chapter) => {
      const holder = partIndex(chapter.mediaFileId);
      return holder >= 0 && (holder < current || (holder === current && chapterStart(chapter) <= position));
    });
  }, [chapters, inParts, partIndex, chapterStart]);
  // The files chapters are places in, when they don't each have their own.
  const sharedFiles = useMemo<FileLength[] | null>(() => {
    if (inParts) return parts.map((part) => ({ id: part.id, seconds: part.duration ?? 0 }));
    if (wholeFile && chapters[0]?.mediaFileId) return [{ id: chapters[0].mediaFileId, seconds: chapters[0].mediaFile?.duration ?? 0 }];
    return null;
  }, [inParts, parts, wholeFile, chapters]);
  const livePosition = player.audio?.audiobookId === audiobookId ? player.transport.position : 0;
  const liveFile = player.audio?.mediaFileId;
  const fractionOf = useCallback((chapter: ChapterRow) => {
    const fileId = chapter.mediaFileId;
    if (!fileId) return 0;
    const positionOf = (id: string, seconds: number) => id === liveFile && livePosition > 0 ? livePosition : heardPosition(progressByFile.get(id), seconds);
    if (!sharedFiles) {
      const seconds = chapter.mediaFile?.duration ?? chapter.durationSecs ?? 0;
      if (progressByFile.get(fileId)?.isWatched && fileId !== liveFile) return 1;
      return seconds > 0 ? Math.min(1, positionOf(fileId, seconds) / seconds) : 0;
    }
    const next = chapters[chapters.findIndex((other) => other.id === chapter.id) + 1];
    return heardFraction(
      sharedFiles,
      { fileId, offset: chapterStart(chapter) },
      next?.mediaFileId ? { fileId: next.mediaFileId, offset: chapterStart(next) } : null,
      (id) => positionOf(id, sharedFiles.find((file) => file.id === id)?.seconds ?? 0),
    );
  }, [sharedFiles, chapters, chapterStart, liveFile, livePosition, progressByFile]);
  const isFinished = useCallback((chapter: ChapterRow) => fractionOf(chapter) >= FINISHED_AT, [fractionOf]);

  const chapterName = (chapter: ChapterRow) => chapter.title ?? `Chapter ${chapter.chapterNumber}`;
  const queue = useMemo<PlayItem[]>(() => !book ? [] : inParts ? parts.map((part, index) => {
    const here = chapters.filter((chapter) => chapter.mediaFileId === part.id);
    const before = chapters.filter((chapter) => { const holder = partIndex(chapter.mediaFileId); return holder >= 0 && holder < index; }).at(-1);
    return {
      mediaFileId: part.id,
      title: book.title,
      subtitle: `Part ${index + 1} of ${parts.length}`,
      artwork: audiobookCover(book),
      entity: { kind: "chapter", id: part.chapterId ?? chapters[0]!.id },
      audiobookId: book.id,
      href: `/audiobooks/${book.id}`,
      duration: part.duration ?? undefined,
      chapters: here.map((chapter) => ({ start: chapter.fileOffsetSecs ?? 0, title: chapterName(chapter) })),
      runningChapter: before ? chapterName(before) : undefined,
    };
  }) : wholeFile ? [{
    mediaFileId: chapters[0]!.mediaFileId!,
    title: book.title,
    subtitle: book.authorName ?? undefined,
    artwork: audiobookCover(book),
    entity: { kind: "chapter", id: chapters[0]!.id },
    audiobookId: book.id,
    href: `/audiobooks/${book.id}`,
    duration: chapters[0]!.mediaFile?.duration ?? undefined,
    chapters: chapters.map((chapter) => ({ start: chapter.startTimeSecs, title: chapterName(chapter) })),
  }] : chapters
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
    })), [chapters, book, wholeFile, inParts, parts, partIndex]);
  const resumeIndex = Math.max(0, queue.findIndex((item) => !progressByFile.get(item.mediaFileId)?.isWatched));
  const resumeItem = queue[resumeIndex];
  const savedResume = resumeItem ? progressByFile.get(resumeItem.mediaFileId) : undefined;
  const resumeChapter = wholeFile || inParts ? chapterAt(savedResume?.currentPosition ?? 0, resumeItem?.mediaFileId) : chapters.find((chapter) => chapter.id === resumeItem?.entity.id);
  const resumeLabel = resumeIndex > 0 || (savedResume && !savedResume.isWatched && savedResume.currentPosition > 0)
    ? `Resume chapter ${resumeChapter?.chapterNumber ?? resumeIndex + 1}` : "Listen";
  const progressUnavailable = progress.error ? "Retry playback progress before listening" : progress.loading ? "Loading playback progress" : undefined;
  const resetProgress = useResetProgress();
  // The whole book is every file it is in (chapters' files, or its parts), so it waits for the full list and for progress.
  const bookReset = useMemo<ProgressReset<unknown>>(() => ({
    files: () => filesWithProgress(progressFileIds, progressByFile),
    disabledReason: () => (!pages.complete ? "Loading all chapters" : progressUnavailable),
  }), [progressFileIds, progressByFile, pages.complete, progressUnavailable]);
  // A chapter has progress of its own only in a file of its own. Chapters that are places in one
  // file (the whole book, or parts) share that file's single position, so only the book resets.
  const chapterReset = useMemo<ProgressReset<ChapterRow>>(() => ({
    files: (rows) => {
      if (wholeFile || inParts) return [];
      const holders = new Map<string, number>();
      for (const chapter of chapters) if (chapter.mediaFileId) holders.set(chapter.mediaFileId, (holders.get(chapter.mediaFileId) ?? 0) + 1);
      return filesWithProgress(rows.map((row) => row.mediaFileId).filter((id) => id && holders.get(id) === 1), progressByFile);
    },
    disabledReason: () => progressUnavailable,
  }), [wholeFile, inParts, chapters, progressByFile, progressUnavailable]);
  const playFrom = useCallback((chapter?: ChapterRow) => {
    if (progressUnavailable || (!chapter && !pages.complete)) return;
    if (inParts) {
      const index = chapter ? partIndex(chapter.mediaFileId) : resumeIndex;
      if (index < 0 || !queue[index]) return;
      const saved = progressByFile.get(queue[index]!.mediaFileId);
      const position = saved && !saved.isWatched ? saved.currentPosition : 0;
      const start = !chapter || (position > chapterStart(chapter) && position < chapterEnd(chapter)) ? position : chapterStart(chapter);
      player.playAudio(queue.map((item, at) => ({ ...item, startPosition: at === index ? start : 0 })), index);
      return;
    }
    if (wholeFile) {
      const item = queue[0];
      if (!item) return;
      const saved = progressByFile.get(item.mediaFileId);
      const position = saved && !saved.isWatched ? saved.currentPosition : 0;
      // Asking for a chapter starts it, unless listening had got part-way through it.
      const start = !chapter || (position > chapter.startTimeSecs && position < chapterEnd(chapter)) ? position : chapter.startTimeSecs;
      player.playAudio([{ ...item, startPosition: start }], 0);
      return;
    }
    const playable = pages.complete ? queue : queue.filter((item) => item.entity.id === chapter?.id);
    const index = chapter ? playable.findIndex((item) => item.entity.id === chapter.id) : resumeIndex;
    if (playable.length === 0 || index < 0) return;
    // Watched chapters always restart, including explicit row selection and later queue items.
    const items = playable.map((item, position) => {
      const saved = progressByFile.get(item.mediaFileId);
      return { ...item, startPosition: position === index && saved && !saved.isWatched ? saved.currentPosition : 0 };
    });
    player.playAudio(items, index);
  }, [progressUnavailable, pages.complete, queue, resumeIndex, progressByFile, player, wholeFile, inParts, partIndex, chapterStart, chapterEnd]);
  const setSelectedWanted = useCallback(async (selected: readonly ChapterRow[], wanted: boolean) => {
    if (!isAdmin) throw new Error("Administrator access required");
    const missing = selected.filter((item) => !item.mediaFileId && item.wanted !== wanted);
    if (!missing.length) throw new Error("No applicable missing chapters selected");
    try {
      const { data } = await updateChapters({ variables: { where: { id: { inList: missing.map((item) => item.id) }, audiobookId: { eq: audiobookId }, mediaFileId: { isNull: true } }, input: { wanted } } });
      const result = assertSuccess(data?.updateChapters, "Could not update selected chapters");
      if (result.affectedCount !== missing.length) throw new Error(`Updated ${result.affectedCount} of ${missing.length} selected chapters. Refresh and retry the remaining items.`);
      for (const item of missing) pages.updateRow({ ...item, wanted });
    } finally { await refetch(); }
  }, [isAdmin, audiobookId, updateChapters, pages.updateRow, refetch]);
  const selectedQueue = useCallback((selectedRows: readonly ChapterRow[]) => {
    const selected = new Set(selectedRows.map((item) => item.id));
    return queue.filter((item) => selected.has(item.entity.id)).map((item, index) => {
      const saved = progressByFile.get(item.mediaFileId);
      return { ...item, startPosition: index === 0 && saved && !saved.isWatched ? saved.currentPosition : 0 };
    });
  }, [queue, progressByFile]);
  const selectedPlaybackUnavailable = (rows: readonly ChapterRow[]) => rows.some((item) => !item.mediaFileId) ? "Some selected chapters have no file. Deselect them to play this selection." : progressUnavailable;
  const actions = useMemo(() => defineActions<ChapterRow>("chapter", [
    { id: "play", label: ([chapter]) => {
        const saved = chapter?.mediaFileId ? progressByFile.get(chapter.mediaFileId) : undefined;
        if (wholeFile || inParts) return chapter && saved && !saved.isWatched && saved.currentPosition > chapterStart(chapter) && saved.currentPosition < chapterEnd(chapter) ? "Resume" : "Play";
        return saved && !saved.isWatched && saved.currentPosition > 0 ? "Resume" : "Play";
      }, icon: IconPlay, group: "open", primary: true, bulk: true,
      visible: (rows) => rows.length === 1 && Boolean(rows[0]?.mediaFileId),
      disabledReason: () => progressUnavailable,
      run: ([chapter]) => { if (chapter?.mediaFileId) playFrom(chapter); } },
    { id: "wanted", label: ([chapter]) => chapter?.wanted ? "Wanted" : "Want", icon: IconBookmark, group: "manage", priority: 1, bulk: true,
      visible: (rows) => rows.length === 1 && isAdmin && Boolean(rows[0] && !rows[0].mediaFileId),
      run: ([chapter]) => chapter && toggleWanted(chapter) },
    { id: "play-selected", label: "Play selected chapters", icon: IconPlay, group: "open", bulk: true,
      visible: (rows) => rows.length > 1 && !wholeFile && !inParts, disabledReason: selectedPlaybackUnavailable,
      run: (rows) => { const items = selectedQueue(rows); if (!items.length || selectedPlaybackUnavailable(rows)) throw new Error("Selected chapters are not ready to play"); player.playAudio(items); } },
    { id: "enqueue", label: "Add to queue", icon: IconPlus, group: "organise", bulk: true,
      visible: () => !wholeFile && !inParts,
      disabledReason: selectedPlaybackUnavailable,
      run: (rows) => { const items = selectedQueue(rows).map((item) => ({ ...item, startPosition: 0 })); if (!items.length || selectedPlaybackUnavailable(rows)) throw new Error("Selected chapters are not ready to queue"); player.enqueue(items); }, successMessage: () => "Added to queue" },
    { id: "want-selected", label: "Want selected missing chapters", icon: IconBookmark, group: "manage", bulk: true,
      visible: (rows) => isAdmin && rows.length > 1 && rows.some((item) => !item.mediaFileId && !item.wanted),
      run: (rows) => setSelectedWanted(rows, true), successMessage: () => "Selected missing chapters wanted" },
    { id: "unwant-selected", label: "Unwant selected missing chapters", icon: IconBookmark, group: "manage", bulk: true,
      visible: (rows) => isAdmin && rows.length > 1 && rows.some((item) => !item.mediaFileId && item.wanted),
      run: (rows) => setSelectedWanted(rows, false), successMessage: () => "Selected missing chapters no longer wanted" },
    ...resetProgressAction<ChapterRow>("audio", chapterReset, resetProgress),
    { id: "copy-titles", label: "Copy chapter titles", icon: IconCopy, group: "organise", bulk: true,
      run: async (rows) => { await copyText(rows.map((item) => item.title ?? `Chapter ${item.chapterNumber}`).join("\n")); }, successMessage: () => "Chapter titles copied" },
  ]), [isAdmin, playFrom, toggleWanted, setSelectedWanted, selectedQueue, player, progressByFile, progressUnavailable, wholeFile, inParts, chapterStart, chapterEnd, chapterReset, resetProgress]);

  const currentItemId = (wholeFile || inParts) && player.audio?.audiobookId === audiobookId ? chapterAt(player.transport.position, player.audio.mediaFileId)?.id : player.audio?.entity.id;
  const playStateOf = (chapter: ChapterRow): RowPlayState => {
    if (chapter.id === currentItemId && player.transport.playing) return "playing";
    if (isFinished(chapter)) return "finished";
    return fractionOf(chapter) > 0 ? "resume" : "ready";
  };
  // Pressing what's already playing pauses or carries on; pressing anything else starts it.
  const press = (chapter: ChapterRow) => {
    if (chapter.id === currentItemId) playerStore.updateTransport({ playing: !player.transport.playing });
    else playFrom(chapter);
  };

  const playBook = queue.length > 0 ? { label: resumeLabel, run: () => playFrom(), disabledReason: !pages.complete ? "Load all chapters to listen to the book" : progressUnavailable } : undefined;
  const { actions: bookActions, dialogs: bookDialogs, openSettings } = useAudiobookEntity<NonNullable<typeof book>>({ open: false, play: playBook, onChanged: () => { void refetch(); pages.refresh(); }, reset: bookReset });

  if (error && !book) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!book) return loading ? <SkeletonHero /> : <EmptyState icon={IconInfo} title="Audiobook not found" description="This item may have been removed from the library." className="page-gutter my-8" />;

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
            <MetaLine items={[book.authorName, book.narratorName ? `read by ${book.narratorName}` : null, book.seriesName ? [book.seriesName, book.seriesPosition && `#${book.seriesPosition}`].filter(Boolean).join(" ") : null, formatRuntime(book.totalDurationSecs), `${book.chapters.pageInfo.totalCount ?? book.chapterCount ?? chapters.length} chapters`]} />
            <AcquisitionChip status={autoDownloadMeta(book.autoDownloadMode)} onPress={isAdmin ? () => openSettings(book, "downloads") : undefined} />
          </>
        ),
        description: book.description,
        actions: (
          <MediaActions target={book} label={book.title} actions={bookActions} play={playBook} />
        ),
      }}
    >
      <AudiobookTableContext.Provider value={{ actions, statuses, currentItemId, fractionOf, playStateOf, press, want: isAdmin ? toggleWanted : undefined, unavailable: progressUnavailable }}>
        <LoadedDataTable
          id={`audiobook-chapters:${user?.id ?? "anonymous"}:${book.libraryId}:${book.id}`}
          label={`${book.title} chapters`} columns={chapterColumns} rows={chapters} getRowId={(chapter) => chapter.id} getRowLabel={(chapter) => chapter.title ?? `Chapter ${chapter.chapterNumber}`}
          height="auto" noun={{ one: "chapter", other: "chapters" }} selectable actions={actions} renderRowActions={noRowMenu}
          renderBulkActions={(rows) => <TableBulkActions set={actions} rows={rows} noun={{ one: "chapter", other: "chapters" }} />}
          complete={pages.complete} search={{ fields: ["title"], placeholder: "Search chapters" }} refetching={loading || pages.loading || progress.loading}
          status={pages.error ? <span role="status">Could not load all chapters. <Button size="sm" variant="ghost" onClick={pages.retry}>Retry chapters</Button></span>
            : progress.error ? <span role="status">Could not load playback progress. <Button size="sm" variant="ghost" onClick={() => void progress.refetch().catch(() => {})}>Retry progress</Button></span>
            : pages.loading ? "Loading remaining chapters…" : progress.loading ? "Loading playback progress…"
            : book.chapterCheckState === "listening" ? "Listening for chapters…"
            : error ? <span role="status">Could not refresh chapters. <Button size="sm" variant="ghost" onClick={() => void refetch()}>Retry refresh</Button></span> : book.chapterCheck ?? undefined}
          emptyMessage="No chapters" onRowOpen={(chapter) => { if (chapter.mediaFileId) playFrom(chapter); }}
        />
      </AudiobookTableContext.Provider>
      {bookDialogs}
    </DetailShell>
  );
}
