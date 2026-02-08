import { createFileRoute, redirect } from "@tanstack/react-router";
import { useState, useEffect, useCallback } from "react";
import { getAccessToken } from "../../lib/auth";
import { useDisclosure } from "@heroui/modal";
import { addToast } from "@heroui/toast";
import { Button } from "@heroui/button";
import { IconRefresh } from "@tabler/icons-react";
import {
  TORRENT_PROGRESS_SUBSCRIPTION,
  TORRENT_ADDED_SUBSCRIPTION,
  TORRENT_REMOVED_SUBSCRIPTION,
  TORRENT_COMPLETED_SUBSCRIPTION,
} from "../../lib/graphql";
import {
  useSubscription,
  gql,
  apolloClient,
  useMutation,
} from "../../lib/graphql/client";
import {
  AddTorrentDocument,
  DownloadsTorrentsDocument,
  PauseTorrentByInfoHashDocument,
  ProcessSourceDocument,
  RematchSourceDocument,
  RemoveTorrentByInfoHashDocument,
  ResumeTorrentByInfoHashDocument,
  type DownloadsTorrentsQuery,
  type ProcessSourceMutation,
  type ProcessSourceMutationVariables,
  type RematchSourceMutation,
  type RematchSourceMutationVariables,
} from "../../lib/graphql/generated/graphql";
import type { DownloadTorrent } from "../../components/downloads/types";
import {
  TorrentTable,
  AddTorrentModal,
  TorrentInfoModal,
  LinkToLibraryModal,
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
  const [torrents, setTorrents] = useState<DownloadTorrent[]>([]);
  const [liveStatsByInfoHash, setLiveStatsByInfoHash] = useState<
    Record<
      string,
      { downloadSpeed: number; uploadSpeed: number; peers: number }
    >
  >({});
  const [isAdding, setIsAdding] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
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
  const [selectedTorrentInfoHash, setSelectedTorrentInfoHash] = useState<
    string | null
  >(null);
  const [torrentToLink, setTorrentToLink] = useState<DownloadTorrent | null>(
    null,
  );
  const [addTorrentMutation] = useMutation(AddTorrentDocument);
  const [pauseTorrentByHash] = useMutation(PauseTorrentByInfoHashDocument);
  const [resumeTorrentByHash] = useMutation(ResumeTorrentByInfoHashDocument);
  const [removeTorrentByHash] = useMutation(RemoveTorrentByInfoHashDocument);
  const [processSource] = useMutation<
    ProcessSourceMutation,
    ProcessSourceMutationVariables
  >(ProcessSourceDocument);
  const [rematchSource] = useMutation<
    RematchSourceMutation,
    RematchSourceMutationVariables
  >(RematchSourceDocument);

  const fetchTorrents = useCallback(async () => {
    try {
      const result = await apolloClient.query<DownloadsTorrentsQuery>({
        query: DownloadsTorrentsDocument,
        fetchPolicy: "network-only",
        variables: { Page: { Limit: 500, Offset: 0 } },
      });
      if (result.data?.Torrents?.Edges) {
        const torrentNodes = result.data.Torrents.Edges.map(({ Node }) => Node);
        setTorrents(torrentNodes);
      }
      if (result.error) {
        const isAuthError = result.error.message
          ?.toLowerCase()
          .includes("authentication");
        if (!isAuthError) {
          addToast({
            title: "Error",
            description: sanitizeError(result.error),
            color: "danger",
          });
        }
      }
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      if (!errorMsg.toLowerCase().includes("authentication")) {
        addToast({
          title: "Error",
          description: sanitizeError(e),
          color: "danger",
        });
      }
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Fetch torrents
  useEffect(() => {
    fetchTorrents();
  }, [fetchTorrents]);

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
      setTorrents((prev) =>
        prev.map((torrent) =>
          torrent.InfoHash === event.InfoHash
            ? {
                ...torrent,
                Progress: event.Progress,
                State: event.State,
              }
            : torrent,
        ),
      );
    },
  });

  useSubscription<{
    TorrentAdded: { Id: number; Name: string; InfoHash: string };
  }>(gql(TORRENT_ADDED_SUBSCRIPTION), {
    onData: () => {
      fetchTorrents();
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
        setTorrents((prev) =>
          prev.filter((torrent) => torrent.InfoHash !== event.InfoHash),
        );
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
        setTorrents((prev) =>
          prev.map((torrent) =>
            torrent.InfoHash === event.InfoHash
              ? { ...torrent, State: "seeding", Progress: 1 }
              : torrent,
          ),
        );
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
        fetchTorrents();
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
        fetchTorrents();
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

      const API_URL = import.meta.env.VITE_API_URL || "http://localhost:3001";

      // Get auth token from cookie storage
      const authToken = getAccessToken() || "";

      const response = await fetch(`${API_URL}/api/torrents/upload`, {
        method: "POST",
        headers: authToken ? { Authorization: `Bearer ${authToken}` } : {},
        body: formData,
      });

      const data = await response.json();

      if (data.success && data.torrent) {
        fetchTorrents();
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
      setTorrents((prev) =>
        prev.map((t) =>
          t.InfoHash === infoHash ? { ...t, State: "paused" } : t,
        ),
      );
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
      setTorrents((prev) =>
        prev.map((t) =>
          t.InfoHash === infoHash ? { ...t, State: "downloading" } : t,
        ),
      );
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
      setTorrents((prev) => prev.filter((t) => t.InfoHash !== infoHash));
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

  const handleOrganize = async (_infoHash: string) => {
    addToast({
      title: "Organize",
      description: "Organize by info hash is not yet available from this view.",
      color: "default",
    });
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

  // Re-match files against library items
  const handleRematch = async (torrent: DownloadTorrent) => {
    const result = await rematchSource({
      variables: {
        SourceType: "torrent",
        SourceId: torrent.InfoHash,
        LibraryId: null, // Match against all libraries
      },
    });
    if (result.data?.RematchSource) {
      const match = result.data.RematchSource;
      if (match.Success) {
        addToast({
          title: "Files Rematched",
          description: `Found ${match.MatchCount} match(es)`,
          color: "success",
        });
      } else {
        addToast({
          title: "Rematch Failed",
          description: match.Error || "Failed to rematch files",
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
        setTorrents((prev) =>
          prev.map((t) =>
            t.InfoHash === infoHash ? { ...t, State: "paused" } : t,
          ),
        );
      }
    }
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
        setTorrents((prev) =>
          prev.map((t) =>
            t.InfoHash === infoHash ? { ...t, State: "downloading" } : t,
          ),
        );
      }
    }
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
        setTorrents((prev) => prev.filter((t) => t.InfoHash !== infoHash));
      }
    }
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
          onPress={fetchTorrents}
          isLoading={isLoading}
        >
          Refresh
        </Button>
      </div>

      <TorrentTable
        torrents={torrents}
        isLoading={isLoading}
        onPause={handlePause}
        onResume={handleResume}
        onRemove={handleRemove}
        onInfo={handleInfo}
        onOrganize={handleOrganize}
        onProcess={handleProcess}
        onRematch={handleRematch}
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
        onLinked={fetchTorrents}
      />
    </div>
  );
}
