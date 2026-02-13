import {
  createFileRoute,
  Link,
  redirect,
  useNavigate,
} from "@tanstack/react-router";
import { useState, useEffect, useCallback, useMemo } from "react";
import { Button } from "@heroui/button";
import {
  Dropdown,
  DropdownTrigger,
  DropdownMenu,
  DropdownItem,
} from "@heroui/dropdown";
import { Card, CardBody } from "@heroui/card";
import { Chip } from "@heroui/chip";
import { Image } from "@heroui/image";
import { Breadcrumbs, BreadcrumbItem } from "@heroui/breadcrumbs";
import { Progress } from "@heroui/progress";
import { useDisclosure } from "@heroui/modal";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { addToast } from "@heroui/toast";
import { RouteError } from "../../components/RouteError";
import { sanitizeError, formatBytes, formatDuration } from "../../lib/format";
import { useQuery, useMutation } from "../../lib/graphql/client";
import {
  AudiobookDetailSetChapterWantedDocument,
  AudiobookDetailRouteDocument,
  DeleteAudiobookRouteDocument,
  LibraryDetailRouteDocument,
  type AudiobookDetailRouteQuery,
} from "../../lib/graphql/generated/graphql";
import {
  type DataTableColumn,
  type RowAction,
} from "../../components/data-table";
import {
  IconBook,
  IconHeadphones,
  IconSearch,
  IconRefresh,
  IconPlayerPlay,
  IconPlayerPause,
  IconDotsVertical,
  IconInfoCircle,
  IconTrash,
  IconUser,
  IconCheck,
  IconX,
} from "@tabler/icons-react";
import { ChapterStatusChip, PlayPauseIndicator } from "../../components/shared";
import { FilePropertiesModal } from "../../components/FilePropertiesModal";
import { usePlaybackContext } from "../../contexts/PlaybackContext";
import { useDataReactivity } from "../../hooks/useSubscription";
import { DetailItemsTable } from "../../components/media/DetailItemsTable";

export const Route = createFileRoute("/audiobooks/$audiobookId")({
  beforeLoad: ({ context, location }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({
        to: "/",
        search: {
          signin: true,
          redirect: location.href,
        },
      });
    }
  },
  component: AudiobookDetailPage,
  errorComponent: RouteError,
});

type AudiobookDetailNode = NonNullable<AudiobookDetailRouteQuery["Audiobook"]>;
type AudiobookChapterNode =
  AudiobookDetailNode["Chapters"]["Edges"][number]["Node"];
type ChapterStatusView = "missing" | "wanted" | "downloading" | "downloaded";

interface AudiobookChapter {
  id: string;
  audiobookId: string;
  chapterNumber: number;
  title: string | null;
  startSecs: number;
  endSecs: number;
  durationSecs: number | null;
  mediaFileId: string | null;
  status: ChapterStatusView;
  wanted: boolean;
  downloadProgress: number | null;
}

interface AudiobookWithChapters {
  audiobook: {
    id: string;
    authorId: string | null;
    libraryId: string;
    title: string;
    sortTitle: string | null;
    subtitle: string | null;
    openlibraryId: string | null;
    isbn: string | null;
    description: string | null;
    publisher: string | null;
    language: string | null;
    narrators: string[];
    seriesName: string | null;
    durationSecs: number | null;
    coverUrl: string | null;
    hasFiles: boolean;
    sizeBytes: number | null;
    path: string | null;
    chapterCount: number;
    downloadedChapterCount: number;
  };
  author: { name: string } | null;
  chapters: AudiobookChapter[];
  chapterCount: number;
  chaptersWithFiles: number;
  missingChapters: number;
  completionPercent: number;
}

// Chapter table columns
const chapterColumns: DataTableColumn<AudiobookChapter>[] = [
  {
    key: "chapterNumber",
    label: "#",
    width: 60,
    sortable: true,
    render: (ch) => (
      <span className="font-mono text-default-500">
        {String(ch.chapterNumber).padStart(2, "0")}
      </span>
    ),
  },
  {
    key: "title",
    label: "Title",
    sortable: true,
    render: (ch) => (
      <span className="font-medium">
        {ch.title || `Chapter ${ch.chapterNumber}`}
      </span>
    ),
  },
  {
    key: "duration",
    label: "Duration",
    width: 100,
    render: (ch) => (
      <span className="text-default-500 text-sm">
        {ch.durationSecs ? formatDuration(ch.durationSecs) : "-"}
      </span>
    ),
  },
  {
    key: "status",
    label: "Status",
    width: 140,
    sortable: true,
    render: (ch) => (
      <ChapterStatusChip
        mediaFileId={ch.mediaFileId}
        downloadProgress={ch.downloadProgress}
        wanted={ch.wanted}
      />
    ),
  },
];

interface ChapterTableProps {
  chapters: AudiobookChapter[];
  audiobookId: string;
  onPlay: (chapter: AudiobookChapter) => void;
  onSearch: (chapter: AudiobookChapter) => void;
  fetchAudiobook: () => void;
}

function ChapterTable({
  chapters,
  audiobookId,
  onPlay,
  onSearch,
  fetchAudiobook,
}: ChapterTableProps) {
  // Get session and updatePlayback directly from context for reliable updates
  const { session, updatePlayback } = usePlaybackContext();

  // Handle pause directly using context
  const handlePause = useCallback(() => {
    updatePlayback({ isPlaying: false });
  }, [updatePlayback]);

  // Compute playing state from session
  // Find chapter by matching mediaFileId since session doesn't have chapterId
  const currentlyPlayingChapterId =
    session?.audiobookId === audiobookId
      ? (chapters.find((ch) => ch.mediaFileId === session?.mediaFileId)?.id ??
        null)
      : null;
  const isPlaying = session?.isPlaying ?? false;

  // Row actions - computed fresh on each render to ensure playing state is always current
  const rowActions: RowAction<AudiobookChapter>[] = [
    // Playing indicator with pause on hover - shown for currently playing chapter
    {
      key: `playing-${currentlyPlayingChapterId || "none"}`,
      label: "Pause",
      icon: (
        <PlayPauseIndicator
          size={16}
          isPlaying={isPlaying}
          colorClass="bg-success"
        />
      ),
      color: "default",
      inDropdown: false,
      isVisible: (ch) =>
        ch.status === "downloaded" &&
        !!ch.mediaFileId &&
        currentlyPlayingChapterId === ch.id &&
        isPlaying,
      onAction: () => handlePause(),
    },
    // Play action - shown for all other chapters or when paused
    {
      key: `play-${currentlyPlayingChapterId || "none"}-${isPlaying}`,
      label: "Play",
      icon: <IconPlayerPlay size={16} />,
      color: "success",
      inDropdown: false,
      isVisible: (ch) =>
        ch.status === "downloaded" &&
        !!ch.mediaFileId &&
        !(currentlyPlayingChapterId === ch.id && isPlaying),
      onAction: (ch) => onPlay(ch),
    },
    {
      key: "search",
      label: "Search",
      icon: <IconSearch size={16} />,
      color: "default",
      inDropdown: false,
      isVisible: (ch) => ch.status === "missing" || ch.status === "wanted",
      onAction: (ch) => onSearch(ch),
    },
  ];

  // Create selection set for highlighting currently playing chapter
  const selectedKeys = useMemo(() => {
    if (currentlyPlayingChapterId) {
      return new Set([currentlyPlayingChapterId]);
    }
    return new Set<string>();
  }, [currentlyPlayingChapterId]);

  // Key that changes when playback state changes to force re-render
  const tableKey = `chapters-${currentlyPlayingChapterId || "none"}-${isPlaying}`;

  return (
    <DetailItemsTable
      tableKey={tableKey}
      headerContent={
        <div className="p-4">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <IconHeadphones size={20} className="text-orange-400" />
            Chapters
          </h2>
        </div>
      }
      data={chapters}
      columns={chapterColumns}
      emptyContent={
        <div className="p-8 text-center">
          <IconHeadphones size={48} className="mx-auto mb-4 text-default-400" />
          <h3 className="text-lg font-semibold mb-2">No Chapters</h3>
          <p className="text-default-500 mb-4">
            Chapter information hasn't been fetched yet.
          </p>
          <Button variant="flat" onPress={fetchAudiobook}>
            Refresh Audiobook
          </Button>
        </div>
      }
      getRowKey={(ch) => ch.id}
      ariaLabel="Audiobook chapters"
      isCompact
      showItemCount={false}
      hideToolbar
      defaultSortColumn="chapterNumber"
      defaultSortDirection="asc"
      rowActions={rowActions}
      selectionMode={currentlyPlayingChapterId ? "single" : "none"}
      selectedKeys={selectedKeys}
    />
  );
}

function AudiobookDetailPage() {
  const { audiobookId } = Route.useParams();
  const navigate = useNavigate();
  const { startAudiobookPlayback, session, updatePlayback } =
    usePlaybackContext();

  const {
    isOpen: isDeleteOpen,
    onOpen: onDeleteOpen,
    onClose: onDeleteClose,
  } = useDisclosure();
  const {
    isOpen: isPropertiesOpen,
    onOpen: onPropertiesOpen,
    onClose: onPropertiesClose,
  } = useDisclosure();
  const [propertiesMediaFileId, setPropertiesMediaFileId] = useState<
    string | null
  >(null);
  const {
    data: audiobookQueryData,
    previousData: previousAudiobookQueryData,
    loading: isLoading,
    refetch: refetchAudiobook,
  } = useQuery(AudiobookDetailRouteDocument, {
    variables: { Id: audiobookId },
    fetchPolicy: "cache-and-network",
  });

  const audiobookNode: AudiobookDetailNode | null =
    audiobookQueryData?.Audiobook ??
    previousAudiobookQueryData?.Audiobook ??
    null;

  const toChapterStatus = useCallback(
    (
      mediaFileId: string | null | undefined,
      wanted: boolean,
      backendStatus: string,
    ): AudiobookChapter["status"] => {
      if (mediaFileId) return "downloaded";
      const normalized = backendStatus.toLowerCase();
      if (normalized === "downloading") return "downloading";
      return wanted ? "wanted" : "missing";
    },
    [],
  );

  const audiobookData = useMemo<AudiobookWithChapters | null>(() => {
    if (!audiobookNode) return null;
    const chapters = (audiobookNode.Chapters?.Edges ?? []).map(
      (edge: { Node: AudiobookChapterNode }) => ({
        id: edge.Node.Id,
        audiobookId: edge.Node.AudiobookId,
        chapterNumber: edge.Node.ChapterNumber,
        title: edge.Node.Title ?? null,
        startSecs: Math.floor(edge.Node.StartTimeSecs),
        endSecs:
          edge.Node.EndTimeSecs != null ? Math.floor(edge.Node.EndTimeSecs) : 0,
        durationSecs: edge.Node.DurationSecs ?? null,
        mediaFileId: edge.Node.MediaFileId ?? null,
        status: toChapterStatus(
          edge.Node.MediaFileId,
          edge.Node.Wanted,
          edge.Node.Status,
        ),
        wanted: edge.Node.Wanted,
        downloadProgress: null,
      }),
    );
    const chaptersWithFiles = chapters.filter((ch) =>
      Boolean(ch.mediaFileId),
    ).length;
    const missingChapters = chapters.filter((ch) => !ch.mediaFileId).length;
    const completionPercent =
      chapters.length > 0
        ? Math.round((chaptersWithFiles / chapters.length) * 100)
        : 0;
    return {
      audiobook: {
        id: audiobookNode.Id,
        authorId: null,
        libraryId: audiobookNode.LibraryId,
        title: audiobookNode.Title,
        sortTitle: audiobookNode.SortTitle ?? null,
        subtitle: null,
        openlibraryId: null,
        isbn: audiobookNode.Isbn ?? null,
        description: audiobookNode.Description ?? null,
        publisher: audiobookNode.Publisher ?? null,
        language: audiobookNode.Language ?? null,
        narrators: audiobookNode.Narrators,
        seriesName: null,
        durationSecs: audiobookNode.TotalDurationSecs ?? null,
        coverUrl: audiobookNode.CoverUrl ?? null,
        hasFiles: audiobookNode.HasFiles,
        sizeBytes: audiobookNode.SizeBytes ?? null,
        path: audiobookNode.Path ?? null,
        chapterCount: chapters.length,
        downloadedChapterCount: chaptersWithFiles,
      },
      author: null,
      chapters,
      chapterCount: chapters.length,
      chaptersWithFiles,
      missingChapters,
      completionPercent,
    };
  }, [audiobookNode, toChapterStatus]);

  const {
    data: libraryData,
    previousData: previousLibraryData,
    refetch: refetchLibrary,
  } = useQuery(LibraryDetailRouteDocument, {
    variables: { Id: audiobookData?.audiobook.libraryId ?? "" },
    skip: !audiobookData?.audiobook.libraryId,
    fetchPolicy: "cache-and-network",
  });

  const library = libraryData?.Library ?? previousLibraryData?.Library ?? null;
  const error = !isLoading && !audiobookData ? "Audiobook not found" : null;

  const fetchAudiobook = useCallback(() => {
    void refetchAudiobook();
    if (audiobookData?.audiobook.libraryId) {
      void refetchLibrary();
    }
  }, [audiobookData?.audiobook.libraryId, refetchAudiobook, refetchLibrary]);

  const [deleteAudiobook, { loading: isDeleting }] = useMutation(
    DeleteAudiobookRouteDocument,
  );
  const [setChaptersWanted] = useMutation(
    AudiobookDetailSetChapterWantedDocument,
  );

  // Update page title
  useEffect(() => {
    if (audiobookData) {
      document.title = `Librarian - ${audiobookData.audiobook.title}`;
    }
    return () => {
      document.title = "Librarian";
    };
  }, [audiobookData]);

  // Keep data fresh with periodic updates and torrent completion events
  // This ensures download progress is updated in real-time
  useDataReactivity(fetchAudiobook, {
    onTorrentComplete: true,
    periodicInterval: false,
    onFocus: false,
  });

  // Handle delete
  const handleDelete = useCallback(async () => {
    if (!audiobookData) return;
    try {
      const result = await deleteAudiobook({ variables: { Id: audiobookId } });

      if (result.data?.DeleteAudiobook?.Success) {
        addToast({
          title: "Audiobook deleted",
          description: `${audiobookData.audiobook.title} has been removed.`,
          color: "success",
        });
        navigate({
          to: "/libraries/$libraryId",
          params: { libraryId: audiobookData.audiobook.libraryId },
        });
      } else {
        addToast({
          title: "Delete failed",
          description:
            result.data?.DeleteAudiobook?.Error || "Failed to delete audiobook",
          color: "danger",
        });
      }
    } catch (e) {
      addToast({
        title: "Error",
        description: sanitizeError(e),
        color: "danger",
      });
    } finally {
      onDeleteClose();
    }
  }, [audiobookData, audiobookId, deleteAudiobook, navigate, onDeleteClose]);

  // Navigate to sources page for this audiobook
  const handleManualHunt = useCallback(() => {
    navigate({ to: "/settings/sources" });
  }, [navigate]);

  // Handle play chapter - start playback with the audio player
  const handlePlayChapter = useCallback(
    (chapter: AudiobookChapter) => {
      if (chapter.mediaFileId && audiobookData) {
        // Get all chapters that have media files for the queue
        const allChapters = audiobookData.chapters.filter(
          (ch) => ch.mediaFileId,
        );
        startAudiobookPlayback(audiobookData.audiobook, chapter, allChapters);
      }
    },
    [audiobookData, startAudiobookPlayback],
  );

  // Note: Playing state and pause handling are now inside ChapterTable component directly from context

  // Navigate to sources page for a specific chapter search
  const handleSearchChapter = useCallback(
    (_chapter: AudiobookChapter) => {
      navigate({ to: "/settings/sources" });
    },
    [navigate],
  );

  const handleSetWantedForAllChapters = useCallback(
    async (wanted: boolean) => {
      if (!audiobookData || audiobookData.chapters.length === 0) {
        addToast({
          title: "No chapters",
          description: "No chapters found to update",
          color: "warning",
        });
        return;
      }

      try {
        const { data } = await setChaptersWanted({
          variables: { AudiobookId: audiobookData.audiobook.id, Wanted: wanted },
        });
        if (!data?.UpdateChapters?.success) {
          addToast({
            title: "Error",
            description:
              data?.UpdateChapters?.error ||
              "Failed to update wanted status for chapters",
            color: "danger",
          });
          return;
        }

        addToast({
          title: wanted ? "Marked as wanted" : "Removed wanted",
          description: wanted
            ? `${data.UpdateChapters.affectedCount} chapters marked as wanted`
            : `${data.UpdateChapters.affectedCount} chapters removed from wanted`,
          color: "success",
        });

        fetchAudiobook();
      } catch (err) {
        console.error("Failed to update chapter wanted state:", err);
        addToast({
          title: "Error",
          description: "Failed to update wanted status",
          color: "danger",
        });
      }
    },
    [audiobookData, fetchAudiobook, setChaptersWanted],
  );

  const playableChapters = useMemo(
    () => (audiobookData ? audiobookData.chapters.filter((ch) => !!ch.mediaFileId) : []),
    [audiobookData],
  );
  const handleOpenProperties = useCallback(() => {
    const firstPlayable = playableChapters[0];
    if (!firstPlayable?.mediaFileId) return;
    setPropertiesMediaFileId(firstPlayable.mediaFileId);
    onPropertiesOpen();
  }, [onPropertiesOpen, playableChapters]);

  if (!audiobookData) {
    if (isLoading) {
      return (
        <div className="container mx-auto p-4">
          <Card>
            <CardBody className="flex flex-col items-center justify-center py-12 text-center">
              <IconBook size={48} className="mx-auto mb-4 text-default-400" />
              <p className="text-default-500">Loading audiobook...</p>
            </CardBody>
          </Card>
        </div>
      );
    }

    return (
      <div className="container mx-auto p-4">
        <Card>
          <CardBody className="text-center py-12">
            <IconBook size={48} className="mx-auto mb-4 text-default-400" />
            <h2 className="text-xl font-semibold mb-2">Audiobook Not Found</h2>
            <p className="text-default-500 mb-4">
              {error || "The audiobook could not be loaded."}
            </p>
            <Button as={Link} to="/libraries" variant="flat">
              Back to Libraries
            </Button>
          </CardBody>
        </Card>
      </div>
    );
  }

  const { audiobook, chapters, author } = audiobookData;
  const isThisAudiobookPlaying =
    session?.audiobookId === audiobookId && !!session?.isPlaying;

  return (
    <div className="container mx-auto p-4  mb-20">
      {/* Breadcrumbs */}
      <Breadcrumbs className="mb-4">
        <BreadcrumbItem>
          <Link to="/libraries">Libraries</Link>
        </BreadcrumbItem>
        {library ? (
          <BreadcrumbItem>
            <Link to="/libraries/$libraryId" params={{ libraryId: library.Id }}>
              {library.Name}
            </Link>
          </BreadcrumbItem>
        ) : (
          <BreadcrumbItem>Library</BreadcrumbItem>
        )}
        <BreadcrumbItem>{audiobook.title}</BreadcrumbItem>
      </Breadcrumbs>

      {/* Audiobook Header */}
      <div className="flex flex-col md:flex-row gap-6 mb-6">
        {/* Cover Art */}
        <div className="w-64 shrink-0 relative group">
          {audiobook.coverUrl ? (
            <Image
              src={audiobook.coverUrl}
              alt={audiobook.title}
              classNames={{
                wrapper: "w-64 h-64",
                img: "w-full h-full object-cover rounded-lg",
              }}
            />
          ) : (
            <div className="w-64 h-64 bg-content2 rounded-lg flex items-center justify-center">
              <IconBook size={64} className="text-default-400" />
            </div>
          )}
          {playableChapters.length > 0 && (
            <button
              onClick={() => {
                if (isThisAudiobookPlaying) {
                  updatePlayback({ isPlaying: false });
                  return;
                }
                if (!audiobookData) return;
                startAudiobookPlayback(
                  audiobookData.audiobook,
                  playableChapters[0],
                  playableChapters,
                );
              }}
              className="absolute inset-0 z-10 flex items-center justify-center bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity duration-200 rounded-lg cursor-pointer"
              aria-label={
                isThisAudiobookPlaying ? "Pause Audiobook" : "Play Audiobook"
              }
            >
              <div
                className={`w-16 h-16 rounded-full ${isThisAudiobookPlaying ? "bg-warning" : "bg-primary"} flex items-center justify-center shadow-lg hover:scale-110 transition-transform`}
              >
                {isThisAudiobookPlaying ? (
                  <IconPlayerPause size={32} className="text-white" />
                ) : (
                  <IconPlayerPlay size={32} className="text-white ml-1" />
                )}
              </div>
            </button>
          )}
        </div>

        {/* Audiobook Info */}
        <div className="flex-1">
          <div className="flex items-start justify-between gap-4 mb-1">
            <h1 className="text-3xl font-bold">{audiobook.title}</h1>
            <Dropdown>
              <DropdownTrigger>
                <Button
                  isIconOnly
                  size="sm"
                  variant="light"
                  aria-label="Audiobook actions"
                >
                  <IconDotsVertical size={18} />
                </Button>
              </DropdownTrigger>
              <DropdownMenu
                aria-label="Audiobook actions menu"
                onAction={(key) => {
                if (key === "search") {
                  handleManualHunt();
                } else if (key === "refresh") {
                    void fetchAudiobook();
                  } else if (key === "wanted-on") {
                    void handleSetWantedForAllChapters(true);
                  } else if (key === "wanted-off") {
                    void handleSetWantedForAllChapters(false);
                  } else if (key === "properties") {
                    handleOpenProperties();
                } else if (key === "delete") {
                  onDeleteOpen();
                }
              }}
              >
                <DropdownItem
                  key="search"
                  startContent={<IconSearch size={16} />}
                >
                  Search for Audiobook
                </DropdownItem>
                <DropdownItem
                  key="refresh"
                  startContent={<IconRefresh size={16} />}
                >
                  Refresh
                </DropdownItem>
                <DropdownItem
                  key="wanted-on"
                  startContent={<IconCheck size={16} />}
                  isDisabled={
                    chapters.length === 0 || chapters.every((ch) => ch.wanted)
                  }
                >
                  Mark as Wanted
                </DropdownItem>
                <DropdownItem
                  key="wanted-off"
                  startContent={<IconX size={16} />}
                  isDisabled={
                    chapters.length === 0 ||
                    chapters.every((ch) => !ch.wanted)
                  }
                >
                  Remove as Wanted
                </DropdownItem>
                {playableChapters.length > 0 ? (
                  <DropdownItem
                    key="properties"
                    startContent={<IconInfoCircle size={16} />}
                  >
                    Properties
                  </DropdownItem>
                ) : null}
                <DropdownItem
                  key="delete"
                  startContent={
                    <IconTrash size={16} className="text-red-400" />
                  }
                  className="text-danger"
                  color="danger"
                >
                  Delete
                </DropdownItem>
              </DropdownMenu>
            </Dropdown>
          </div>
          {audiobook.subtitle && (
            <p className="text-lg text-default-500 mb-2">
              {audiobook.subtitle}
            </p>
          )}

          {/* Author */}
          {author && (
            <div className="flex items-center gap-2 text-default-600 mb-3">
              <IconUser size={16} />
              <span>by {author.name}</span>
            </div>
          )}

          {/* Tags */}
          <div className="flex flex-wrap items-center gap-2 mb-4">
            {audiobook.seriesName && (
              <Chip size="sm" variant="flat" color="secondary">
                {audiobook.seriesName}
              </Chip>
            )}
            {audiobook.language && (
              <Chip size="sm" variant="flat">
                {audiobook.language.toUpperCase()}
              </Chip>
            )}
            {audiobook.publisher && (
              <Chip size="sm" variant="flat">
                {audiobook.publisher}
              </Chip>
            )}
          </div>

          {/* Description */}
          {audiobook.description && (
            <p className="text-default-600 mb-4 line-clamp-3">
              {audiobook.description}
            </p>
          )}

          {/* Narrators */}
          {audiobook.narrators && audiobook.narrators.length > 0 && (
            <div className="mb-4">
              <p className="text-sm text-default-400">
                Narrated by:{" "}
                <span className="text-default-600">
                  {audiobook.narrators.join(", ")}
                </span>
              </p>
            </div>
          )}

          {/* Completion Progress */}
          <div className="mb-4">
            <div className="flex items-center justify-between text-sm mb-1">
              <span className="text-default-500">
                {audiobookData.chaptersWithFiles} of{" "}
                {audiobookData.chapterCount} chapters
              </span>
              <span className="font-medium">
                {audiobookData.completionPercent.toFixed(0)}%
              </span>
            </div>
            <Progress
              aria-label="Audiobook completion"
              value={audiobookData.completionPercent}
              color={
                audiobookData.completionPercent === 100 ? "success" : "primary"
              }
              size="sm"
            />
            {audiobookData.missingChapters > 0 && (
              <p className="text-sm text-warning mt-1">
                {audiobookData.missingChapters} chapter
                {audiobookData.missingChapters !== 1 ? "s" : ""} missing
              </p>
            )}
          </div>

          {/* Stats */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
            <div>
              <p className="text-xs text-default-400">Chapters</p>
              <p className="text-lg font-semibold">
                {audiobookData.chapterCount}
              </p>
            </div>
            <div>
              <p className="text-xs text-default-400">Duration</p>
              <p className="text-lg font-semibold">
                {audiobook.durationSecs
                  ? formatDuration(audiobook.durationSecs)
                  : "-"}
              </p>
            </div>
            <div>
              <p className="text-xs text-default-400">Size</p>
              <p className="text-lg font-semibold">
                {audiobook.sizeBytes ? formatBytes(audiobook.sizeBytes) : "-"}
              </p>
            </div>
            <div>
              <p className="text-xs text-default-400">ISBN</p>
              <p className="text-lg font-semibold">{audiobook.isbn || "-"}</p>
            </div>
          </div>
        </div>
      </div>

      <ChapterTable
        chapters={chapters}
        audiobookId={audiobookData.audiobook.id}
        onPlay={handlePlayChapter}
        onSearch={handleSearchChapter}
        fetchAudiobook={fetchAudiobook}
      />

      {/* Delete Confirmation Modal */}
      {audiobookData && (
        <Modal isOpen={isDeleteOpen} onClose={onDeleteClose}>
          <ModalContent>
            <ModalHeader>Delete Audiobook</ModalHeader>
            <ModalBody>
              <p>
                Are you sure you want to delete{" "}
                <strong>{audiobookData.audiobook.title}</strong>?
              </p>
              <p className="text-sm text-default-500 mt-2">
                This will remove the audiobook from the library. Associated
                files will not be deleted.
              </p>
            </ModalBody>
            <ModalFooter>
              <Button variant="flat" onPress={onDeleteClose}>
                Cancel
              </Button>
              <Button
                color="danger"
                onPress={handleDelete}
                isLoading={isDeleting}
              >
                Delete
              </Button>
            </ModalFooter>
          </ModalContent>
        </Modal>
      )}
      <FilePropertiesModal
        isOpen={isPropertiesOpen}
        onClose={() => {
          onPropertiesClose();
          setPropertiesMediaFileId(null);
        }}
        mediaFileId={propertiesMediaFileId}
        title={audiobook ? audiobook.title : undefined}
      />
    </div>
  );
}
