import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { IconAdjustments, IconBookmark, IconBookmarkFilled, IconDownload, IconPlayerPlayFilled, IconTrash } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, ConfirmDialog, DataTable, ErrorState, GlassButton, KeyValueList, MetaLine, Panel, SkeletonHero, StatusChip, type DataTableColumn } from "@/components/ui";
import { AcquisitionChip } from "@/features/acquisition/AcquisitionChip";
import { AcquisitionDialog } from "@/features/acquisition/AcquisitionDialog";
import { autoDownloadMeta } from "@/features/acquisition/mode";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { usePlayer } from "@/features/player/usePlayer";
import type { PlayItem } from "@/features/player/store";
import { AudiobookDetailDocument, EntityAudiobookDeleteDocument, EntityChapterUpdateDocument, PlaybackProgressForFilesDocument, type AudiobookDetailQuery } from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { audiobookCover } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatBytes, formatClock, formatDate, formatRuntime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { statusMeta } from "@/lib/status";

import { DetailShell } from "./DetailShell";

type ChapterRow = NonNullable<AudiobookDetailQuery["audiobook"]>["chapters"]["edges"][number]["node"];

export function AudiobookPage({ audiobookId }: { audiobookId: string }) {
  const navigate = useNavigate();
  const player = usePlayer();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const { data, previousData, loading, error, refetch } = useQuery(AudiobookDetailDocument, { variables: { id: audiobookId } });
  const book = (data ?? previousData)?.audiobook ?? null;
  const chapters = useMemo(() => book?.chapters.edges.map((edge) => edge.node) ?? [], [book]);
  const ids = useMemo(() => chapters.map((chapter) => chapter.id), [chapters]);
  const statuses = useContentStatuses("CHAPTER", ids);
  const fileIds = useMemo(() => chapters.map((chapter) => chapter.mediaFileId).filter((id): id is string => Boolean(id)), [chapters]);
  const progress = useQuery(PlaybackProgressForFilesDocument, { variables: { userId: user?.id ?? "", mediaFileIds: fileIds }, skip: !user || fileIds.length === 0 });
  const progressByFile = useMemo(() => new Map(progress.data?.playbackProgresses.edges.map((edge) => [edge.node.mediaFileId, edge.node]) ?? []), [progress.data]);
  const [deleteBook, { loading: deleting }] = useMutation(EntityAudiobookDeleteDocument);
  const [updateChapter] = useMutation(EntityChapterUpdateDocument);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [searching, setSearching] = useState(false);
  const [acquisition, setAcquisition] = useState(false);

  const toggleWanted = async (chapter: ChapterRow) => {
    try {
      assertSuccess((await updateChapter({ variables: { id: chapter.id, input: { wanted: !chapter.wanted } } })).data?.updateChapter, "Could not update chapter");
      void refetch();
    } catch (mutationError) {
      toast.danger(errorMessage(mutationError));
    }
  };

  if (error && !book) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!book) return loading ? <SkeletonHero /> : null;

  const queue: PlayItem[] = chapters
    .filter((chapter) => chapter.mediaFileId)
    .map((chapter) => ({
      mediaFileId: chapter.mediaFileId!,
      title: book.title,
      subtitle: chapter.title ?? `Chapter ${chapter.chapterNumber}`,
      artwork: audiobookCover(book),
      entity: { kind: "chapter", id: chapter.id },
      href: `/audiobooks/${book.id}`,
      duration: chapter.durationSecs ?? undefined,
    }));
  const resumeIndex = Math.max(0, queue.findIndex((item) => !progressByFile.get(item.mediaFileId)?.isWatched));
  const playFrom = (chapter?: ChapterRow) => {
    if (queue.length === 0) return;
    const index = chapter ? Math.max(0, queue.findIndex((item) => item.entity.id === chapter.id)) : resumeIndex;
    const items = queue.map((item, position) => (position === index ? { ...item, startPosition: progressByFile.get(item.mediaFileId)?.currentPosition } : item));
    player.playAudio(items, index);
  };

  const columns: Array<DataTableColumn<ChapterRow>> = [
    { id: "n", header: "#", size: 64, align: "end", numeric: true, cell: (chapter) => (player.audio?.entity.id === chapter.id ? <IconPlayerPlayFilled size={14} className="ml-auto text-brand" /> : chapter.chapterNumber) },
    { id: "title", header: "Chapter", cell: (chapter) => <span className={player.audio?.entity.id === chapter.id ? "text-brand" : "text-foreground"}>{chapter.title ?? `Chapter ${chapter.chapterNumber}`}</span> },
    { id: "status", header: "", size: 130, cell: (chapter) => (chapter.mediaFileId ? (progressByFile.get(chapter.mediaFileId)?.isWatched ? <StatusChip status={{ label: "Finished", tone: "success", dot: "bg-success" }} /> : null) : <StatusChip status={statusMeta(statuses.get(chapter.id))} />) },
    { id: "format", header: "Format", size: 140, hideBelow: "md", cell: (chapter) => <span className="text-muted">{chapter.mediaFile ? [chapter.mediaFile.audioCodec?.toUpperCase(), formatBytes(chapter.mediaFile.size)].filter(Boolean).join(" · ") : "—"}</span> },
    { id: "duration", header: "Length", size: 90, align: "end", numeric: true, cell: (chapter) => formatClock(chapter.durationSecs ?? chapter.mediaFile?.duration) },
    ...(isAdmin
      ? [
          {
            id: "want",
            header: "",
            size: 110,
            align: "end" as const,
            cell: (chapter: ChapterRow) =>
              chapter.mediaFileId ? null : (
                <Button size="sm" variant="ghost" onPress={() => void toggleWanted(chapter)}>
                  {chapter.wanted ? <IconBookmarkFilled size={14} className="text-brand" /> : <IconBookmark size={14} />} {chapter.wanted ? "Wanted" : "Want"}
                </Button>
              ),
          },
        ]
      : []),
  ];

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
      hero={{
        backdrop: audiobookCover(book),
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
            <MetaLine items={[book.authorName, book.narratorName ? `read by ${book.narratorName}` : null, formatRuntime(book.totalDurationSecs), `${chapters.filter((chapter) => chapter.mediaFileId).length}/${chapters.length} chapters`]} />
            <AcquisitionChip status={autoDownloadMeta(book.autoDownloadMode)} onPress={() => setAcquisition(true)} />
          </>
        ),
        description: book.description,
        actions: (
          <>
            {queue.length > 0 ? (
              <GlassButton emphasis="brand" size="lg" refract onPress={() => playFrom()}>
                <IconPlayerPlayFilled /> {resumeIndex > 0 ? `Resume chapter ${chapters.find((chapter) => chapter.id === queue[resumeIndex]?.entity.id)?.chapterNumber ?? resumeIndex + 1}` : "Listen"}
              </GlassButton>
            ) : null}
            {isAdmin ? (
              <>
                <GlassButton size="lg" refract onPress={() => setSearching(true)}>
                  <IconDownload /> Find releases
                </GlassButton>
                <GlassButton size="lg" refract isIconOnly aria-label="Download settings" onPress={() => setAcquisition(true)}>
                  <IconAdjustments />
                </GlassButton>
                <GlassButton size="lg" refract isIconOnly aria-label="Remove from library" onPress={() => setConfirmDelete(true)}>
                  <IconTrash />
                </GlassButton>
              </>
            ) : null}
          </>
        ),
      }}
    >
      <DataTable<ChapterRow> columns={columns} rows={chapters} getRowId={(chapter) => chapter.id} density="compact" noun="chapters" onRowClick={(chapter) => chapter.mediaFileId && playFrom(chapter)} />
      <Panel title="Details" className="lg:max-w-2xl">
        <KeyValueList columns={2} items={[{ label: "Publisher", value: book.publisher }, { label: "Published", value: formatDate(book.publishedDate) }, { label: "Language", value: book.language }, { label: "ISBN", value: book.isbn, mono: true }, { label: "ASIN", value: book.asin, mono: true }, { label: "Size", value: formatBytes(book.sizeBytes) }, { label: "Folder", value: book.path, mono: true }]} />
      </Panel>
      <AcquisitionDialog isOpen={acquisition} onOpenChange={setAcquisition} target={{ kind: "audiobook", id: book.id, title: book.title, libraryId: book.libraryId, autoDownloadMode: book.autoDownloadMode, qualityProfileId: book.qualityProfileId }} onSaved={() => void refetch()} />
      <ReleaseSearchDialog isOpen={searching} onOpenChange={setSearching} query={[book.authorName, book.title].filter(Boolean).join(" ")} libraryId={book.libraryId} author={book.authorName} target={{ audiobookId: book.id }} />
      <ConfirmDialog isOpen={confirmDelete} onOpenChange={setConfirmDelete} title={`Remove ${book.title}?`} description="The audiobook and its chapters leave the catalogue. Files stay on disk." confirmLabel="Remove" destructive isPending={deleting} onConfirm={remove} />
    </DetailShell>
  );
}
