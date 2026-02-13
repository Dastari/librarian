import { createFileRoute, redirect } from "@tanstack/react-router";
import { useState, useMemo, useCallback } from "react";
import { useDisclosure } from "@heroui/modal";
import { addToast } from "@heroui/toast";
import { Button } from "@heroui/button";
import { IconRefresh } from "@tabler/icons-react";
import { authFetch } from "../../lib/api/authFetch";
import {
  TORRENT_PROGRESS_SUBSCRIPTION,
  TORRENT_ADDED_SUBSCRIPTION,
  TORRENT_REMOVED_SUBSCRIPTION,
  TORRENT_COMPLETED_SUBSCRIPTION,
} from "../../lib/graphql";
import {
  useSubscription,
  gql,
  useQuery,
  useMutation,
  apolloClient,
} from "../../lib/graphql/client";
import {
  AddTorrentDocument,
  DownloadsTorrentsDocument,
  PauseTorrentByInfoHashDocument,
  ProcessSourceDocument,
  RemoveTorrentByInfoHashDocument,
  ResumeTorrentByInfoHashDocument,
  TorrentByInfoHashWithFilesDocument,
  type DownloadsTorrentsQuery,
  type DownloadsTorrentsQueryVariables,
  type ProcessSourceMutation,
  type ProcessSourceMutationVariables,
  type TorrentByInfoHashWithFilesQuery,
  type TorrentByInfoHashWithFilesQueryVariables,
} from "../../lib/graphql/generated/graphql";
import type { DownloadTorrent } from "../../components/downloads/types";
import {
  TorrentTable,
  AddTorrentModal,
  TorrentInfoModal,
  LinkToLibraryModal,
  MediaFilesMatchDialog,
  type MediaFileMatchInput,
} from "../../components/downloads";
import { sanitizeError } from "../../lib/format";
import { RouteError } from "../../components/RouteError";

export const Route = createFileRoute("/downloads/")({
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
  component: DownloadsPage,
  errorComponent: RouteError,
});

function DownloadsPage() {
  const downloadsQueryVariables = useMemo<DownloadsTorrentsQueryVariables>(
    () => ({ Page: { Limit: 500, Offset: 0 } }),
    [],
  );
  const {
    data,
    previousData,
    loading,
    refetch: refetchTorrents,
  } = useQuery(DownloadsTorrentsDocument, {
    variables: downloadsQueryVariables,
    fetchPolicy: "cache-and-network",
    notifyOnNetworkStatusChange: true,
  });

  const baseTorrents = useMemo<DownloadTorrent[]>(
    () =>
      (
        data?.Torrents?.Edges ??
        previousData?.Torrents?.Edges ??
        []
      ).map(({ Node }) => Node),
    [data?.Torrents?.Edges, previousData?.Torrents?.Edges],
  );

  const [liveStatsByInfoHash, setLiveStatsByInfoHash] = useState<
    Record<
      string,
      { downloadSpeed: number; uploadSpeed: number; peers: number }
    >
  >({});
  const [progressByInfoHash, setProgressByInfoHash] = useState<
    Record<string, { Progress: number; State: string }>
  >({});
  const [isAdding, setIsAdding] = useState(false);
  const { isOpen, onOpen, onClose } = useDisclosure();
  const {
    isOpen: isInfoOpen,
    onOpen: onInfoOpen,
    onClose: onInfoClose,
  } = useDisclosure();
  const {
    isOpen: isLinkOpen,
    onOpen: onLinkOpen,
    onClose: onLinkClose,
  } = useDisclosure();
  const {
    isOpen: isMatchOpen,
    onOpen: onMatchOpen,
    onClose: onMatchClose,
  } = useDisclosure();
  const [selectedTorrentInfoHash, setSelectedTorrentInfoHash] = useState<
    string | null
  >(null);
  const [torrentToLink, setTorrentToLink] = useState<DownloadTorrent | null>(
    null,
  );
  const [matchTorrentInfoHash, setMatchTorrentInfoHash] =
    useState<string | null>(null);
  const [matchMediaFiles, setMatchMediaFiles] = useState<MediaFileMatchInput[]>(
    [],
  );
  const [matchContextName, setMatchContextName] = useState<string | null>(null);
  const [addTorrentMutation] = useMutation(AddTorrentDocument);
  const [pauseTorrentByHash] = useMutation(PauseTorrentByInfoHashDocument);
  const [resumeTorrentByHash] = useMutation(ResumeTorrentByInfoHashDocument);
  const [removeTorrentByHash] = useMutation(RemoveTorrentByInfoHashDocument);
  const [processSource] = useMutation<
    ProcessSourceMutation,
    ProcessSourceMutationVariables
  >(ProcessSourceDocument);

  const torrents = useMemo(
    () =>
      baseTorrents.map((torrent) => {
        const override = progressByInfoHash[torrent.InfoHash];
        if (!override) return torrent;
        return {
          ...torrent,
          Progress: override.Progress,
          State: override.State,
        };
      }),
    [baseTorrents, progressByInfoHash],
  );

  const upsertTorrentInCache = useCallback(
    (torrent: {
      Id: string;
      InfoHash: string;
      Name: string;
      State: string;
      Progress: number;
      TotalBytes: number;
      DownloadedBytes: number;
      UploadedBytes: number;
      SavePath: string;
      AddedAt: string;
    }) => {
      apolloClient.cache.updateQuery<DownloadsTorrentsQuery, DownloadsTorrentsQueryVariables>(
        { query: DownloadsTorrentsDocument, variables: downloadsQueryVariables },
        (existing) => {
          if (!existing?.Torrents) {
            return {
              Torrents: {
                Edges: [{ Node: torrent }],
                PageInfo: { TotalCount: 1, HasNextPage: false },
              },
            };
          }

          const edges = existing.Torrents.Edges ?? [];
          const idx = edges.findIndex((edge) => edge.Node.InfoHash === torrent.InfoHash);

          if (idx >= 0) {
            const nextEdges = [...edges];
            nextEdges[idx] = {
              ...nextEdges[idx],
              Node: {
                ...nextEdges[idx].Node,
                ...torrent,
              },
            };
            return {
              ...existing,
              Torrents: {
                ...existing.Torrents,
                Edges: nextEdges,
              },
            };
          }

          return {
            ...existing,
            Torrents: {
              ...existing.Torrents,
              Edges: [{ Node: torrent }, ...edges],
              PageInfo: {
                ...existing.Torrents.PageInfo,
                TotalCount: (existing.Torrents.PageInfo.TotalCount ?? edges.length) + 1,
              },
            },
          };
        },
      );
    },
    [downloadsQueryVariables],
  );

  const removeTorrentFromCache = useCallback(
    (infoHash: string) => {
      apolloClient.cache.updateQuery<DownloadsTorrentsQuery, DownloadsTorrentsQueryVariables>(
        { query: DownloadsTorrentsDocument, variables: downloadsQueryVariables },
        (existing) => {
          if (!existing?.Torrents) return existing;
          const edges = existing.Torrents.Edges ?? [];
          const nextEdges = edges.filter((edge) => edge.Node.InfoHash !== infoHash);
          if (nextEdges.length === edges.length) return existing;
          return {
            ...existing,
            Torrents: {
              ...existing.Torrents,
              Edges: nextEdges,
              PageInfo: {
                ...existing.Torrents.PageInfo,
                TotalCount: Math.max(
                  0,
                  (existing.Torrents.PageInfo.TotalCount ?? edges.length) - 1,
                ),
              },
            },
          };
        },
      );
    },
    [downloadsQueryVariables],
  );

  // Realtime updates (torrent client events)
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
    onData: ({ data }) => {
      const event = data.data?.TorrentProgress;
      if (!event) {
        return;
      }
      setLiveStatsByInfoHash((prev) => ({
        ...prev,
        [event.InfoHash]: {
          downloadSpeed: event.DownloadSpeed ?? 0,
          uploadSpeed: event.UploadSpeed ?? 0,
          peers: event.Peers ?? 0,
        },
      }));
      setProgressByInfoHash((prev) => ({
        ...prev,
        [event.InfoHash]: {
          Progress: event.Progress,
          State: event.State,
        },
      }));
    },
  });

  useSubscription<{
    TorrentAdded: { Id: number; Name: string; InfoHash: string };
  }>(gql(TORRENT_ADDED_SUBSCRIPTION), {
    onData: ({ data }) => {
      const event = data.data?.TorrentAdded;
      if (!event) return;
      upsertTorrentInCache({
        Id: event.InfoHash,
        InfoHash: event.InfoHash,
        Name: event.Name,
        State: "queued",
        Progress: 0,
        TotalBytes: 0,
        DownloadedBytes: 0,
        UploadedBytes: 0,
        SavePath: "",
        AddedAt: new Date().toISOString(),
      });
    },
  });

  useSubscription<{ TorrentRemoved: { Id: number; InfoHash: string } }>(
    gql(TORRENT_REMOVED_SUBSCRIPTION),
    {
      onData: ({ data }) => {
        const event = data.data?.TorrentRemoved;
        if (!event) {
          return;
        }
        setLiveStatsByInfoHash((prev) => {
          const next = { ...prev };
          delete next[event.InfoHash];
          return next;
        });
        setProgressByInfoHash((prev) => {
          const next = { ...prev };
          delete next[event.InfoHash];
          return next;
        });
        removeTorrentFromCache(event.InfoHash);
      },
    },
  );

  useSubscription<{ TorrentCompleted: { Id: number; InfoHash: string } }>(
    gql(TORRENT_COMPLETED_SUBSCRIPTION),
    {
      onData: ({ data }) => {
        const event = data.data?.TorrentCompleted;
        if (!event) {
          return;
        }
        setProgressByInfoHash((prev) => ({
          ...prev,
          [event.InfoHash]: { State: "seeding", Progress: 1 },
        }));
        const existingTorrent = baseTorrents.find(
          (torrent) => torrent.InfoHash === event.InfoHash,
        );
        if (existingTorrent) {
          upsertTorrentInCache({
            ...existingTorrent,
            State: "seeding",
            Progress: 1,
          });
        }
      },
    },
  );

  // Add torrent handlers
  const handleAddMagnet = async (magnet: string) => {
    setIsAdding(true);
    try {
      const result = await addTorrentMutation({
        variables: {
          Input: { Magnet: magnet },
        },
      });
      const data = result.data?.AddTorrent;
      const success = data?.Success;
      const torrent = data?.Torrent;
      const err = data?.Error;
      if (success && torrent) {
        const name = torrent.Name;
        addToast({
          title: "Torrent Added",
          description: `Started downloading: ${name}`,
          color: "success",
        });
        void refetchTorrents();
      } else {
        addToast({
          title: "Error",
          description: sanitizeError(err ?? result.error?.message ?? "Failed"),
          color: "danger",
        });
      }
    } catch {
      addToast({
        title: "Error",
        description: "Failed to add torrent",
        color: "danger",
      });
    } finally {
      setIsAdding(false);
    }
  };

  const handleAddUrl = async (url: string) => {
    setIsAdding(true);
    try {
      const result = await addTorrentMutation({
        variables: {
          Input: { Url: url },
        },
      });
      const data = result.data?.AddTorrent;
      const success = data?.Success;
      const torrent = data?.Torrent;
      const err = data?.Error;
      if (success && torrent) {
        const name = torrent.Name;
        addToast({
          title: "Torrent Added",
          description: `Started downloading: ${name}`,
          color: "success",
        });
        void refetchTorrents();
      } else {
        addToast({
          title: "Error",
          description: sanitizeError(err ?? result.error?.message ?? "Failed"),
          color: "danger",
        });
      }
    } catch {
      addToast({
        title: "Error",
        description: "Failed to add torrent",
        color: "danger",
      });
    } finally {
      setIsAdding(false);
    }
  };

  const handleAddFile = async (file: File) => {
    setIsAdding(true);

    try {
      const formData = new FormData();
      formData.append("file", file);

      const response = await authFetch("/api/torrents/upload", {
        method: "POST",
        body: formData,
      });

      const data = await response.json();

      if (data.success && data.torrent) {
        void refetchTorrents();
        addToast({
          title: "Torrent Added",
          description: `Started downloading: ${data.torrent.name}`,
          color: "success",
        });
      } else {
        addToast({
          title: "Error",
          description: data.error || "Failed to upload torrent file",
          color: "danger",
        });
      }
    } catch (e) {
      addToast({
        title: "Error",
        description: "Failed to upload torrent file",
        color: "danger",
      });
      console.error(e);
    } finally {
      setIsAdding(false);
    }
  };

  // Single torrent actions (by infoHash – entity Torrents list)
  const handlePause = async (infoHash: string) => {
    const result = await pauseTorrentByHash({
      variables: {
        InfoHash: infoHash,
      },
    });
    const data = result.data?.PauseTorrentByInfoHash;
    if (data?.Success) {
      setProgressByInfoHash((prev) => ({
        ...prev,
        [infoHash]: {
          Progress: prev[infoHash]?.Progress ?? 0,
          State: "paused",
        },
      }));
      void refetchTorrents();
    }
  };

  const handleResume = async (infoHash: string) => {
    const result = await resumeTorrentByHash({
      variables: {
        InfoHash: infoHash,
      },
    });
    const data = result.data?.ResumeTorrentByInfoHash;
    if (data?.Success) {
      setProgressByInfoHash((prev) => ({
        ...prev,
        [infoHash]: {
          Progress: prev[infoHash]?.Progress ?? 0,
          State: "downloading",
        },
      }));
      void refetchTorrents();
    }
  };

  const handleRemove = async (infoHash: string) => {
    const result = await removeTorrentByHash({
      variables: {
        InfoHash: infoHash,
        DeleteFiles: false,
      },
    });
    const data = result.data?.RemoveTorrentByInfoHash;
    if (data?.Success) {
      setLiveStatsByInfoHash((prev) => {
        const next = { ...prev };
        delete next[infoHash];
        return next;
      });
      setProgressByInfoHash((prev) => {
        const next = { ...prev };
        delete next[infoHash];
        return next;
      });
      void refetchTorrents();
      addToast({
        title: "Torrent Removed",
        description: "The torrent has been removed.",
        color: "success",
      });
    }
  };

  const handleInfo = (infoHash: string) => {
    setSelectedTorrentInfoHash(infoHash);
    onInfoOpen();
  };

  // Process pending file matches (copy files to library)
  const handleProcess = async (torrent: DownloadTorrent) => {
    const result = await processSource({
      variables: {
        SourceType: "torrent",
        SourceId: torrent.InfoHash,
      },
    });
    if (result.data?.ProcessSource) {
      const proc = result.data.ProcessSource;
      if (proc.Success) {
        addToast({
          title: "Files Processed",
          description: `Copied ${proc.FilesProcessed} file(s) to library${proc.FilesFailed > 0 ? `, ${proc.FilesFailed} failed` : ""}`,
          color: "success",
        });
      } else {
        addToast({
          title: "Processing Failed",
          description:
            proc.Error || proc.Messages[0] || "Failed to process files",
          color: "danger",
        });
      }
    } else if (result.error) {
      addToast({
        title: "Error",
        description: sanitizeError(result.error),
        color: "danger",
      });
    }
  };

  const handleOpenMatchDialog = async (torrent: DownloadTorrent) => {
    setMatchTorrentInfoHash(torrent.InfoHash);
    setMatchContextName(torrent.Name);
    try {
      const result = await apolloClient.query<
        TorrentByInfoHashWithFilesQuery,
        TorrentByInfoHashWithFilesQueryVariables
      >({
        query: TorrentByInfoHashWithFilesDocument,
        variables: {
          Where: { InfoHash: { Eq: torrent.InfoHash } },
          Page: { Limit: 1, Offset: 0 },
        },
        fetchPolicy: "network-only",
      });
      const files =
        result.data?.Torrents?.Edges?.[0]?.Node?.Files?.Edges?.map((edge) => ({
          RowId: `torrent:${torrent.InfoHash}:${edge.Node.FileIndex}`,
          FileIndex: edge.Node.FileIndex,
          FilePath: edge.Node.FilePath,
          FileSize: edge.Node.FileSize,
        })) ?? [];
      setMatchMediaFiles(files);
    } catch {
      setMatchMediaFiles([]);
    }
    onMatchOpen();
  };

  // Bulk actions (by infoHash)
  const handleBulkPause = async (infoHashes: string[]) => {
    let successCount = 0;
    for (const infoHash of infoHashes) {
      const result = await pauseTorrentByHash({
        variables: {
          InfoHash: infoHash,
        },
      });
      if (result.data?.PauseTorrentByInfoHash?.Success) {
        successCount++;
      }
    }
    void refetchTorrents();
    addToast({
      title: "Paused Torrents",
      description: `Paused ${successCount} of ${infoHashes.length} torrent(s)`,
      color: "success",
    });
  };

  const handleBulkResume = async (infoHashes: string[]) => {
    let successCount = 0;
    for (const infoHash of infoHashes) {
      const result = await resumeTorrentByHash({
        variables: {
          InfoHash: infoHash,
        },
      });
      if (result.data?.ResumeTorrentByInfoHash?.Success) {
        successCount++;
      }
    }
    void refetchTorrents();
    addToast({
      title: "Resumed Torrents",
      description: `Resumed ${successCount} of ${infoHashes.length} torrent(s)`,
      color: "success",
    });
  };

  const handleBulkRemove = async (infoHashes: string[]) => {
    let successCount = 0;
    for (const infoHash of infoHashes) {
      const result = await removeTorrentByHash({
        variables: {
          InfoHash: infoHash,
          DeleteFiles: false,
        },
      });
      if (result.data?.RemoveTorrentByInfoHash?.Success) {
        successCount++;
        setLiveStatsByInfoHash((prev) => {
          const next = { ...prev };
          delete next[infoHash];
          return next;
        });
        setProgressByInfoHash((prev) => {
          const next = { ...prev };
          delete next[infoHash];
          return next;
        });
      }
    }
    void refetchTorrents();
    addToast({
      title: "Removed Torrents",
      description: `Removed ${successCount} of ${infoHashes.length} torrent(s)`,
      color: "success",
    });
  };

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 min-w-0 grow flex flex-col ">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Downloads</h1>
          <p className="text-default-500">Manage your torrent downloads</p>
        </div>
        <Button
          size="sm"
          variant="flat"
          startContent={<IconRefresh size={16} />}
          onPress={() => void refetchTorrents()}
          isLoading={loading}
        >
          Refresh
        </Button>
      </div>

      <TorrentTable
        torrents={torrents}
        isLoading={loading && torrents.length === 0}
        onPause={handlePause}
        onResume={handleResume}
        onRemove={handleRemove}
        onInfo={handleInfo}
        onProcess={handleProcess}
        onMatch={handleOpenMatchDialog}
        onLinkToLibrary={(torrent) => {
          setTorrentToLink(torrent);
          onLinkOpen();
        }}
        onBulkPause={handleBulkPause}
        onBulkResume={handleBulkResume}
        onBulkRemove={handleBulkRemove}
        onAddClick={onOpen}
        liveStatsByInfoHash={liveStatsByInfoHash}
      />

      {/* Add Torrent Modal */}
      <AddTorrentModal
        isOpen={isOpen}
        onClose={onClose}
        onAddMagnet={handleAddMagnet}
        onAddUrl={handleAddUrl}
        onAddFile={handleAddFile}
        isLoading={isAdding}
      />

      {/* Torrent Info Modal */}
      <TorrentInfoModal
        torrentInfoHash={selectedTorrentInfoHash}
        isOpen={isInfoOpen}
        onClose={onInfoClose}
      />

      {/* Link to Library Modal */}
      <LinkToLibraryModal
        isOpen={isLinkOpen}
        onClose={onLinkClose}
        torrent={torrentToLink}
        onLinked={() => void refetchTorrents()}
      />

      <MediaFilesMatchDialog
        isOpen={isMatchOpen}
        onClose={() => {
          setMatchMediaFiles([]);
          setMatchContextName(null);
          onMatchClose();
        }}
        mediaFiles={matchMediaFiles}
        contextName={matchContextName}
        torrentInfoHash={matchTorrentInfoHash}
        onApplied={() => void refetchTorrents()}
      />
    </div>
  );
}
