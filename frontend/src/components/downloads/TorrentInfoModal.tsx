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
import type {
  TorrentDetails,
  TorrentFileInfo,
  PendingFileMatch,
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
  DeletePendingFileMatchFromTorrentModalDocument,
  PendingFileMatchesBySourceDocument,
  TorrentModalMediaFilesByPathsDocument,
  TorrentByInfoHashWithFilesDocument,
  TorrentFileChangedDocument,
  TorrentProgressDocument,
  TorrentUnmatchMediaFileRuntimeDocument,
  type AnalyzeMediaFileForTorrentMutation,
  type AnalyzeMediaFileForTorrentMutationVariables,
  type CreateUnmatchedMediaFileFromTorrentMutation,
  type CreateUnmatchedMediaFileFromTorrentMutationVariables,
  type DeletePendingFileMatchFromTorrentModalMutation,
  type PendingFileMatchesBySourceQuery,
  type TorrentModalMediaFilesByPathsQuery,
  type TorrentByInfoHashWithFilesQueryVariables,
  type TorrentByInfoHashWithFilesQuery,
  type TorrentUnmatchMediaFileRuntimeMutation,
} from "../../lib/graphql/generated/graphql";
import {
  apolloClient,
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
      candidates.add(
        joinPath(joinPath(savePath, torrentName), normalizedFilePath),
      );
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
  return Boolean(
    extension && SUPPORTED_TORRENT_MEDIA_EXTENSIONS.has(extension),
  );
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
    TorrentByInfoHashWithFilesQuery["torrents"]["edges"][number]["node"];

  const [entityLiveStats, setEntityLiveStats] = useState<{
    downloadSpeed: number;
    uploadSpeed: number;
    peers: number;
  } | null>(null);
  const [removedMatchIds, setRemovedMatchIds] = useState<Set<string>>(
    () => new Set(),
  );
  const [propertiesMediaFileId, setPropertiesMediaFileId] = useState<
    string | null
  >(null);
  const [processingFileKey, setProcessingFileKey] = useState<string | null>(
    null,
  );
  const [isMatchDialogOpen, setIsMatchDialogOpen] = useState(false);
  const [matchFileIndex, setMatchFileIndex] = useState<number | null>(null);

  const isEntityMode = Boolean(torrentInfoHash);
  const entityTorrentQueryVariables =
    useMemo<TorrentByInfoHashWithFilesQueryVariables>(
      () => ({
        where: { infoHash: { eq: torrentInfoHash ?? "" } },
        page: { limit: 1, offset: 0 },
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
      entityData?.torrents?.edges ?? previousEntityData?.torrents?.edges ?? [];
    return edges[0]?.node ?? null;
  }, [entityData?.torrents?.edges, previousEntityData?.torrents?.edges]);

  const details = null as unknown as TorrentDetails | null;
  const sourceInfoHash =
    details?.infoHash ?? entityTorrent?.infoHash ?? torrentInfoHash ?? null;

  const {
    data: fileMatchesData,
    previousData: previousFileMatchesData,
    refetch: refetchFileMatches,
  } = useQuery(PendingFileMatchesBySourceDocument, {
    variables: {
      where: {
        sourceType: { eq: "torrent" },
        sourceId: { eq: sourceInfoHash ?? "" },
      },
      page: { limit: 500, offset: 0 },
    },
    skip: !isOpen || !sourceInfoHash,
    fetchPolicy: "cache-and-network",
    notifyOnNetworkStatusChange: true,
  });

  const [removeMatchMutation] =
    useMutation<DeletePendingFileMatchFromTorrentModalMutation>(
      DeletePendingFileMatchFromTorrentModalDocument,
    );
  const [createUnmatchedMediaFile] = useMutation<
    CreateUnmatchedMediaFileFromTorrentMutation,
    CreateUnmatchedMediaFileFromTorrentMutationVariables
  >(CreateUnmatchedMediaFileFromTorrentDocument);
  const [analyzeMediaFile] = useMutation<
    AnalyzeMediaFileForTorrentMutation,
    AnalyzeMediaFileForTorrentMutationVariables
  >(AnalyzeMediaFileForTorrentDocument);
  const [unmatchMediaFile] = useMutation<
    TorrentUnmatchMediaFileRuntimeMutation
  >(TorrentUnmatchMediaFileRuntimeDocument);

  const fileMatches = useMemo<PendingFileMatch[]>(() => {
    const edges =
      fileMatchesData?.pendingFileMatches?.edges ??
      previousFileMatchesData?.pendingFileMatches?.edges ??
      [];
    return edges
      .map(
        (
          edge: PendingFileMatchesBySourceQuery["pendingFileMatches"]["edges"][number],
        ) => ({
          id: edge.node.id,
          sourceType: edge.node.sourceType,
          sourceId: edge.node.sourceId ?? null,
          sourceFileIndex: edge.node.sourceFileIndex ?? null,
          sourcePath: edge.node.sourcePath,
          fileSize: edge.node.fileSize,
          episodeId: edge.node.episodeId ?? null,
          movieId: edge.node.movieId ?? null,
          trackId: edge.node.trackId ?? null,
          chapterId: edge.node.chapterId ?? null,
          matchType: (edge.node.matchType === "manual"
            ? "manual"
            : "auto") as PendingFileMatch["matchType"],
          matchConfidence: edge.node.matchConfidence ?? null,
          parsedResolution: edge.node.parsedResolution ?? null,
          parsedCodec: edge.node.parsedCodec ?? null,
          parsedSource: edge.node.parsedSource ?? null,
          parsedAudio: edge.node.parsedAudio ?? null,
          copied: Boolean(edge.node.copiedAt && !edge.node.copyError),
          copiedAt: edge.node.copiedAt ?? null,
          copyError: edge.node.copyError ?? null,
          createdAt: "",
        }),
      )
      .filter((match) => !removedMatchIds.has(match.id));
  }, [
    fileMatchesData?.pendingFileMatches?.edges,
    previousFileMatchesData?.pendingFileMatches?.edges,
    removedMatchIds,
  ]);

  // Handle removing a match
  const handleRemoveMatch = useCallback(
    async (matchId: string) => {
      const result = await removeMatchMutation({
        variables: { id: matchId },
      });
      if (result.data?.deletePendingFileMatch.success) {
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
          description:
            result.data?.deletePendingFileMatch.error || "Failed to remove match",
          color: "danger",
        });
      }
    },
    [removeMatchMutation, refetchFileMatches],
  );

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

  const currentSavePath = details?.savePath ?? entityTorrent?.savePath ?? null;
  const currentTorrentName = details?.name ?? entityTorrent?.name ?? null;

  const visibleFileRows = useMemo(
    () =>
      isEntityMode
        ? (entityTorrent?.files?.edges?.map((e) => e.node) ?? []).map(
            (file) => ({
              key: `entity-${file.fileIndex}`,
              filePath: file.filePath,
            }),
          )
        : (details?.files ?? []).map((file) => ({
            key: `legacy-${file.index}`,
            filePath: file.path,
          })),
    [isEntityMode, entityTorrent?.files?.edges, details?.files],
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
    variables: { paths: mediaLookupPaths },
    skip: !isOpen || mediaLookupPaths.length === 0,
    fetchPolicy: "cache-and-network",
    notifyOnNetworkStatusChange: true,
  });

  type MediaLookupNode =
    TorrentModalMediaFilesByPathsQuery["mediaFiles"]["edges"][number]["node"] & {
      episodeId?: string | null;
      movieId?: string | null;
      trackId?: string | null;
      chapterId?: string | null;
    };

  const mediaByNormalizedPath = useMemo(() => {
    const map = new Map<string, MediaLookupNode>();
    const edges =
      mediaByPathData?.mediaFiles?.edges ??
      previousMediaByPathData?.mediaFiles?.edges ??
      [];
    for (const edge of edges) {
      map.set(normalizePathForLookup(edge.node.path), edge.node);
    }
    return map;
  }, [
    mediaByPathData?.mediaFiles?.edges,
    previousMediaByPathData?.mediaFiles?.edges,
  ]);

  const resolveMediaForFile = useCallback(
    (filePath: string) => {
      const candidates = buildPathCandidates(
        filePath,
        currentSavePath,
        currentTorrentName,
      );
      for (const candidate of candidates) {
        const media = mediaByNormalizedPath.get(
          normalizePathForLookup(candidate),
        );
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
      if (currentMedia && hasFfprobeData(currentMedia.metadata)) {
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
        let mediaFileId = currentMedia?.id ?? null;
        let analyzePath = currentMedia?.path ?? null;

        if (!mediaFileId) {
          const bestPath = getBestFilePath(
            filePath,
            currentSavePath,
            currentTorrentName,
          );
          const relativePath = getRelativePath(
            bestPath,
            currentSavePath,
            currentTorrentName,
          );
          const originalName = bestPath.split("/").pop() ?? bestPath;

          const createResult = await createUnmatchedMediaFile({
            variables: {
              input: {
                addedAt: new Date().toISOString(),
                isHdr: false,
                libraryId: UNMATCHED_LIBRARY_ID,
                metadata: JSON.stringify({
                  sourceType: "torrent",
                  unmatchedReason: "Manually processed from torrent modal",
                }),
                originalName: originalName,
                path: bestPath,
                relativePath: relativePath,
                size: Math.max(0, Math.floor(fileSize)),
              },
            },
          });

          const createData = createResult.data?.createMediaFile;
          if (!createData?.success || !createData.mediaFile?.id) {
            throw new Error(createData?.error || "Failed to create media file");
          }

          mediaFileId = createData.mediaFile.id;
          analyzePath = createData.mediaFile.path;
        }

        if (!mediaFileId || !analyzePath) {
          throw new Error("Missing media file information for analysis");
        }

        const analyzeResult = await analyzeMediaFile({
          variables: {
            mediaFileId: mediaFileId,
            path: analyzePath,
          },
        });
        const analyzeData = analyzeResult.data?.analyzeMediaFile;
        if (!analyzeData?.success) {
          throw new Error(
            analyzeData?.message || "Failed to queue media analysis",
          );
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
      if (mediaFile?.id) {
        setPropertiesMediaFileId(mediaFile.id);
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
            paths: buildPathCandidates(
              filePath,
              currentSavePath,
              currentTorrentName,
            ),
          },
          fetchPolicy: "network-only",
        });
        const found = result.data?.mediaFiles?.edges?.[0]?.node;
        if (found?.id) {
          setPropertiesMediaFileId(found.id);
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
      const analyzed = hasFfprobeData(mediaFile?.metadata);
      const canProcess = isSupportedTorrentMediaPath(filePath);
      const existingMatchId =
        mediaFile?.episodeId ??
        mediaFile?.movieId ??
        mediaFile?.trackId ??
        mediaFile?.chapterId ??
        null;
      const hasExistingMatch = Boolean(existingMatchId);

      return {
        mediaFileId: mediaFile?.id ?? null,
        hasAnalyzedMedia: analyzed,
        canProcess,
        processFileKey: rowKey,
        isProcessing: processingFileKey === rowKey,
        matchLabel: hasExistingMatch ? "Rematch" : "Match",
        onOpenMatch: () => {
          setMatchFileIndex(fileIndex);
          setIsMatchDialogOpen(true);
        },
        canUnmatch: hasExistingMatch && Boolean(mediaFile?.id),
        onUnmatch: mediaFile?.id
          ? () => {
              void (async () => {
                try {
                  const result = await unmatchMediaFile({
                    variables: { mediaFileId: mediaFile.id },
                  });
                  if (!result.data?.unmatchMediaFile?.success) {
                    addToast({
                      title: "Unmatch failed",
                      description:
                        result.data?.unmatchMediaFile?.reason ||
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
    torrentFileChanged: {
      action: "Created" | "Updated" | "Deleted";
      id: string;
      torrentFile?: {
        torrentId: string;
        fileIndex: number;
        filePath: string;
        fileSize: number;
        downloadedBytes: number;
        progress: number;
      };
    };
  }>(TorrentFileChangedDocument, {
    skip: !isOpen || !torrentInfoHash || !entityTorrent?.id,
    onData: ({ data }) => {
      const payload = data.data?.torrentFileChanged;
      const torrentFile = payload?.torrentFile;
      if (!torrentFile || torrentFile.torrentId !== entityTorrent?.id) {
        return;
      }
      apolloClient.cache.updateQuery<TorrentByInfoHashWithFilesQuery>(
        {
          query: TorrentByInfoHashWithFilesDocument,
          variables: entityTorrentQueryVariables,
        },
        (existing) => {
          if (!existing?.torrents?.edges?.length) {
            return existing;
          }
          const currentNode = existing.torrents.edges[0]?.node;
          if (!currentNode || currentNode.id !== torrentFile.torrentId) {
            return existing;
          }

          const currentEdges = currentNode.files?.edges ?? [];
          const existingIndex = currentEdges.findIndex(
            (edge) => edge.node.fileIndex === torrentFile.fileIndex,
          );

          let nextFileEdges = currentEdges;
          if (payload.action === "Deleted") {
            if (existingIndex === -1) return existing;
            nextFileEdges = currentEdges.filter(
              (edge) => edge.node.fileIndex !== torrentFile.fileIndex,
            );
          } else if (existingIndex >= 0) {
            nextFileEdges = [...currentEdges];
            nextFileEdges[existingIndex] = {
              ...nextFileEdges[existingIndex],
              node: {
                ...nextFileEdges[existingIndex].node,
                fileIndex: torrentFile.fileIndex,
                filePath: torrentFile.filePath,
                fileSize: torrentFile.fileSize,
                downloadedBytes: torrentFile.downloadedBytes,
                progress: torrentFile.progress,
              },
            };
          } else {
            nextFileEdges = [
              ...currentEdges,
              {
                node: {
                  fileIndex: torrentFile.fileIndex,
                  filePath: torrentFile.filePath,
                  fileSize: torrentFile.fileSize,
                  downloadedBytes: torrentFile.downloadedBytes,
                  progress: torrentFile.progress,
                },
              },
            ];
          }

          const nextEdges = [...existing.torrents.edges];
          nextEdges[0] = {
            ...nextEdges[0],
            node: {
              ...currentNode,
              files: {
                ...currentNode.files,
                edges: nextFileEdges,
              },
            },
          };

          return {
            ...existing,
            torrents: {
              ...existing.torrents,
              edges: nextEdges,
            },
          };
        },
      );
    },
  });

  useSubscription<{
    torrentProgress: {
      id: number;
      infoHash: string;
      progress: number;
      downloadSpeed: number;
      uploadSpeed: number;
      peers: number;
      state: string;
    };
  }>(TorrentProgressDocument, {
    skip: !isOpen || (!torrentInfoHash && torrentId == null),
    onData: ({ data }) => {
      const progress = data.data?.torrentProgress;
      if (!progress) return;

      if (torrentInfoHash) {
        if (progress.infoHash !== torrentInfoHash) return;
        setEntityLiveStats({
          downloadSpeed: progress.downloadSpeed ?? 0,
          uploadSpeed: progress.uploadSpeed ?? 0,
          peers: progress.peers ?? 0,
        });
        apolloClient.cache.updateQuery<TorrentByInfoHashWithFilesQuery>(
          {
            query: TorrentByInfoHashWithFilesDocument,
            variables: entityTorrentQueryVariables,
          },
          (existing) => {
            if (!existing?.torrents?.edges?.length) return existing;
            const currentNode = existing.torrents.edges[0]?.node;
            if (!currentNode || currentNode.infoHash !== progress.infoHash) {
              return existing;
            }
            const nextEdges = [...existing.torrents.edges];
            nextEdges[0] = {
              ...nextEdges[0],
              node: {
                ...currentNode,
                progress: progress.progress ?? currentNode.progress,
                state: progress.state ?? currentNode.state,
              },
            };
            return {
              ...existing,
              torrents: {
                ...existing.torrents,
                edges: nextEdges,
              },
            };
          },
        );
        return;
      }

      return;
    },
  });

  const hasEntityData = Boolean(entityTorrent);
  const showLoading = isEntityMode ? entityLoading && !hasEntityData : false;

  const error = useMemo(() => {
    if (!isOpen) return null;
    if (isEntityMode) {
      if (entityQueryError) return sanitizeError(entityQueryError);
      if (!entityLoading && !entityTorrent) return "Torrent not found";
      return null;
    }
    if (torrentId != null && !details) return "Torrent not found";
    return null;
  }, [
    isOpen,
    isEntityMode,
    entityQueryError,
    entityLoading,
    entityTorrent,
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
                {details?.name ?? entityTorrent?.name ?? "Torrent Details"}
              </h2>
              {(details ?? entityTorrent) && (
                <code className="text-xs text-default-400 font-mono mt-1 block">
                  {details?.infoHash ?? entityTorrent?.infoHash}
                </code>
              )}
            </div>
            {(details ?? entityTorrent) && (
              <div className="flex items-center gap-2 flex-shrink-0">
                {(() => {
                  const stateValue = (
                    details?.state ??
                    entityTorrent?.state ??
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
                (entityTorrent && entityTorrent.progress >= 1)) ? (
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
                        {formatBytes(entityTorrent.downloadedBytes)} of{" "}
                        {formatBytes(entityTorrent.totalBytes)}
                      </span>
                      <span className="font-semibold tabular-nums">
                        {(entityTorrent.progress * 100).toFixed(1)}%
                      </span>
                    </div>
                    <Progress
                      value={entityTorrent.progress * 100}
                      color={
                        entityTorrent.state === "error"
                          ? "danger"
                          : entityTorrent.progress >= 1
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
                  value={formatBytes(entityTorrent.downloadedBytes)}
                  icon={<IconArrowDown size={20} className="text-blue-400" />}
                  valueColor="primary"
                />
                <StatCard
                  title="Uploaded"
                  value={formatBytes(entityTorrent.uploadedBytes)}
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
                  {entityTorrent.savePath}
                </code>
              </div>
              {entityTorrent.files?.edges?.length ? (
                <div className="space-y-3">
                  <span className="text-sm font-medium text-default-600">
                    Files ({entityTorrent.files.edges.length})
                  </span>
                  <DataTable
                    data={entityTorrent.files.edges.map((e: any) => e.node)}
                    columns={[
                      {
                        key: "FilePath",
                        label: "File",
                        render: (n) => (
                          <div className="flex items-start gap-2 min-w-0">
                            <div className="mt-0.5 flex-shrink-0">
                              {getFileIcon(n.filePath, false, { size: 18 })}
                            </div>
                            <span className="truncate block" title={n.filePath}>
                              {n.filePath.split(/[/\\]/).pop() ?? n.filePath}
                            </span>
                          </div>
                        ),
                      },
                      {
                        key: "FileSize",
                        label: "Size",
                        render: (n) => formatBytes(n.fileSize),
                        width: 100,
                      },
                      {
                        key: "Progress",
                        label: "Progress",
                        render: (n) => (
                          <FileProgressBar
                            progress={n.progress}
                            ariaLabel={`${n.filePath} progress`}
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
                              n.filePath,
                              n.fileSize,
                              `entity-${n.fileIndex}`,
                              n.fileIndex,
                            )}
                          />
                        ),
                      },
                    ]}
                    getRowKey={(n) => n.fileIndex.toString()}
                    fillHeight={false}
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
                        <strong>error:</strong> {details.error}
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
                    fillHeight={false}
                    isCompact
                    isStriped
                    hideToolbar
                    removeWrapper
                    showItemCount={false}
                    defaultSortColumn="path"
                    searchFn={(file, term) =>
                      file.path.toLowerCase().includes(term.toLowerCase())
                    }
                    toolbarQueryPlaceholder="Search files..."
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
