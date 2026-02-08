import { useMemo, useState, type ReactNode } from "react";
import { useQueryState, parseAsString } from "nuqs";
import { Progress } from "@heroui/progress";
import { Chip } from "@heroui/chip";
import { Button, ButtonGroup } from "@heroui/button";
import { Tooltip } from "@heroui/tooltip";
import { Skeleton } from "@heroui/skeleton";
import { useDisclosure } from "@heroui/modal";
import { ConfirmModal } from "../ConfirmModal";
import {
  DataTable,
  type DataTableColumn,
  type BulkAction,
  type RowAction,
} from "../data-table";
import type { DownloadTorrent } from "./types";
import { formatBytes, formatRelativeTime } from "../../lib/format";
import {
  IconPlayerPlay,
  IconPlayerPause,
  IconTrash,
  IconPlus,
  IconInfoCircle,
  IconFolder,
  IconLibrary,
  IconCopy,
  IconRefresh,
} from "@tabler/icons-react";
import { TorrentCard, TORRENT_STATE_INFO } from "./TorrentCard";

// ============================================================================
// Component Props
// ============================================================================

export interface TorrentTableProps {
  torrents: DownloadTorrent[];
  isLoading?: boolean;
  onPause: (infoHash: string) => void;
  onResume: (infoHash: string) => void;
  onRemove: (infoHash: string) => void;
  onInfo: (infoHash: string) => void;
  onOrganize: (infoHash: string) => void;
  onProcess: (torrent: DownloadTorrent) => void;
  onRematch: (torrent: DownloadTorrent) => void;
  onLinkToLibrary: (torrent: DownloadTorrent) => void;
  onBulkPause: (infoHashes: string[]) => void;
  onBulkResume: (infoHashes: string[]) => void;
  onBulkRemove: (infoHashes: string[]) => void;
  onAddClick: () => void;
  liveStatsByInfoHash?: Record<
    string,
    { downloadSpeed: number; uploadSpeed: number; peers: number }
  >;
}

// ============================================================================
// Main Component
// ============================================================================

// State filter options
interface StateFilterOption {
  key: string;
  label: string;
  color: "primary" | "success" | "warning" | "secondary" | "default" | "danger";
}

const STATE_FILTER_OPTIONS: StateFilterOption[] = [
  { key: "DOWNLOADING", label: "Downloading", color: "primary" },
  { key: "SEEDING", label: "Seeding", color: "success" },
  { key: "PAUSED", label: "Paused", color: "warning" },
  { key: "CHECKING", label: "Checking", color: "secondary" },
  { key: "QUEUED", label: "Queued", color: "default" },
  { key: "ERROR", label: "Error", color: "danger" },
];

export function TorrentTable({
  torrents,
  isLoading = false,
  onPause,
  onResume,
  onRemove,
  onInfo,
  onOrganize,
  onProcess,
  onRematch,
  onLinkToLibrary,
  onBulkPause,
  onBulkResume,
  onBulkRemove,
  onAddClick,
  liveStatsByInfoHash = {},
}: TorrentTableProps) {
  // Confirm modal state
  const {
    isOpen: isConfirmOpen,
    onOpen: onConfirmOpen,
    onClose: onConfirmClose,
  } = useDisclosure();
  const [torrentToRemove, setTorrentToRemove] =
    useState<DownloadTorrent | null>(null);

  // State filter - persisted in URL via nuqs
  const [stateFilter, setStateFilter] = useQueryState(
    "state",
    parseAsString.withDefault(""),
  );
  const normalizedStateFilter = stateFilter === "" ? null : stateFilter;

  // Calculate state counts for filter badges
  const stateCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const t of torrents) {
      const state = t.State.toUpperCase();
      counts[state] = (counts[state] || 0) + 1;
    }
    return counts;
  }, [torrents]);

  // Filter torrents by state
  const filteredTorrents = useMemo(() => {
    if (!normalizedStateFilter) return torrents;
    return torrents.filter(
      (t) => t.State.toUpperCase() === normalizedStateFilter,
    );
  }, [torrents, normalizedStateFilter]);

  // Column definitions with skeleton support
  const columns: DataTableColumn<DownloadTorrent>[] = useMemo(
    () => [
      {
        key: "Name",
        label: "NAME",
        sortable: true,
        skeleton: () => (
          <div className="flex flex-col gap-1">
            <Skeleton className="w-full h-4 rounded" />
          </div>
        ),
        render: (torrent) => (
          <div className="flex flex-col gap-1 min-w-0">
            <button
              type="button"
              className="font-medium truncate text-left hover:text-primary transition-colors"
              title={torrent.Name}
              onClick={() => onInfo(torrent.InfoHash)}
            >
              {torrent.Name}
            </button>
          </div>
        ),
        sortFn: (a, b) => a.Name.localeCompare(b.Name),
      },
      {
        key: "Progress",
        label: "PROGRESS",
        width: 300,
        sortable: true,
        skeleton: () => (
          <div className="flex flex-row gap-4 items-center">
            <Skeleton className="w-full h-4 rounded" />
            <Skeleton className="w-10 h-3 rounded" />
          </div>
        ),
        render: (torrent) => {
          const state = torrent.State.toUpperCase();
          return (
            <div className="flex flex-row gap-4 items-center">
              <Progress
                value={torrent.Progress * 100}
                color={
                  state === "SEEDING"
                    ? "success"
                    : state === "ERROR"
                      ? "danger"
                      : state === "PAUSED"
                        ? "warning"
                        : "primary"
                }
                size="md"
                aria-label="Download progress"
              />
              <span className="text-xs text-default-500 tabular-nums">
                {(torrent.Progress * 100).toFixed(1)}%
              </span>
            </div>
          );
        },
        sortFn: (a, b) => a.Progress - b.Progress,
      },
      {
        key: "TotalBytes",
        label: "SIZE",
        width: 100,
        sortable: true,
        skeleton: () => <Skeleton className="w-16 h-4 rounded" />,
        render: (torrent) => (
          <span className="text-sm tabular-nums">
            {formatBytes(torrent.TotalBytes)}
          </span>
        ),
        sortFn: (a, b) => a.TotalBytes - b.TotalBytes,
      },
      {
        key: "liveStats",
        label: "SPEED / PEERS",
        width: 180,
        sortable: false,
        skeleton: () => <Skeleton className="w-24 h-4 rounded" />,
        render: (torrent) => {
          const stats = liveStatsByInfoHash[torrent.InfoHash];
          if (!stats) {
            return <span className="text-xs text-default-400">-</span>;
          }
          const down =
            stats.downloadSpeed > 0
              ? `${formatBytes(stats.downloadSpeed)}/s`
              : "-";
          const up =
            stats.uploadSpeed > 0 ? `${formatBytes(stats.uploadSpeed)}/s` : "-";
          return (
            <div className="flex flex-col text-xs text-default-500 tabular-nums">
              <span>
                ↓ {down} ↑ {up}
              </span>
              <span>{stats.peers} peers</span>
            </div>
          );
        },
      },
      {
        key: "State",
        label: "STATUS",
        width: 120,
        sortable: true,
        skeleton: () => <Skeleton className="w-20 h-5 rounded-full" />,
        render: (torrent) => {
          const state = torrent.State.toUpperCase();
          const stateInfo =
            TORRENT_STATE_INFO[state as keyof typeof TORRENT_STATE_INFO];
          return (
            <Chip
              size="sm"
              variant="flat"
              color={stateInfo?.color ?? "default"}
            >
              {stateInfo?.label ?? state}
            </Chip>
          );
        },
        sortFn: (a, b) => a.State.localeCompare(b.State),
      },
      {
        key: "AddedAt",
        label: "ADDED",
        width: { width: 100, minWidth: 80 },
        sortable: true,
        truncate: false,
        skeleton: () => <Skeleton className="w-16 h-4 rounded" />,
        render: (torrent) => (
          <Tooltip
            content={
              torrent.AddedAt
                ? new Date(torrent.AddedAt).toLocaleString()
                : "Unknown"
            }
          >
            <span className="text-xs text-default-500 whitespace-nowrap">
              {formatRelativeTime(torrent.AddedAt)}
            </span>
          </Tooltip>
        ),
        sortFn: (a, b) => {
          const aTime = a.AddedAt ? new Date(a.AddedAt).getTime() : 0;
          const bTime = b.AddedAt ? new Date(b.AddedAt).getTime() : 0;
          return bTime - aTime; // Most recent first
        },
      },
    ],
    [onInfo],
  );

  // Filter row content with state filter chips
  const filterRowContent: ReactNode = useMemo(
    () => (
      <ButtonGroup size="sm" variant="solid">
        <Button
          variant={normalizedStateFilter === null ? "solid" : "flat"}
          color={normalizedStateFilter === null ? "primary" : "default"}
          onPress={() => setStateFilter("")}
        >
          All ({torrents.length})
        </Button>
        {STATE_FILTER_OPTIONS.map((option) => {
          const count = stateCounts[option.key] || 0;
          if (count === 0) return null;
          return (
            <Button
              key={option.key}
              variant={normalizedStateFilter === option.key ? "solid" : "flat"}
              color={
                normalizedStateFilter === option.key ? option.color : "default"
              }
              onPress={() =>
                setStateFilter(
                  normalizedStateFilter === option.key ? "" : option.key,
                )
              }
              className="gap-1"
            >
              <span>{option.label}</span>
              <Chip size="sm" variant="flat" className="ml-1">
                {count}
              </Chip>
            </Button>
          );
        })}
      </ButtonGroup>
    ),
    [normalizedStateFilter, stateCounts, torrents.length, setStateFilter],
  );

  // Bulk actions
  const bulkActions: BulkAction<DownloadTorrent>[] = useMemo(
    () => [
      {
        key: "resume",
        label: "Resume",
        icon: <IconPlayerPlay size={16} className="text-green-400" />,
        color: "success",
        onAction: (items) => onBulkResume(items.map((t) => t.InfoHash)),
      },
      {
        key: "pause",
        label: "Pause",
        icon: <IconPlayerPause size={16} className="text-amber-400" />,
        color: "warning",
        onAction: (items) => onBulkPause(items.map((t) => t.InfoHash)),
      },
      {
        key: "remove",
        label: "Remove",
        icon: <IconTrash size={16} className="text-red-400" />,
        color: "danger",
        isDestructive: true,
        confirm: true,
        confirmMessage: "Remove selected torrents?",
        onAction: (items) => onBulkRemove(items.map((t) => t.InfoHash)),
      },
    ],
    [onBulkPause, onBulkResume, onBulkRemove],
  );

  // Row actions
  const rowActions: RowAction<DownloadTorrent>[] = useMemo(
    () => [
      {
        key: "resume",
        label: "Resume",
        icon: <IconPlayerPlay size={16} className="text-green-400" />,
        color: "success",
        inDropdown: false,
        isVisible: (torrent) => torrent.State.toUpperCase() === "PAUSED",
        onAction: (torrent) => onResume(torrent.InfoHash),
      },
      {
        key: "pause",
        label: "Pause",
        icon: <IconPlayerPause size={16} className="text-amber-400" />,
        color: "warning",
        inDropdown: false,
        isVisible: (torrent) => {
          const state = torrent.State.toUpperCase();
          return state === "DOWNLOADING" || state === "SEEDING";
        },
        onAction: (torrent) => onPause(torrent.InfoHash),
      },
      {
        key: "info",
        label: "Info",
        icon: <IconInfoCircle size={16} />,
        inDropdown: true,
        onAction: (torrent) => onInfo(torrent.InfoHash),
      },
      {
        key: "process",
        label: "Process",
        icon: <IconCopy size={16} className="text-green-400" />,
        inDropdown: true,
        isVisible: (torrent) => {
          const state = torrent.State.toUpperCase();
          return state === "SEEDING" || torrent.Progress >= 1;
        },
        onAction: (torrent) => onProcess(torrent),
      },
      {
        key: "rematch",
        label: "Rematch",
        icon: <IconRefresh size={16} className="text-blue-400" />,
        inDropdown: true,
        isVisible: (torrent) => {
          const state = torrent.State.toUpperCase();
          return state === "SEEDING" || torrent.Progress >= 1;
        },
        onAction: (torrent) => onRematch(torrent),
      },
      {
        key: "organize",
        label: "Organize (Legacy)",
        icon: <IconFolder size={16} className="text-amber-400" />,
        inDropdown: true,
        isVisible: (torrent) => {
          const state = torrent.State.toUpperCase();
          return state === "SEEDING" || torrent.Progress >= 1;
        },
        onAction: (torrent) => onOrganize(torrent.InfoHash),
      },
      {
        key: "link-library",
        label: "Link to Library",
        icon: <IconLibrary size={16} className="text-blue-400" />,
        inDropdown: true,
        isVisible: (torrent) => {
          const state = torrent.State.toUpperCase();
          return state === "SEEDING" || torrent.Progress >= 1;
        },
        onAction: (torrent) => onLinkToLibrary(torrent),
      },
      {
        key: "remove",
        label: "Remove",
        icon: <IconTrash size={16} className="text-red-400" />,
        isDestructive: true,
        inDropdown: true,
        onAction: (torrent) => {
          setTorrentToRemove(torrent);
          onConfirmOpen();
        },
      },
    ],
    [
      onPause,
      onResume,
      onInfo,
      onOrganize,
      onProcess,
      onRematch,
      onLinkToLibrary,
      onConfirmOpen,
    ],
  );

  // Custom search function
  const searchFn = (torrent: DownloadTorrent, term: string) => {
    const lowerTerm = term.toLowerCase();
    return (
      torrent.Name.toLowerCase().includes(lowerTerm) ||
      torrent.InfoHash.toLowerCase().includes(lowerTerm)
    );
  };

  // Empty content - simpler message inside the table
  const emptyContent = (
    <div className="py-8 text-center">
      <p className="text-default-500 mb-2">No active downloads</p>
      <p className="text-xs text-default-400">
        Click the + button above to add a torrent
      </p>
    </div>
  );

  // Footer content - total size

  return (
    <>
      <DataTable
        stateKey="torrents"
        skeletonDelay={500}
        data={filteredTorrents}
        columns={columns}
        getRowKey={(torrent) => torrent.Id}
        isLoading={isLoading}
        skeletonRowCount={12}
        selectionMode="multiple"
        checkboxSelectionOnly
        searchFn={searchFn}
        searchPlaceholder="Search torrents..."
        defaultSortColumn="Name"
        fillHeight
        showViewModeToggle
        defaultViewMode="table"
        cardRenderer={({ item }) => (
          <TorrentCard
            torrent={item}
            onPause={onPause}
            onResume={onResume}
            onRemove={onRemove}
            showCheckboxSpace
          />
        )}
        cardGridClassName="grid grid-cols-1 lg:grid-cols-2 gap-4"
        bulkActions={bulkActions}
        rowActions={rowActions}
        emptyContent={emptyContent}
        ariaLabel="Torrents table"
        filterRowContent={filterRowContent}
        toolbarContent={
          <Tooltip content="Add Torrent">
            <Button isIconOnly color="primary" size="sm" onPress={onAddClick}>
              <IconPlus size={16} />
            </Button>
          </Tooltip>
        }
        toolbarContentPosition="end"
      />

      <ConfirmModal
        isOpen={isConfirmOpen}
        onClose={onConfirmClose}
        onConfirm={() => {
          if (torrentToRemove) {
            onRemove(torrentToRemove.InfoHash);
          }
          onConfirmClose();
        }}
        title="Remove Torrent"
        message={`Are you sure you want to remove "${torrentToRemove?.Name}"?`}
        description="This will stop the download but will not delete any downloaded files."
        confirmLabel="Remove"
        confirmColor="danger"
      />
    </>
  );
}
