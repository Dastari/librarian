import { useState, useEffect, useCallback } from "react";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { Button } from "@heroui/button";
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
import { DataTable, type DataTableColumn } from "../data-table";
import { ErrorState } from "../shared";
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
} from "@tabler/icons-react";
import { getFileIcon } from "../../lib/fileIcons";
import {
  PendingFileMatchesBySourceDocument,
  TorrentByInfoHashWithFilesDocument,
  TorrentChangedDocument,
  type PendingFileMatchesBySourceQuery,
  type TorrentByInfoHashWithFilesQuery,
} from "../../lib/graphql/generated/graphql";
import { apolloClient } from "../../lib/graphql/client";

interface TorrentInfoModalProps {
  /** Legacy numeric id (session handle). Prefer torrentInfoHash when using entity list. */
  torrentId?: number | null;
  /** Entity torrent info hash – fetches one Torrent by InfoHash and shows basic info. */
  torrentInfoHash?: string | null;
  isOpen: boolean;
  onClose: () => void;
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

// Helper to create file columns with match info
function createFileColumns(
  matchesByIndex: Map<number, PendingFileMatch>,
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
      width: 50,
      align: "center",
      render: (file) => {
        const match = matchesByIndex.get(file.index);
        if (!match || !onRemoveMatch) {
          return null;
        }
        return (
          <Tooltip content="Remove match">
            <Button
              isIconOnly
              size="sm"
              variant="light"
              color="danger"
              onPress={() => onRemoveMatch(match.id)}
            >
              <IconTrash size={14} />
            </Button>
          </Tooltip>
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
  const [details, setDetails] = useState<TorrentDetails | null>(null);
  const [entityTorrent, setEntityTorrent] = useState<EntityTorrentNode | null>(
    null,
  );
  const [fileMatches, setFileMatches] = useState<PendingFileMatch[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [entityLiveStats, setEntityLiveStats] = useState<{
    downloadSpeed: number;
    uploadSpeed: number;
    peers: number;
  } | null>(null);

  const fetchEntityTorrent = useCallback(
    async (showLoading = false) => {
      if (!torrentInfoHash) return;
      if (showLoading) {
        setIsLoading(true);
      }

      try {
        const result = await apolloClient.query({
          query: TorrentByInfoHashWithFilesDocument,
          variables: {
            Where: { InfoHash: { Eq: torrentInfoHash } },
            Page: { Limit: 1, Offset: 0 },
          },
          fetchPolicy: "network-only",
        });
        const node = result.data?.Torrents?.Edges?.[0]?.Node;
        if (node) {
          setEntityTorrent(node);
        } else if (showLoading) {
          setError("Torrent not found");
        }
      } catch (e) {
        if (showLoading) {
          setError(e instanceof Error ? e.message : String(e));
        }
      } finally {
        if (showLoading) {
          setIsLoading(false);
        }
      }
    },
    [torrentInfoHash],
  );

  const fetchLegacyDetails = useCallback(
    async (showLoading = false, includeMatches = false) => {
      if (torrentId == null) return;
      if (showLoading) {
        setIsLoading(true);
      }

      try {
        const result = await queryPromise<{ torrentDetails: TorrentDetails }>(
          TORRENT_DETAILS_QUERY,
          {
            id: torrentId,
          },
        );
        if (result.data?.torrentDetails) {
          const det = result.data.torrentDetails;
          setDetails(det);

          if (includeMatches) {
            try {
              const matchResult = await apolloClient.query({
                query: PendingFileMatchesBySourceDocument,
                variables: {
                  Where: {
                    SourceType: { Eq: "torrent" },
                    SourceId: { Eq: det.infoHash },
                  },
                  Page: { Limit: 500, Offset: 0 },
                },
                fetchPolicy: "network-only",
              });

              if (matchResult.data?.PendingFileMatches?.Edges) {
                const mappedMatches: PendingFileMatch[] =
                  matchResult.data.PendingFileMatches.Edges.map(
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
                      matchType:
                        edge.Node.MatchType === "manual" ? "manual" : "auto",
                      matchConfidence: edge.Node.MatchConfidence ?? null,
                      parsedResolution: edge.Node.ParsedResolution ?? null,
                      parsedCodec: edge.Node.ParsedCodec ?? null,
                      parsedSource: edge.Node.ParsedSource ?? null,
                      parsedAudio: edge.Node.ParsedAudio ?? null,
                      copied: Boolean(
                        edge.Node.CopiedAt && !edge.Node.CopyError,
                      ),
                      copiedAt: edge.Node.CopiedAt ?? null,
                      copyError: edge.Node.CopyError ?? null,
                      createdAt: "",
                    }),
                  );
                setFileMatches(mappedMatches);
              }
            } catch {
              // File matches are optional, don't fail the whole modal
              console.warn("Failed to fetch file matches");
            }
          }
        } else if (showLoading && result.error) {
          setError(sanitizeError(result.error));
        } else if (showLoading) {
          setError("Torrent not found");
        }
      } catch (err) {
        if (showLoading) {
          setError(sanitizeError(err));
        }
      } finally {
        if (showLoading) {
          setIsLoading(false);
        }
      }
    },
    [torrentId],
  );

  // Handle removing a match
  const handleRemoveMatch = useCallback(async (matchId: string) => {
    const result = await mutationPromise<{
      removeMatch: RemoveMatchResult;
    }>(REMOVE_MATCH_MUTATION, { matchId });
    if (result.data?.removeMatch.success) {
      setFileMatches((prev) => prev.filter((m) => m.id !== matchId));
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
  }, []);

  // Create a map of file index to match for quick lookup
  const matchesByIndex = new Map(
    fileMatches
      .filter((m) => m.sourceFileIndex !== null)
      .map((m) => [m.sourceFileIndex as number, m]),
  );

  useEffect(() => {
    if (!isOpen) return;

    setDetails(null);
    setEntityTorrent(null);
    setFileMatches([]);
    setError(null);
    setEntityLiveStats(null);

    if (torrentInfoHash) {
      fetchEntityTorrent(true);
      return;
    }

    if (torrentId != null) {
      fetchLegacyDetails(true, true);
    }
  }, [
    isOpen,
    torrentId,
    torrentInfoHash,
    fetchEntityTorrent,
    fetchLegacyDetails,
  ]);

  useEffect(() => {
    if (!isOpen) return;

    const subscriptions: Array<{ unsubscribe?: () => void }> = [];

    if (torrentInfoHash && entityTorrent?.Id) {
      try {
        const torrentSub = subscriptionStream(
          TorrentChangedDocument,
          {},
        ).subscribe({
          next: () => {
            fetchEntityTorrent();
          },
          error: () => {},
        });
        subscriptions.push(torrentSub);
      } catch {
        // Ignore subscription setup failures
      }

      try {
        const fileSub = subscriptionStream<{
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
        }>(TORRENT_FILE_CHANGED_SUBSCRIPTION, {}).subscribe({
          next: (result: any) => {
            const payload = result.data?.TorrentFileChanged;
            const torrentFile = payload?.TorrentFile;
            if (!torrentFile || torrentFile.TorrentId !== entityTorrent.Id) {
              return;
            }

            setEntityTorrent((prev) => {
              if (!prev?.Files?.Edges) return prev;
              if (torrentFile.TorrentId !== prev.Id) return prev;

              const edges = prev.Files.Edges;
              const existingIndex = edges.findIndex(
                (edge) => edge.Node.FileIndex === torrentFile.FileIndex,
              );

              if (payload.Action === "Deleted") {
                if (existingIndex === -1) return prev;
                const nextEdges = edges.filter(
                  (_, index) => index !== existingIndex,
                );
                return {
                  ...prev,
                  Files: { ...prev.Files, Edges: nextEdges },
                };
              }

              const nextNode = {
                FileIndex: torrentFile.FileIndex,
                FilePath: torrentFile.FilePath,
                FileSize: torrentFile.FileSize,
                DownloadedBytes: torrentFile.DownloadedBytes,
                Progress: torrentFile.Progress,
              };

              if (existingIndex === -1) {
                return {
                  ...prev,
                  Files: {
                    ...prev.Files,
                    Edges: [...edges, { Node: nextNode }],
                  },
                };
              }

              const nextEdges = edges.map((edge, index) =>
                index === existingIndex ? { ...edge, Node: nextNode } : edge,
              );

              return { ...prev, Files: { ...prev.Files, Edges: nextEdges } };
            });
          },
          error: () => {},
        });
        subscriptions.push(fileSub);
      } catch {
        // Ignore subscription setup failures
      }

      try {
        const progressSub = subscriptionStream<{
          TorrentProgress: {
            InfoHash: string;
            DownloadSpeed: number;
            UploadSpeed: number;
            Peers: number;
          };
        }>(TORRENT_PROGRESS_SUBSCRIPTION).subscribe({
          next: (result: any) => {
            const progress = result.data?.TorrentProgress;
            if (!progress || progress.InfoHash !== torrentInfoHash) return;
            setEntityLiveStats({
              downloadSpeed: progress.DownloadSpeed ?? 0,
              uploadSpeed: progress.UploadSpeed ?? 0,
              peers: progress.Peers ?? 0,
            });
          },
          error: () => {},
        });
        subscriptions.push(progressSub);
      } catch {
        // Ignore subscription setup failures
      }
    }

    if (torrentId != null) {
      try {
        const progressSub = subscriptionStream<{
          TorrentProgress: { Id: number };
        }>(TORRENT_PROGRESS_SUBSCRIPTION).subscribe({
          next: (result: any) => {
            if (result.data?.TorrentProgress?.Id === torrentId) {
              fetchLegacyDetails(false, false);
            }
          },
          error: () => {},
        });
        subscriptions.push(progressSub);
      } catch {
        // Ignore subscription setup failures
      }
    }

    return () => {
      subscriptions.forEach((sub) => sub.unsubscribe?.());
    };
  }, [
    isOpen,
    torrentInfoHash,
    torrentId,
    entityTorrent?.Id,
    fetchEntityTorrent,
    fetchLegacyDetails,
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
          {isLoading && (
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

          {entityTorrent && !isLoading && !details && (
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

          {details && !isLoading && (
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
