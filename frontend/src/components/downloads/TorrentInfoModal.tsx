import { useState, useEffect, useCallback, useMemo } from "react";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { Button } from "@heroui/button";
import {
  Dropdown,
  DropdownItem,
  DropdownMenu,
  DropdownTrigger,
} from "@heroui/dropdown";
import { Spinner } from "@heroui/spinner";
import { Progress } from "@heroui/progress";
import { Chip } from "@heroui/chip";
import { Card, CardBody } from "@heroui/card";
import { Tooltip } from "@heroui/tooltip";
import { addToast } from "@heroui/toast";
import {
  TORRENT_DETAILS_QUERY,
  REMOVE_MATCH_MUTATION,
  TORRENT_PROGRESS_SUBSCRIPTION,
  TORRENT_FILE_CHANGED_SUBSCRIPTION,
  type TorrentDetails,
  type TorrentFileInfo,
  type PendingFileMatch,
  type RemoveMatchResult,
} from "../../lib/graphql";
import { formatBytes, sanitizeError } from "../../lib/format";
import { TORRENT_STATE_INFO } from "./TorrentCard";
import { MediaFilesMatchDialog } from "./MediaFilesMatchDialog";
import { DataTable, type DataTableColumn } from "../data-table";
import { ErrorState } from "../shared";
import { FilePropertiesModal } from "../FilePropertiesModal";
import {
  IconCheck,
  IconArrowDown,
  IconArrowUp,
  IconFolder,
  IconLink,
  IconX,
  IconTrash,
  IconCopy,
  IconBolt,
  IconUsers,
  IconDotsVertical,
  IconInfoCircle,
} from "@tabler/icons-react";
import { getFileIcon } from "../../lib/fileIcons";
import {
  AnalyzeMediaFileForTorrentDocument,
  CreateUnmatchedMediaFileFromTorrentDocument,
  PendingFileMatchesBySourceDocument,
  TorrentModalMediaFilesByPathsDocument,
  TorrentByInfoHashWithFilesDocument,
  type AnalyzeMediaFileForTorrentMutation,
  type AnalyzeMediaFileForTorrentMutationVariables,
  type CreateUnmatchedMediaFileFromTorrentMutation,
  type CreateUnmatchedMediaFileFromTorrentMutationVariables,
  type PendingFileMatchesBySourceQuery,
  type TorrentModalMediaFilesByPathsQuery,
  type TorrentByInfoHashWithFilesQueryVariables,
  type TorrentByInfoHashWithFilesQuery,
} from "../../lib/graphql/generated/graphql";
import {
  apolloClient,
  gql,
  useMutation,
  useQuery,
  useSubscription,
} from "../../lib/graphql/client";

interface TorrentInfoModalProps {
  /** Legacy numeric id (session handle). Prefer torrentInfoHash when using entity list. */
  torrentId?: number | null;
  /** Entity torrent info hash – fetches one Torrent by InfoHash and shows basic info. */
  torrentInfoHash?: string | null;
  isOpen: boolean;
  onClose: () => void;
}

type TorrentUnmatchMediaFileMutationData = {
  UnmatchMediaFile: {
    Success: boolean;
    Reason?: string | null;
  };
};

type TorrentUnmatchMediaFileMutationVariables = {
  MediaFileId: string;
};

const TORRENT_UNMATCH_MEDIA_FILE_MUTATION = gql(`
  mutation TorrentInfoUnmatchMediaFile($MediaFileId: String!) {
    UnmatchMediaFile(MediaFileId: $MediaFileId) {
      Success
      Reason
    }
  }
`);

const SUPPORTED_TORRENT_MEDIA_EXTENSIONS = new Set([
  "mkv",
  "mp4",
  "avi",
  "m4v",
  "mov",
  "wmv",
  "flv",
  "webm",
  "mpeg",
  "mpg",
  "ts",
  "m2ts",
  "mp3",
  "flac",
  "m4a",
  "m4b",
  "aac",
  "ogg",
  "opus",
  "wav",
  "wma",
  "aiff",
  "alac",
  "ape",
  "dsf",
  "dff",
  "srt",
  "ass",
  "ssa",
  "sub",
  "vtt",
  "ttml",
  "smi",
  "sami",
  "idx",
  "sup",
]);
const UNMATCHED_LIBRARY_ID = "__torrent_unmatched__";

function normalizePathForLookup(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+/g, "/").toLowerCase();
}

function isAbsolutePath(path: string): boolean {
  return (
    path.startsWith("/") ||
    path.startsWith("\\\\") ||
    /^[A-Za-z]:[\\/]/.test(path)
  );
}

function joinPath(base: string, segment: string): string {
  const normalizedBase = base.replace(/\\/g, "/").replace(/\/+$/, "");
  const normalizedSegment = segment.replace(/\\/g, "/").replace(/^\/+/, "");
  return `${normalizedBase}/${normalizedSegment}`;
}

function buildPathCandidates(
  filePath: string,
  savePath?: string | null,
  torrentName?: string | null,
): string[] {
  const candidates = new Set<string>();
  const normalizedFilePath = filePath.replace(/\\/g, "/");
  candidates.add(filePath);
  candidates.add(normalizedFilePath);

  if (savePath && !isAbsolutePath(normalizedFilePath)) {
    candidates.add(joinPath(savePath, normalizedFilePath));
    if (torrentName) {
      candidates.add(joinPath(joinPath(savePath, torrentName), normalizedFilePath));
    }
  }

  return Array.from(candidates);
}

function getBestFilePath(
  filePath: string,
  savePath?: string | null,
  torrentName?: string | null,
): string {
  if (isAbsolutePath(filePath)) {
    return filePath.replace(/\\/g, "/");
  }
  const candidates = buildPathCandidates(filePath, savePath, torrentName);
  return candidates[0] ?? filePath;
}

function getRelativePath(
  filePath: string,
  savePath?: string | null,
  torrentName?: string | null,
): string {
  const normalized = filePath.replace(/\\/g, "/");
  if (!isAbsolutePath(normalized)) {
    return normalized;
  }
  if (!savePath) {
    return normalized.split("/").pop() ?? normalized;
  }

  const savePrefix = `${savePath.replace(/\\/g, "/").replace(/\/+$/, "")}/`;
  if (torrentName) {
    const withTorrentPrefix = `${savePrefix}${torrentName}/`;
    if (normalized.startsWith(withTorrentPrefix)) {
      return normalized.slice(withTorrentPrefix.length);
    }
  }
  if (normalized.startsWith(savePrefix)) {
    return normalized.slice(savePrefix.length);
  }
  return normalized.split("/").pop() ?? normalized;
}

function hasFfprobeData(metadata: string | null | undefined): boolean {
  if (!metadata) return false;
  const trimmed = metadata.trim();
  if (!trimmed) return false;
  return trimmed !== "{}" && trimmed !== "null";
}

function isSupportedTorrentMediaPath(path: string): boolean {
  const fileName = path.split("/").pop() ?? path;
  const extension = fileName.includes(".")
    ? fileName.split(".").pop()?.toLowerCase()
    : undefined;
  return Boolean(extension && SUPPORTED_TORRENT_MEDIA_EXTENSIONS.has(extension));
}

function FileProgressBar({
  progress,
  ariaLabel,
}: {
  progress: number;
  ariaLabel: string;
}) {
  const percent = Math.max(0, Math.min(100, progress * 100));
  const labelClass =
    percent >= 18
      ? "text-white/90"
      : percent >= 1
        ? "text-default-600"
        : "text-default-400";

  return (
    <div className="relative w-full min-w-[120px]">
      <Progress
        value={percent}
        size="sm"
        color={progress >= 1 ? "success" : "primary"}
        aria-label={ariaLabel}
        classNames={{ track: "h-4", indicator: "h-4" }}
      />
      <div
        className={`absolute inset-0 flex items-center justify-center text-[10px] font-semibold tabular-nums ${labelClass}`}
      >
        {percent.toFixed(0)}%
      </div>
    </div>
  );
}

interface FileActionContext {
  mediaFileId: string | null;
  hasAnalyzedMedia: boolean;
  canProcess: boolean;
  processFileKey: string;
  isProcessing: boolean;
  onOpenProperties: () => void;
  onProcess: () => void;
  matchLabel: "Match" | "Rematch";
  onOpenMatch: () => void;
  canUnmatch: boolean;
  onUnmatch?: () => void;
  removeMatchId?: string | null;
  onRemoveMatch?: (matchId: string) => void;
}

function FileActionsMenu({
  actionContext,
}: {
  actionContext: FileActionContext;
}) {
  const {
    mediaFileId,
    hasAnalyzedMedia,
    canProcess,
    processFileKey,
    isProcessing,
    onOpenProperties,
    onProcess,
    matchLabel,
    onOpenMatch,
    canUnmatch,
    onUnmatch,
    removeMatchId,
    onRemoveMatch,
  } = actionContext;

  return (
    <Dropdown placement="bottom-end">
      <DropdownTrigger>
        <Button isIconOnly size="sm" variant="light">
          <IconDotsVertical size={14} />
        </Button>
      </DropdownTrigger>
      <DropdownMenu aria-label={`Actions for ${processFileKey}`}>
        <DropdownItem
          key="properties"
          startContent={<IconInfoCircle size={14} />}
          isDisabled={!mediaFileId && !canProcess}
          onPress={onOpenProperties}
        >
          Properties
        </DropdownItem>
        {canProcess && !hasAnalyzedMedia ? (
          <DropdownItem
            key="process"
            startContent={<IconBolt size={14} />}
            isDisabled={isProcessing}
            onPress={onProcess}
          >
            {isProcessing ? "Processing..." : "Process file"}
          </DropdownItem>
        ) : null}
        <DropdownItem
          key="match"
          startContent={<IconLink size={14} />}
          onPress={onOpenMatch}
        >
          {matchLabel}
        </DropdownItem>
        {canUnmatch && onUnmatch ? (
          <DropdownItem
            key="unmatch"
            className="text-warning"
            color="warning"
            startContent={<IconX size={14} />}
            onPress={onUnmatch}
          >
            Unmatch
          </DropdownItem>
        ) : null}
        {removeMatchId && onRemoveMatch ? (
          <DropdownItem
            key="remove-match"
            className="text-danger"
            color="danger"
            startContent={<IconTrash size={14} />}
            onPress={() => onRemoveMatch(removeMatchId)}
          >
            Remove match
          </DropdownItem>
        ) : null}
      </DropdownMenu>
    </Dropdown>
  );
}

// Helper to create file columns with match info
function createFileColumns(
  matchesByIndex: Map<number, PendingFileMatch>,
  getFileActionContext: (file: TorrentFileInfo) => FileActionContext,
  onRemoveMatch?: (matchId: string) => void,
): DataTableColumn<TorrentFileInfo>[] {
  return [
    {
      key: "match",
      label: "Match",
      width: 100,
      align: "center",
      render: (file) => {
        const match = matchesByIndex.get(file.index);
        if (!match) {
          return (
            <Tooltip content="No match - file not linked to library">
              <Chip size="sm" color="default" variant="flat">
                Unmatched
              </Chip>
            </Tooltip>
          );
        }
        const matchType = match.episodeId
          ? "Episode"
          : match.movieId
            ? "Movie"
            : match.trackId
              ? "Track"
              : match.chapterId
                ? "Chapter"
                : "None";
        if (matchType === "None") {
          return (
            <Tooltip content="No library item matched">
              <Chip
                size="sm"
                color="warning"
                variant="flat"
                startContent={<IconX size={12} />}
              >
                None
              </Chip>
            </Tooltip>
          );
        }
        return (
          <Tooltip content={`Matched to ${matchType}`}>
            <Chip
              size="sm"
              color="success"
              variant="flat"
              startContent={<IconLink size={12} />}
            >
              {matchType}
            </Chip>
          </Tooltip>
        );
      },
    },
    {
      key: "status",
      label: "Status",
      width: 90,
      align: "center",
      render: (file) => {
        const match = matchesByIndex.get(file.index);
        if (!match) {
          return <span className="text-default-400">-</span>;
        }
        if (match.copied) {
          return (
            <Tooltip
              content={`Copied ${match.copiedAt ? new Date(match.copiedAt).toLocaleString() : ""}`}
            >
              <Chip
                size="sm"
                color="success"
                variant="flat"
                startContent={<IconCopy size={12} />}
              >
                Copied
              </Chip>
            </Tooltip>
          );
        }
        if (match.copyError) {
          return (
            <Tooltip content={match.copyError}>
              <Chip
                size="sm"
                color="danger"
                variant="flat"
                startContent={<IconX size={12} />}
              >
                Error
              </Chip>
            </Tooltip>
          );
        }
        return (
          <Tooltip content="File will be copied when download completes">
            <Chip size="sm" color="warning" variant="flat">
              Pending
            </Chip>
          </Tooltip>
        );
      },
    },
    {
      key: "path",
      label: "File",
      render: (file) => {
        const fileName = file.path.split("/").pop() || file.path;
        const directory = file.path.includes("/")
          ? file.path.substring(0, file.path.lastIndexOf("/"))
          : null;
        const match = matchesByIndex.get(file.index);
        return (
          <div className="flex items-start gap-2 min-w-0">
            <div className="mt-0.5 flex-shrink-0">
              {getFileIcon(file.path, false, { size: 18 })}
            </div>
            <div className="min-w-0 h-10">
              <Tooltip content={file.path} delay={500}>
                <div className="truncate font-medium text-sm max-w-xs lg:max-w-md">
                  {fileName}
                </div>
              </Tooltip>
              {directory && (
                <div className="text-xs text-default-400 truncate max-w-xs lg:max-w-md">
                  {directory}
                </div>
              )}
              {match?.parsedResolution && (
                <div className="text-xs text-default-500 mt-0.5">
                  {[match.parsedResolution, match.parsedCodec]
                    .filter(Boolean)
                    .join(" ")}
                </div>
              )}
            </div>
          </div>
        );
      },
    },
    {
      key: "size",
      label: "Size",
      width: 100,
      align: "end",
      render: (file) => (
        <span className="text-sm tabular-nums text-default-500">
          {formatBytes(file.size)}
        </span>
      ),
      sortFn: (a, b) => a.size - b.size,
    },
    {
      key: "progress",
      label: "Progress",
      width: 180,
      align: "start",
      render: (file) => (
        <FileProgressBar
          progress={file.progress}
          ariaLabel={`${file.path} progress`}
        />
      ),
      sortFn: (a, b) => a.progress - b.progress,
    },
    {
      key: "actions",
      label: "",
      width: 72,
      align: "center",
      render: (file) => {
        const match = matchesByIndex.get(file.index);
        return (
          <FileActionsMenu
            actionContext={{
              ...getFileActionContext(file),
              removeMatchId: match?.id ?? null,
              onRemoveMatch,
            }}
          />
        );
      },
    },
  ];
}

export function TorrentInfoModal({
  torrentId,
  torrentInfoHash,
  isOpen,
  onClose,
}: TorrentInfoModalProps) {
  type EntityTorrentNode =
    TorrentByInfoHashWithFilesQuery["Torrents"]["Edges"][number]["Node"];
  type LegacyTorrentDetailsQuery = { torrentDetails: TorrentDetails | null };
  type LegacyTorrentDetailsQueryVariables = { id: number };
  type RemoveMatchMutationData = { removeMatch: RemoveMatchResult };

  const [entityLiveStats, setEntityLiveStats] = useState<{
    downloadSpeed: number;
    uploadSpeed: number;
    peers: number;
  } | null>(null);
  const [removedMatchIds, setRemovedMatchIds] = useState<Set<string>>(
    () => new Set(),
  );
  const [propertiesMediaFileId, setPropertiesMediaFileId] = useState<string | null>(
    null,
  );
  const [processingFileKey, setProcessingFileKey] = useState<string | null>(null);
  const [isMatchDialogOpen, setIsMatchDialogOpen] = useState(false);
  const [matchFileIndex, setMatchFileIndex] = useState<number | null>(null);

  const isEntityMode = Boolean(torrentInfoHash);
  const entityTorrentQueryVariables = useMemo<TorrentByInfoHashWithFilesQueryVariables>(
    () => ({
      Where: { InfoHash: { Eq: torrentInfoHash ?? "" } },
      Page: { Limit: 1, Offset: 0 },
    }),
    [torrentInfoHash],
  );

  const {
    data: entityData,
    previousData: previousEntityData,
    loading: entityLoading,
    error: entityQueryError,
  } = useQuery(TorrentByInfoHashWithFilesDocument, {
    variables: entityTorrentQueryVariables,
    skip: !isOpen || !torrentInfoHash,
    fetchPolicy: "cache-and-network",
    notifyOnNetworkStatusChange: true,
  });

  const entityTorrent = useMemo<EntityTorrentNode | null>(() => {
    const edges =
      entityData?.Torrents?.Edges ?? previousEntityData?.Torrents?.Edges ?? [];
    return edges[0]?.Node ?? null;
  }, [entityData?.Torrents?.Edges, previousEntityData?.Torrents?.Edges]);

  const {
    data: legacyData,
    previousData: previousLegacyData,
    loading: legacyLoading,
    error: legacyQueryError,
    refetch: refetchLegacyDetails,
  } = useQuery<LegacyTorrentDetailsQuery, LegacyTorrentDetailsQueryVariables>(
    gql(TORRENT_DETAILS_QUERY),
    {
      variables: { id: torrentId as number },
      skip: !isOpen || Boolean(torrentInfoHash) || torrentId == null,
      fetchPolicy: "cache-and-network",
      notifyOnNetworkStatusChange: true,
    },
  );

  const details =
    legacyData?.torrentDetails ?? previousLegacyData?.torrentDetails ?? null;

  const {
    data: fileMatchesData,
    previousData: previousFileMatchesData,
    refetch: refetchFileMatches,
  } = useQuery(PendingFileMatchesBySourceDocument, {
    variables: {
      Where: {
        SourceType: { Eq: "torrent" },
        SourceId: { Eq: details?.infoHash ?? "" },
      },
      Page: { Limit: 500, Offset: 0 },
    },
    skip: !isOpen || !details?.infoHash,
    fetchPolicy: "cache-and-network",
    notifyOnNetworkStatusChange: true,
  });

  const [removeMatchMutation] = useMutation<
    RemoveMatchMutationData,
    { matchId: string }
  >(gql(REMOVE_MATCH_MUTATION));
  const [createUnmatchedMediaFile] = useMutation<
    CreateUnmatchedMediaFileFromTorrentMutation,
    CreateUnmatchedMediaFileFromTorrentMutationVariables
  >(CreateUnmatchedMediaFileFromTorrentDocument);
  const [analyzeMediaFile] = useMutation<
    AnalyzeMediaFileForTorrentMutation,
    AnalyzeMediaFileForTorrentMutationVariables
  >(AnalyzeMediaFileForTorrentDocument);
  const [unmatchMediaFile] = useMutation<
    TorrentUnmatchMediaFileMutationData,
    TorrentUnmatchMediaFileMutationVariables
  >(TORRENT_UNMATCH_MEDIA_FILE_MUTATION);

  const fileMatches = useMemo<PendingFileMatch[]>(() => {
    const edges =
      fileMatchesData?.PendingFileMatches?.Edges ??
      previousFileMatchesData?.PendingFileMatches?.Edges ??
      [];
    return edges
      .map(
        (
          edge: PendingFileMatchesBySourceQuery["PendingFileMatches"]["Edges"][number],
        ) => ({
          id: edge.Node.Id,
          sourceType: edge.Node.SourceType,
          sourceId: edge.Node.SourceId ?? null,
          sourceFileIndex: edge.Node.SourceFileIndex ?? null,
          sourcePath: edge.Node.SourcePath,
          fileSize: edge.Node.FileSize,
          episodeId: edge.Node.EpisodeId ?? null,
          movieId: edge.Node.MovieId ?? null,
          trackId: edge.Node.TrackId ?? null,
          chapterId: edge.Node.ChapterId ?? null,
          matchType: (
            edge.Node.MatchType === "manual" ? "manual" : "auto"
          ) as PendingFileMatch["matchType"],
          matchConfidence: edge.Node.MatchConfidence ?? null,
          parsedResolution: edge.Node.ParsedResolution ?? null,
          parsedCodec: edge.Node.ParsedCodec ?? null,
          parsedSource: edge.Node.ParsedSource ?? null,
          parsedAudio: edge.Node.ParsedAudio ?? null,
          copied: Boolean(edge.Node.CopiedAt && !edge.Node.CopyError),
          copiedAt: edge.Node.CopiedAt ?? null,
          copyError: edge.Node.CopyError ?? null,
          createdAt: "",
        }),
      )
      .filter((match) => !removedMatchIds.has(match.id));
  }, [
    fileMatchesData?.PendingFileMatches?.Edges,
    previousFileMatchesData?.PendingFileMatches?.Edges,
    removedMatchIds,
  ]);

  // Handle removing a match
  const handleRemoveMatch = useCallback(async (matchId: string) => {
    const result = await removeMatchMutation({
      variables: { matchId },
    });
    if (result.data?.removeMatch.success) {
      setRemovedMatchIds((prev) => new Set(prev).add(matchId));
      void refetchFileMatches();
      addToast({
        title: "Match Removed",
        description: "The file match has been removed",
        color: "success",
      });
    } else {
      addToast({
        title: "Error",
        description: result.data?.removeMatch.error || "Failed to remove match",
        color: "danger",
      });
    }
  }, [removeMatchMutation, refetchFileMatches]);

  // Create a map of file index to match for quick lookup
  const matchesByIndex = useMemo(
    () =>
      new Map(
        fileMatches
          .filter((m) => m.sourceFileIndex !== null)
          .map((m) => [m.sourceFileIndex as number, m]),
      ),
    [fileMatches],
  );

  const currentSavePath = details?.savePath ?? entityTorrent?.SavePath ?? null;
  const currentTorrentName = details?.name ?? entityTorrent?.Name ?? null;

  const visibleFileRows = useMemo(
    () =>
      isEntityMode
        ? (entityTorrent?.Files?.Edges?.map((e) => e.Node) ?? []).map((file) => ({
            key: `entity-${file.FileIndex}`,
            filePath: file.FilePath,
          }))
        : (details?.files ?? []).map((file) => ({
            key: `legacy-${file.index}`,
            filePath: file.path,
          })),
    [isEntityMode, entityTorrent?.Files?.Edges, details?.files],
  );

  const mediaLookupPaths = useMemo(() => {
    const candidates = new Set<string>();
    for (const row of visibleFileRows) {
      for (const candidate of buildPathCandidates(
        row.filePath,
        currentSavePath,
        currentTorrentName,
      )) {
        candidates.add(candidate);
      }
    }
    return Array.from(candidates);
  }, [visibleFileRows, currentSavePath, currentTorrentName]);

  const {
    data: mediaByPathData,
    previousData: previousMediaByPathData,
    refetch: refetchMediaByPath,
  } = useQuery(TorrentModalMediaFilesByPathsDocument, {
    variables: { Paths: mediaLookupPaths },
    skip: !isOpen || mediaLookupPaths.length === 0,
    fetchPolicy: "cache-and-network",
    notifyOnNetworkStatusChange: true,
  });

  type MediaLookupNode =
    TorrentModalMediaFilesByPathsQuery["MediaFiles"]["Edges"][number]["Node"] & {
      EpisodeId?: string | null;
      MovieId?: string | null;
      TrackId?: string | null;
      ChapterId?: string | null;
    };

  const mediaByNormalizedPath = useMemo(() => {
    const map = new Map<string, MediaLookupNode>();
    const edges =
      mediaByPathData?.MediaFiles?.Edges ??
      previousMediaByPathData?.MediaFiles?.Edges ??
      [];
    for (const edge of edges) {
      map.set(normalizePathForLookup(edge.Node.Path), edge.Node);
    }
    return map;
  }, [mediaByPathData?.MediaFiles?.Edges, previousMediaByPathData?.MediaFiles?.Edges]);

  const resolveMediaForFile = useCallback(
    (filePath: string) => {
      const candidates = buildPathCandidates(filePath, currentSavePath, currentTorrentName);
      for (const candidate of candidates) {
        const media = mediaByNormalizedPath.get(normalizePathForLookup(candidate));
        if (media) {
          return media;
        }
      }
      return null;
    },
    [currentSavePath, currentTorrentName, mediaByNormalizedPath],
  );

  const handleProcessFile = useCallback(
    async (params: {
      filePath: string;
      fileSize: number;
      processFileKey: string;
    }) => {
      const { filePath, fileSize, processFileKey } = params;
      const currentMedia = resolveMediaForFile(filePath);
      if (currentMedia && hasFfprobeData(currentMedia.Metadata)) {
        return;
      }
      if (!isSupportedTorrentMediaPath(filePath)) {
        addToast({
          title: "Unsupported file type",
          description: "This file type is not processed for media analysis.",
          color: "warning",
        });
        return;
      }

      setProcessingFileKey(processFileKey);
      try {
        let mediaFileId = currentMedia?.Id ?? null;
        let analyzePath = currentMedia?.Path ?? null;

        if (!mediaFileId) {
          const bestPath = getBestFilePath(filePath, currentSavePath, currentTorrentName);
          const relativePath = getRelativePath(bestPath, currentSavePath, currentTorrentName);
          const originalName = bestPath.split("/").pop() ?? bestPath;

          const createResult = await createUnmatchedMediaFile({
            variables: {
              Input: {
                AddedAt: new Date().toISOString(),
                IsHdr: false,
                LibraryId: UNMATCHED_LIBRARY_ID,
                Metadata: JSON.stringify({
                  SourceType: "torrent",
                  UnmatchedReason: "Manually processed from torrent modal",
                }),
                OriginalName: originalName,
                Path: bestPath,
                RelativePath: relativePath,
                Size: Math.max(0, Math.floor(fileSize)),
              },
            },
          });

          const createData = createResult.data?.CreateMediaFile;
          if (!createData?.Success || !createData.MediaFile?.Id) {
            throw new Error(createData?.Error || "Failed to create media file");
          }

          mediaFileId = createData.MediaFile.Id;
          analyzePath = createData.MediaFile.Path;
        }

        if (!mediaFileId || !analyzePath) {
          throw new Error("Missing media file information for analysis");
        }

        const analyzeResult = await analyzeMediaFile({
          variables: {
            MediaFileId: mediaFileId,
            Path: analyzePath,
          },
        });
        const analyzeData = analyzeResult.data?.AnalyzeMediaFile;
        if (!analyzeData?.Success) {
          throw new Error(analyzeData?.Message || "Failed to queue media analysis");
        }

        addToast({
          title: "File queued",
          description: "Media analysis has been queued for this file.",
          color: "success",
        });
        void refetchMediaByPath();
      } catch (error) {
        addToast({
          title: "Processing failed",
          description: sanitizeError(error),
          color: "danger",
        });
      } finally {
        setProcessingFileKey((current) =>
          current === processFileKey ? null : current,
        );
      }
    },
    [
      resolveMediaForFile,
      currentSavePath,
      currentTorrentName,
      createUnmatchedMediaFile,
      analyzeMediaFile,
      refetchMediaByPath,
    ],
  );

  const handleOpenProperties = useCallback(
    async (filePath: string) => {
      const mediaFile = resolveMediaForFile(filePath);
      if (mediaFile?.Id) {
        setPropertiesMediaFileId(mediaFile.Id);
        return;
      }

      if (!isSupportedTorrentMediaPath(filePath)) {
        addToast({
          title: "No properties available",
          description: "This file type does not expose media properties.",
          color: "default",
        });
        return;
      }

      try {
        const result = await apolloClient.query({
          query: TorrentModalMediaFilesByPathsDocument,
          variables: {
            Paths: buildPathCandidates(filePath, currentSavePath, currentTorrentName),
          },
          fetchPolicy: "network-only",
        });
        const found = result.data?.MediaFiles?.Edges?.[0]?.Node;
        if (found?.Id) {
          setPropertiesMediaFileId(found.Id);
          void refetchMediaByPath();
          return;
        }

        addToast({
          title: "No media file record",
          description: "Process this file first to generate metadata.",
          color: "warning",
        });
      } catch (error) {
        addToast({
          title: "Failed to load properties",
          description: sanitizeError(error),
          color: "danger",
        });
      }
    },
    [
      resolveMediaForFile,
      currentSavePath,
      currentTorrentName,
      refetchMediaByPath,
    ],
  );

  const buildActionContext = useCallback(
    (
      filePath: string,
      fileSize: number,
      rowKey: string,
      fileIndex: number,
    ): FileActionContext => {
      const mediaFile = resolveMediaForFile(filePath);
      const analyzed = hasFfprobeData(mediaFile?.Metadata);
      const canProcess = isSupportedTorrentMediaPath(filePath);
      const existingMatchId =
        mediaFile?.EpisodeId ??
        mediaFile?.MovieId ??
        mediaFile?.TrackId ??
        mediaFile?.ChapterId ??
        null;
      const hasExistingMatch = Boolean(existingMatchId);

      return {
        mediaFileId: mediaFile?.Id ?? null,
        hasAnalyzedMedia: analyzed,
        canProcess,
        processFileKey: rowKey,
        isProcessing: processingFileKey === rowKey,
        matchLabel: hasExistingMatch ? "Rematch" : "Match",
        onOpenMatch: () => {
          setMatchFileIndex(fileIndex);
          setIsMatchDialogOpen(true);
        },
        canUnmatch: hasExistingMatch && Boolean(mediaFile?.Id),
        onUnmatch: mediaFile?.Id
          ? () => {
              void (async () => {
                try {
                  const result = await unmatchMediaFile({
                    variables: { MediaFileId: mediaFile.Id },
                  });
                  if (!result.data?.UnmatchMediaFile?.Success) {
                    addToast({
                      title: "Unmatch failed",
                      description:
                        result.data?.UnmatchMediaFile?.Reason ||
                        "Failed to unmatch media file",
                      color: "danger",
                    });
                    return;
                  }
                  addToast({
                    title: "File unmatched",
                    description: "The file has been unlinked from media.",
                    color: "success",
                  });
                  void refetchMediaByPath();
                } catch (error) {
                  addToast({
                    title: "Unmatch failed",
                    description: sanitizeError(error),
                    color: "danger",
                  });
                }
              })();
            }
          : undefined,
        onOpenProperties: () => {
          void handleOpenProperties(filePath);
        },
        onProcess: () => {
          void handleProcessFile({
            filePath,
            fileSize,
            processFileKey: rowKey,
          });
        },
      };
    },
    [
      resolveMediaForFile,
      processingFileKey,
      refetchMediaByPath,
      handleOpenProperties,
      handleProcessFile,
      unmatchMediaFile,
    ],
  );

  useEffect(() => {
    setEntityLiveStats(null);
    setRemovedMatchIds(new Set());
    setPropertiesMediaFileId(null);
    setProcessingFileKey(null);
    setIsMatchDialogOpen(false);
    setMatchFileIndex(null);
  }, [isOpen, torrentId, torrentInfoHash]);

  useSubscription<{
    TorrentFileChanged: {
      Action: "Created" | "Updated" | "Deleted";
      Id: string;
      TorrentFile?: {
        TorrentId: string;
        FileIndex: number;
        FilePath: string;
        FileSize: number;
        DownloadedBytes: number;
        Progress: number;
      };
    };
  }>(gql(TORRENT_FILE_CHANGED_SUBSCRIPTION), {
    skip: !isOpen || !torrentInfoHash || !entityTorrent?.Id,
    onData: ({ data }) => {
      const payload = data.data?.TorrentFileChanged;
      const torrentFile = payload?.TorrentFile;
      if (!torrentFile || torrentFile.TorrentId !== entityTorrent?.Id) {
        return;
      }
      apolloClient.cache.updateQuery<TorrentByInfoHashWithFilesQuery>(
        {
          query: TorrentByInfoHashWithFilesDocument,
          variables: entityTorrentQueryVariables,
        },
        (existing) => {
          if (!existing?.Torrents?.Edges?.length) {
            return existing;
          }
          const currentNode = existing.Torrents.Edges[0]?.Node;
          if (!currentNode || currentNode.Id !== torrentFile.TorrentId) {
            return existing;
          }

          const currentEdges = currentNode.Files?.Edges ?? [];
          const existingIndex = currentEdges.findIndex(
            (edge) => edge.Node.FileIndex === torrentFile.FileIndex,
          );

          let nextFileEdges = currentEdges;
          if (payload.Action === "Deleted") {
            if (existingIndex === -1) return existing;
            nextFileEdges = currentEdges.filter(
              (edge) => edge.Node.FileIndex !== torrentFile.FileIndex,
            );
          } else if (existingIndex >= 0) {
            nextFileEdges = [...currentEdges];
            nextFileEdges[existingIndex] = {
              ...nextFileEdges[existingIndex],
              Node: {
                ...nextFileEdges[existingIndex].Node,
                FileIndex: torrentFile.FileIndex,
                FilePath: torrentFile.FilePath,
                FileSize: torrentFile.FileSize,
                DownloadedBytes: torrentFile.DownloadedBytes,
                Progress: torrentFile.Progress,
              },
            };
          } else {
            nextFileEdges = [
              ...currentEdges,
              {
                Node: {
                  FileIndex: torrentFile.FileIndex,
                  FilePath: torrentFile.FilePath,
                  FileSize: torrentFile.FileSize,
                  DownloadedBytes: torrentFile.DownloadedBytes,
                  Progress: torrentFile.Progress,
                },
              },
            ];
          }

          const nextEdges = [...existing.Torrents.Edges];
          nextEdges[0] = {
            ...nextEdges[0],
            Node: {
              ...currentNode,
              Files: {
                ...currentNode.Files,
                Edges: nextFileEdges,
              },
            },
          };

          return {
            ...existing,
            Torrents: {
              ...existing.Torrents,
              Edges: nextEdges,
            },
          };
        },
      );
    },
  });

  useSubscription<{
    TorrentProgress: {
      Id: number;
      InfoHash: string;
      Progress: number;
      DownloadSpeed: number;
      UploadSpeed: number;
      Peers: number;
      State: string;
    };
  }>(gql(TORRENT_PROGRESS_SUBSCRIPTION), {
    skip: !isOpen || (!torrentInfoHash && torrentId == null),
    onData: ({ data }) => {
      const progress = data.data?.TorrentProgress;
      if (!progress) return;

      if (torrentInfoHash) {
        if (progress.InfoHash !== torrentInfoHash) return;
        setEntityLiveStats({
          downloadSpeed: progress.DownloadSpeed ?? 0,
          uploadSpeed: progress.UploadSpeed ?? 0,
          peers: progress.Peers ?? 0,
        });
        apolloClient.cache.updateQuery<TorrentByInfoHashWithFilesQuery>(
          {
            query: TorrentByInfoHashWithFilesDocument,
            variables: entityTorrentQueryVariables,
          },
          (existing) => {
            if (!existing?.Torrents?.Edges?.length) return existing;
            const currentNode = existing.Torrents.Edges[0]?.Node;
            if (!currentNode || currentNode.InfoHash !== progress.InfoHash) {
              return existing;
            }
            const nextEdges = [...existing.Torrents.Edges];
            nextEdges[0] = {
              ...nextEdges[0],
              Node: {
                ...currentNode,
                Progress: progress.Progress ?? currentNode.Progress,
                State: progress.State ?? currentNode.State,
              },
            };
            return {
              ...existing,
              Torrents: {
                ...existing.Torrents,
                Edges: nextEdges,
              },
            };
          },
        );
        return;
      }

      if (torrentId != null && progress.Id === torrentId) {
        void refetchLegacyDetails();
      }
    },
  });

  const hasEntityData = Boolean(entityTorrent);
  const hasLegacyData = Boolean(details);
  const showLoading = isEntityMode
    ? entityLoading && !hasEntityData
    : legacyLoading && !hasLegacyData;

  const error = useMemo(() => {
    if (!isOpen) return null;
    if (isEntityMode) {
      if (entityQueryError) return sanitizeError(entityQueryError);
      if (!entityLoading && !entityTorrent) return "Torrent not found";
      return null;
    }
    if (legacyQueryError) return sanitizeError(legacyQueryError);
    if (!legacyLoading && torrentId != null && !details) return "Torrent not found";
    return null;
  }, [
    isOpen,
    isEntityMode,
    entityQueryError,
    entityLoading,
    entityTorrent,
    legacyQueryError,
    legacyLoading,
    torrentId,
    details,
  ]);

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      size="5xl"
      scrollBehavior="inside"
      classNames={{
        wrapper: "overflow-hidden",
        base: "max-h-[90vh]",
      }}
    >
      <ModalContent>
        <ModalHeader className="flex flex-col gap-2 pb-4">
          <div className="flex items-start justify-between gap-4">
            <div className="min-w-0 flex-1">
              <h2 className="text-xl font-semibold truncate pr-4">
                {details?.name ?? entityTorrent?.Name ?? "Torrent Details"}
              </h2>
              {(details ?? entityTorrent) && (
                <code className="text-xs text-default-400 font-mono mt-1 block">
                  {details?.infoHash ?? entityTorrent?.InfoHash}
                </code>
              )}
            </div>
            {(details ?? entityTorrent) && (
              <div className="flex items-center gap-2 flex-shrink-0">
                {(() => {
                  const stateValue = (
                    details?.state ??
                    entityTorrent?.State ??
                    ""
                  ).toUpperCase() as keyof typeof TORRENT_STATE_INFO;
                  return (
                    <Chip
                      size="sm"
                      color={TORRENT_STATE_INFO[stateValue]?.color ?? "default"}
                      variant="flat"
                    >
                      {TORRENT_STATE_INFO[stateValue]?.label ?? stateValue}
                    </Chip>
                  );
                })()}
                {(details?.finished ??
                (entityTorrent && entityTorrent.Progress >= 1)) ? (
                  <Chip
                    size="sm"
                    color="success"
                    variant="flat"
                    startContent={
                      <IconCheck size={12} className="text-green-400" />
                    }
                  >
                    Complete
                  </Chip>
                ) : null}
              </div>
            )}
          </div>
        </ModalHeader>

        <ModalBody className="py-6">
          {showLoading && (
            <div className="flex flex-col items-center justify-center py-16 gap-3">
              <Spinner size="lg" />
              <span className="text-default-500 text-sm">
                Loading torrent details...
              </span>
            </div>
          )}

          {error && (
            <ErrorState title="Failed to Load Details" message={error} />
          )}

          {entityTorrent && !showLoading && !details && (
            <div className="space-y-6">
              <Card className="bg-content2/50">
                <CardBody className="p-4">
                  <div className="space-y-3">
                    <div className="flex items-center justify-between text-sm">
                      <span className="text-default-500">
                        {formatBytes(entityTorrent.DownloadedBytes)} of{" "}
                        {formatBytes(entityTorrent.TotalBytes)}
                      </span>
                      <span className="font-semibold tabular-nums">
                        {(entityTorrent.Progress * 100).toFixed(1)}%
                      </span>
                    </div>
                    <Progress
                      value={entityTorrent.Progress * 100}
                      color={
                        entityTorrent.State === "error"
                          ? "danger"
                          : entityTorrent.Progress >= 1
                            ? "success"
                            : "primary"
                      }
                      size="md"
                      aria-label="Download progress"
                      classNames={{ track: "h-3", indicator: "h-3" }}
                    />
                  </div>
                </CardBody>
              </Card>
              <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
                <StatCard
                  title="Downloaded"
                  value={formatBytes(entityTorrent.DownloadedBytes)}
                  icon={<IconArrowDown size={20} className="text-blue-400" />}
                  valueColor="primary"
                />
                <StatCard
                  title="Uploaded"
                  value={formatBytes(entityTorrent.UploadedBytes)}
                  icon={<IconArrowUp size={20} className="text-green-400" />}
                  valueColor="success"
                />
                <StatCard
                  title="Speed"
                  value={
                    entityLiveStats
                      ? `${formatBytes(entityLiveStats.downloadSpeed)}/s`
                      : "-"
                  }
                  subtitle={
                    entityLiveStats
                      ? `Up ${formatBytes(entityLiveStats.uploadSpeed)}/s`
                      : undefined
                  }
                  icon={<IconBolt size={20} className="text-amber-400" />}
                />
                <StatCard
                  title="Peers"
                  value={
                    entityLiveStats ? entityLiveStats.peers.toString() : "-"
                  }
                  icon={<IconUsers size={20} className="text-default-400" />}
                />
              </div>
              <div className="bg-content2/30 rounded-lg p-3">
                <div className="flex items-center gap-2 mb-1">
                  <IconFolder size={16} className="text-amber-400" />
                  <span className="text-xs font-medium text-default-500 uppercase tracking-wide">
                    Save Location
                  </span>
                </div>
                <code className="text-sm text-default-600 break-all">
                  {entityTorrent.SavePath}
                </code>
              </div>
              {entityTorrent.Files?.Edges?.length ? (
                <div className="space-y-3">
                  <span className="text-sm font-medium text-default-600">
                    Files ({entityTorrent.Files.Edges.length})
                  </span>
                  <DataTable
                    data={entityTorrent.Files.Edges.map((e: any) => e.Node)}
                    columns={[
                      {
                        key: "FilePath",
                        label: "File",
                        render: (n) => (
                          <div className="flex items-start gap-2 min-w-0">
                            <div className="mt-0.5 flex-shrink-0">
                              {getFileIcon(n.FilePath, false, { size: 18 })}
                            </div>
                            <span className="truncate block" title={n.FilePath}>
                              {n.FilePath.split(/[/\\]/).pop() ?? n.FilePath}
                            </span>
                          </div>
                        ),
                      },
                      {
                        key: "FileSize",
                        label: "Size",
                        render: (n) => formatBytes(n.FileSize),
                        width: 100,
                      },
                      {
                        key: "Progress",
                        label: "Progress",
                        render: (n) => (
                          <FileProgressBar
                            progress={n.Progress}
                            ariaLabel={`${n.FilePath} progress`}
                          />
                        ),
                        width: 300,
                      },
                      {
                        key: "actions",
                        label: "",
                        width: 72,
                        align: "center",
                        render: (n) => (
                          <FileActionsMenu
                            actionContext={buildActionContext(
                              n.FilePath,
                              n.FileSize,
                              `entity-${n.FileIndex}`,
                              n.FileIndex,
                            )}
                          />
                        ),
                      },
                    ]}
                    getRowKey={(n) => n.FileIndex.toString()}
                    isCompact
                    hideToolbar
                    removeWrapper
                  />
                </div>
              ) : null}
            </div>
          )}

          {details && !showLoading && (
            <div className="space-y-6">
              {/* Progress Section */}
              <Card className="bg-content2/50">
                <CardBody className="p-4">
                  <div className="space-y-3">
                    <div className="flex items-center justify-between text-sm">
                      <span className="text-default-500">
                        {details.downloadedFormatted} of {details.sizeFormatted}
                      </span>
                      <span className="font-semibold tabular-nums">
                        {details.progressPercent.toFixed(1)}%
                      </span>
                    </div>
                    <Progress
                      value={details.progressPercent}
                      color={
                        details.state === "ERROR"
                          ? "danger"
                          : details.finished
                            ? "success"
                            : "primary"
                      }
                      size="md"
                      aria-label="Download progress"
                      classNames={{
                        track: "h-3",
                        indicator: "h-3",
                      }}
                    />
                    {details.error && (
                      <div className="text-danger text-sm bg-danger-50/50 p-3 rounded-lg border border-danger-200 mt-3">
                        <strong>Error:</strong> {details.error}
                      </div>
                    )}
                  </div>
                </CardBody>
              </Card>

              {/* Stats Grid */}
              <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
                {/* Transfer Stats */}
                <StatCard
                  title="Download"
                  value={details.downloadSpeedFormatted}
                  subtitle={
                    details.timeRemainingFormatted
                      ? `ETA: ${details.timeRemainingFormatted}`
                      : undefined
                  }
                  icon={<IconArrowDown size={20} className="text-blue-400" />}
                  valueColor="primary"
                />
                <StatCard
                  title="Upload"
                  value={details.uploadSpeedFormatted}
                  subtitle={`Ratio: ${details.ratio.toFixed(2)}`}
                  icon={<IconArrowUp size={20} className="text-green-400" />}
                  valueColor={details.ratio >= 1 ? "success" : undefined}
                />
                <StatCard
                  title="Peers"
                  value={details.peerStats.live.toString()}
                  subtitle={`${details.peerStats.connecting} connecting`}
                  icon="👥"
                  valueColor="success"
                />
                <StatCard
                  title="Pieces"
                  value={`${details.piecesDownloaded} / ${details.pieceCount}`}
                  subtitle={
                    details.averagePieceDownloadMs
                      ? `Avg: ${details.averagePieceDownloadMs}ms`
                      : undefined
                  }
                  icon="🧩"
                />
              </div>

              {/* Detailed Stats Row */}
              <div className="grid grid-cols-3 lg:grid-cols-6 gap-3 text-sm">
                <MiniStat
                  label="Downloaded"
                  value={details.downloadedFormatted}
                />
                <MiniStat label="Uploaded" value={details.uploadedFormatted} />
                <MiniStat
                  label="Peers Queued"
                  value={details.peerStats.queued.toString()}
                />
                <MiniStat
                  label="Peers Seen"
                  value={details.peerStats.seen.toString()}
                />
                <MiniStat
                  label="Peers Dead"
                  value={details.peerStats.dead.toString()}
                  color="danger"
                />
                <MiniStat
                  label="Not Needed"
                  value={details.peerStats.notNeeded.toString()}
                />
              </div>

              {/* Save Path */}
              <div className="bg-content2/30 rounded-lg p-3">
                <div className="flex items-center gap-2 mb-1">
                  <IconFolder size={16} className="text-amber-400" />
                  <span className="text-xs font-medium text-default-500 uppercase tracking-wide">
                    Save Location
                  </span>
                </div>
                <code className="text-sm text-default-600 break-all">
                  {details.savePath}
                </code>
              </div>

              {/* Files Table */}
              {details.files.length > 0 && (
                <div className="space-y-3">
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-sm font-medium text-default-600">
                      Files ({details.files.length})
                    </span>
                    {fileMatches.length > 0 && (
                      <span className="text-xs text-default-400">
                        {
                          fileMatches.filter(
                            (m) =>
                              m.episodeId ||
                              m.movieId ||
                              m.trackId ||
                              m.chapterId,
                          ).length
                        }{" "}
                        matched
                      </span>
                    )}
                  </div>
                  <DataTable
                    skeletonDelay={500}
                    data={details.files}
                    columns={createFileColumns(
                      matchesByIndex,
                      (file) =>
                        buildActionContext(
                          file.path,
                          file.size,
                          `legacy-${file.index}`,
                          file.index,
                        ),
                      handleRemoveMatch,
                    )}
                    getRowKey={(file) => file.index}
                    isCompact
                    isStriped
                    hideToolbar
                    removeWrapper
                    showItemCount={false}
                    defaultSortColumn="path"
                    searchFn={(file, term) =>
                      file.path.toLowerCase().includes(term.toLowerCase())
                    }
                    searchPlaceholder="Search files..."
                    classNames={{
                      wrapper: "max-h-80",
                      table: "min-w-full",
                    }}
                  />
                </div>
              )}
            </div>
          )}
        </ModalBody>

        <ModalFooter className="pt-4">
          <Button variant="flat" onPress={onClose}>
            Close
          </Button>
        </ModalFooter>
      </ModalContent>
      <FilePropertiesModal
        isOpen={Boolean(propertiesMediaFileId)}
        onClose={() => setPropertiesMediaFileId(null)}
        mediaFileId={propertiesMediaFileId}
      />
      <MediaFilesMatchDialog
        isOpen={isMatchDialogOpen}
        onClose={() => {
          setIsMatchDialogOpen(false);
          setMatchFileIndex(null);
        }}
        torrentInfoHash={torrentInfoHash ?? details?.infoHash ?? null}
        initialFileIndex={matchFileIndex}
        onApplied={() => {
          void refetchMediaByPath();
          void refetchFileMatches();
        }}
      />
    </Modal>
  );
}

// Stat card component for main metrics
function StatCard({
  title,
  value,
  subtitle,
  icon,
  valueColor,
}: {
  title: string;
  value: string;
  subtitle?: string;
  icon: React.ReactNode;
  valueColor?: "primary" | "success" | "danger";
}) {
  const colorClass = valueColor
    ? valueColor === "success"
      ? "text-success"
      : valueColor === "danger"
        ? "text-danger"
        : "text-primary"
    : "text-foreground";

  return (
    <Card className="bg-content2/50">
      <CardBody className="p-3">
        <div className="flex items-start justify-between">
          <div>
            <span className="text-xs text-default-400 uppercase tracking-wide">
              {title}
            </span>
            <div className={`text-lg font-bold tabular-nums ${colorClass}`}>
              {value}
            </div>
            {subtitle && (
              <span className="text-xs text-default-400">{subtitle}</span>
            )}
          </div>
          <span className="text-xl opacity-60">{icon}</span>
        </div>
      </CardBody>
    </Card>
  );
}

// Mini stat for secondary metrics
function MiniStat({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color?: "success" | "danger" | "primary";
}) {
  const colorClass = color
    ? color === "success"
      ? "text-success"
      : color === "danger"
        ? "text-danger"
        : "text-primary"
    : "text-foreground";

  return (
    <div className="bg-content2/30 rounded-lg p-2 text-center">
      <div className="text-xs text-default-400 mb-0.5">{label}</div>
      <div className={`font-semibold tabular-nums ${colorClass}`}>{value}</div>
    </div>
  );
}
