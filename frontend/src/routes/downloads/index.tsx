import { createFileRoute, redirect } from "@tanstack/react-router";
import { useState, useMemo, useCallback } from "react";
import { useDisclosure } from "@heroui/modal";
import { addToast } from "@heroui/toast";
import { Button } from "@heroui/button";
import { IconRefresh } from "@tabler/icons-react";
import { authFetch } from "../../lib/api/authFetch";
import {
  useSubscription,
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
  TorrentAddedDocument,
  TorrentByInfoHashWithFilesDocument,
  TorrentCompletedDocument,
  TorrentProgressDocument,
  TorrentRemovedDocument,
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
    () => ({ page: { limit: 500, offset: 0 } }),
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
      (data?.torrents?.edges ?? previousData?.torrents?.edges ?? []).map(
        ({ node }) => node,
      ),
    [data?.torrents?.edges, previousData?.torrents?.edges],
  );

  const [liveStatsByInfoHash, setLiveStatsByInfoHash] = useState<
    Record<
      string,
      { downloadSpeed: number; uploadSpeed: number; peers: number }
    >
  >({});
  const [progressByInfoHash, setProgressByInfoHash] = useState<
    Record<string, { progress: number; state: string }>
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
  const [matchTorrentInfoHash, setMatchTorrentInfoHash] = useState<
    string | null
  >(null);
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
        const override = progressByInfoHash[torrent.infoHash];
        if (!override) return torrent;
        return {
          ...torrent,
          progress: override.progress,
          state: override.state,
        };
      }),
    [baseTorrents, progressByInfoHash],
  );

  const upsertTorrentInCache = useCallback(
    (torrent: {
      id: string;
      infoHash: string;
      name: string;
      state: string;
      progress: number;
      totalBytes: number;
      downloadedBytes: number;
      uploadedBytes: number;
      savePath: string;
      addedAt: string;
    }) => {
      apolloClient.cache.updateQuery<
        DownloadsTorrentsQuery,
        DownloadsTorrentsQueryVariables
      >(
        {
          query: DownloadsTorrentsDocument,
          variables: downloadsQueryVariables,
        },
        (existing) => {
          if (!existing?.torrents) {
            return {
              torrents: {
                edges: [{ node: torrent }],
                pageInfo: { totalCount: 1, hasNextPage: false },
              },
            };
          }

          const edges = existing.torrents.edges ?? [];
          const idx = edges.findIndex(
            (edge) => edge.node.infoHash === torrent.infoHash,
          );

          if (idx >= 0) {
            const nextEdges = [...edges];
            nextEdges[idx] = {
              ...nextEdges[idx],
              node: {
                ...nextEdges[idx].node,
                ...torrent,
              },
            };
            return {
              ...existing,
              torrents: {
                ...existing.torrents,
                edges: nextEdges,
              },
            };
          }

          return {
            ...existing,
            torrents: {
              ...existing.torrents,
              edges: [{ node: torrent }, ...edges],
              pageInfo: {
                ...existing.torrents.pageInfo,
                totalCount:
                  (existing.torrents.pageInfo.totalCount ?? edges.length) + 1,
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
      apolloClient.cache.updateQuery<
        DownloadsTorrentsQuery,
        DownloadsTorrentsQueryVariables
      >(
        {
          query: DownloadsTorrentsDocument,
          variables: downloadsQueryVariables,
        },
        (existing) => {
          if (!existing?.torrents) return existing;
          const edges = existing.torrents.edges ?? [];
          const nextEdges = edges.filter(
            (edge) => edge.node.infoHash !== infoHash,
          );
          if (nextEdges.length === edges.length) return existing;
          return {
            ...existing,
            torrents: {
              ...existing.torrents,
              edges: nextEdges,
              pageInfo: {
                ...existing.torrents.pageInfo,
                totalCount: Math.max(
                  0,
                  (existing.torrents.pageInfo.totalCount ?? edges.length) - 1,
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
    onData: ({ data }) => {
      const event = data.data?.torrentProgress;
      if (!event) {
        return;
      }
      setLiveStatsByInfoHash((prev) => ({
        ...prev,
        [event.infoHash]: {
          downloadSpeed: event.downloadSpeed ?? 0,
          uploadSpeed: event.uploadSpeed ?? 0,
          peers: event.peers ?? 0,
        },
      }));
      setProgressByInfoHash((prev) => ({
        ...prev,
        [event.infoHash]: {
          progress: event.progress,
          state: event.state,
        },
      }));
    },
  });

  useSubscription<{
    torrentAdded: { id: number; name: string; infoHash: string };
  }>(TorrentAddedDocument, {
    onData: ({ data }) => {
      const event = data.data?.torrentAdded;
      if (!event) return;
      upsertTorrentInCache({
        id: event.infoHash,
        infoHash: event.infoHash,
        name: event.name,
        state: "queued",
        progress: 0,
        totalBytes: 0,
        downloadedBytes: 0,
        uploadedBytes: 0,
        savePath: "",
        addedAt: new Date().toISOString(),
      });
    },
  });

  useSubscription<{ torrentRemoved: { id: number; infoHash: string } }>(
    TorrentRemovedDocument,
    {
      onData: ({ data }) => {
        const event = data.data?.torrentRemoved;
        if (!event) {
          return;
        }
        setLiveStatsByInfoHash((prev) => {
          const next = { ...prev };
          delete next[event.infoHash];
          return next;
        });
        setProgressByInfoHash((prev) => {
          const next = { ...prev };
          delete next[event.infoHash];
          return next;
        });
        removeTorrentFromCache(event.infoHash);
      },
    },
  );

  useSubscription<{ torrentCompleted: { id: number; infoHash: string } }>(
    TorrentCompletedDocument,
    {
      onData: ({ data }) => {
        const event = data.data?.torrentCompleted;
        if (!event) {
          return;
        }
        setProgressByInfoHash((prev) => ({
          ...prev,
          [event.infoHash]: { state: "seeding", progress: 1 },
        }));
        const existingTorrent = baseTorrents.find(
          (torrent) => torrent.infoHash === event.infoHash,
        );
        if (existingTorrent) {
          upsertTorrentInCache({
            ...existingTorrent,
            state: "seeding",
            progress: 1,
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
          input: { magnet },
        },
      });
      const data = result.data?.addTorrent;
      const success = data?.success;
      const torrent = data?.torrent;
      const err = data?.error;
      if (success && torrent) {
        const name = torrent.name;
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
          input: { url },
        },
      });
      const data = result.data?.addTorrent;
      const success = data?.success;
      const torrent = data?.torrent;
      const err = data?.error;
      if (success && torrent) {
        const name = torrent.name;
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
        infoHash: infoHash,
      },
    });
    const data = result.data?.pauseTorrentByInfoHash;
    if (data?.success) {
      setProgressByInfoHash((prev) => ({
        ...prev,
        [infoHash]: {
          progress: prev[infoHash]?.progress ?? 0,
          state: "paused",
        },
      }));
      void refetchTorrents();
    }
  };

  const handleResume = async (infoHash: string) => {
    const result = await resumeTorrentByHash({
      variables: {
        infoHash: infoHash,
      },
    });
    const data = result.data?.resumeTorrentByInfoHash;
    if (data?.success) {
      setProgressByInfoHash((prev) => ({
        ...prev,
        [infoHash]: {
          progress: prev[infoHash]?.progress ?? 0,
          state: "downloading",
        },
      }));
      void refetchTorrents();
    }
  };

  const handleRemove = async (infoHash: string) => {
    const result = await removeTorrentByHash({
      variables: {
        infoHash: infoHash,
        deleteFiles: false,
      },
    });
    const data = result.data?.removeTorrentByInfoHash;
    if (data?.success) {
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
        sourceType: "torrent",
        sourceId: torrent.infoHash,
      },
    });
    if (result.data?.processSource) {
      const proc = result.data.processSource;
      if (proc.success) {
        addToast({
          title: "Files Processed",
          description: `Copied ${proc.filesProcessed} file(s) to library${proc.filesFailed > 0 ? `, ${proc.filesFailed} failed` : ""}`,
          color: "success",
        });
      } else {
        addToast({
          title: "Processing Failed",
          description:
            proc.error || proc.messages[0] || "Failed to process files",
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
    setMatchTorrentInfoHash(torrent.infoHash);
    setMatchContextName(torrent.name);
    try {
      const result = await apolloClient.query<
        TorrentByInfoHashWithFilesQuery,
        TorrentByInfoHashWithFilesQueryVariables
      >({
        query: TorrentByInfoHashWithFilesDocument,
        variables: {
          where: { infoHash: { eq: torrent.infoHash } },
          page: { limit: 1, offset: 0 },
        },
        fetchPolicy: "network-only",
      });
      const files =
        result.data?.torrents?.edges?.[0]?.node?.files?.edges?.map((edge) => ({
          RowId: `torrent:${torrent.infoHash}:${edge.node.fileIndex}`,
          fileIndex: edge.node.fileIndex,
          filePath: edge.node.filePath,
          fileSize: edge.node.fileSize,
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
          infoHash: infoHash,
        },
      });
      if (result.data?.pauseTorrentByInfoHash?.success) {
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
          infoHash: infoHash,
        },
      });
      if (result.data?.resumeTorrentByInfoHash?.success) {
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
          infoHash: infoHash,
          deleteFiles: false,
        },
      });
      if (result.data?.removeTorrentByInfoHash?.success) {
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
    <div className="container mx-auto flex h-full min-h-0 min-w-0 grow flex-col overflow-hidden px-4 py-8 sm:px-6 lg:px-8">
      <div className="mb-6 flex shrink-0 items-center justify-between">
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
