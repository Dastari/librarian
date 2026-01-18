import { useMemo, useState } from 'react'
import { Progress } from '@heroui/progress'
import { Chip } from '@heroui/chip'
import { Button } from '@heroui/button'
import { Tooltip } from '@heroui/tooltip'
import { Skeleton } from '@heroui/skeleton'
import { useDisclosure } from '@heroui/modal'
import { ConfirmModal } from '../ConfirmModal'
import {
  DataTable,
  type DataTableColumn,
  type DataTableFilter,
  type FilterOption,
  type BulkAction,
  type RowAction,
} from '../data-table'
import type { Torrent } from '../../lib/graphql'
import { formatBytes, formatSpeed, formatEta, formatRelativeTime } from '../../lib/format'
import { IconPlayerPlay, IconPlayerPause, IconTrash, IconPlus, IconInfoCircle, IconFolder, IconArrowDown, IconArrowUp } from '@tabler/icons-react'
import { TorrentCard, TORRENT_STATE_INFO } from './TorrentCard'

// ============================================================================
// Component Props
// ============================================================================

export interface TorrentTableProps {
  torrents: Torrent[]
  isLoading?: boolean
  onPause: (id: number) => void
  onResume: (id: number) => void
  onRemove: (id: number) => void
  onInfo: (id: number) => void
  onOrganize: (id: number) => void
  onBulkPause: (ids: number[]) => void
  onBulkResume: (ids: number[]) => void
  onBulkRemove: (ids: number[]) => void
  onAddClick: () => void
}

// ============================================================================
// Main Component
// ============================================================================

export function TorrentTable({
  torrents,
  isLoading = false,
  onPause,
  onResume,
  onRemove,
  onInfo,
  onOrganize,
  onBulkPause,
  onBulkResume,
  onBulkRemove,
  onAddClick,
}: TorrentTableProps) {
  // Confirm modal state
  const { isOpen: isConfirmOpen, onOpen: onConfirmOpen, onClose: onConfirmClose } = useDisclosure()
  const [torrentToRemove, setTorrentToRemove] = useState<Torrent | null>(null)

  // Calculate state counts for filter badges
  const stateCounts = useMemo(() => {
    const counts: Record<string, number> = {}
    for (const t of torrents) {
      counts[t.state] = (counts[t.state] || 0) + 1
    }
    return counts
  }, [torrents])

  // Column definitions with skeleton support
  const columns: DataTableColumn<Torrent>[] = useMemo(
    () => [
      {
        key: 'name',
        label: 'NAME',
        sortable: true,
        skeleton: () => (
          <div className="flex flex-col gap-1">
            <Skeleton className="w-full h-4 rounded" />
          </div>
        ),
        render: (torrent) => (
          <div className="flex flex-col gap-1 min-w-0">
            <span className="font-medium truncate" title={torrent.name}>
              {torrent.name}
            </span>
          </div>
        ),
        sortFn: (a, b) => a.name.localeCompare(b.name),
      },
      {
        key: 'progress',
        label: 'PROGRESS',
        width: 300,
        sortable: true,
        skeleton: () => (
          <div className="flex flex-row gap-4 items-center">
            <Skeleton className="w-full h-4 rounded" />
            <Skeleton className="w-10 h-3 rounded" />
          </div>
        ),
        render: (torrent) => (
          <div className="flex flex-row gap-4 items-center">
            <Progress
              value={torrent.progress * 100}
              color={
                torrent.state === 'SEEDING'
                  ? 'success'
                  : torrent.state === 'ERROR'
                    ? 'danger'
                    : torrent.state === 'PAUSED'
                      ? 'warning'
                      : 'primary'
              }
              size="md"
              aria-label="Download progress"
            />
            <span className="text-xs text-default-500 tabular-nums">
              {(torrent.progress * 100).toFixed(1)}%
            </span>
          </div>
        ),
        sortFn: (a, b) => a.progress - b.progress,
      },
      {
        key: 'size',
        label: 'SIZE',
        width: 100,
        sortable: true,
        skeleton: () => <Skeleton className="w-16 h-4 rounded" />,
        render: (torrent) => (
          <span className="text-sm tabular-nums">
            {torrent.sizeFormatted || formatBytes(torrent.size)}
          </span>
        ),
        sortFn: (a, b) => (a.size || 0) - (b.size || 0),
      },
      {
        key: 'speed',
        label: 'SPEED',
        width: 120,
        sortable: true,
        skeleton: () => (
          <div className="flex flex-col gap-1.5 pt-1">
            <Skeleton className="w-20 h-3 rounded" />
            <Skeleton className="w-20 h-3 rounded" />
            <Skeleton className="w-16 h-3 rounded" />
          </div>
        ),
        render: (torrent) => (
          <div className="flex flex-col gap-0.5 text-xs tabular-nums">
            {(torrent.state === 'DOWNLOADING' || torrent.state === 'SEEDING') && (
              <>
                <span className="text-primary flex items-center gap-1">
                  <IconArrowDown size={12} className="text-blue-400" /> {torrent.downloadSpeedFormatted || formatSpeed(torrent.downloadSpeed)}
                </span>
                <span className="text-success flex items-center gap-1">
                  <IconArrowUp size={12} className="text-green-400" /> {torrent.uploadSpeedFormatted || formatSpeed(torrent.uploadSpeed)}
                </span>
              </>
            )}
            {torrent.peers > 0 ? (
              <span className="text-default-400">{torrent.peers} peers</span>
            ) : (
              <span className="text-default-400">&nbsp;</span>
            )}
          </div>
        ),
        sortFn: (a, b) => (a.downloadSpeed || 0) - (b.downloadSpeed || 0),
      },
      {
        key: 'state',
        label: 'STATUS',
        width: 120,
        sortable: true,
        skeleton: () => <Skeleton className="w-20 h-5 rounded-full" />,
        render: (torrent) => (
          <Chip
            size="sm"
            color={TORRENT_STATE_INFO[torrent.state]?.color || 'default'}
            variant="flat"
          >
            {TORRENT_STATE_INFO[torrent.state]?.label || torrent.state}
          </Chip>
        ),
        sortFn: (a, b) => a.state.localeCompare(b.state),
      },
      {
        key: 'eta',
        label: 'ETA',
        width: 80,
        sortable: true,
        skeleton: () => <Skeleton className="w-12 h-4 rounded" />,
        render: (torrent) => (
          <span className="text-sm text-default-500 tabular-nums">
            {torrent.state === 'DOWNLOADING' ? formatEta(torrent.eta) : '—'}
          </span>
        ),
        sortFn: (a, b) => (a.eta || Infinity) - (b.eta || Infinity),
      },
      {
        key: 'addedAt',
        label: 'ADDED',
        width: { width: 100, minWidth: 80 },
        sortable: true,
        truncate: false,
        skeleton: () => <Skeleton className="w-16 h-4 rounded" />,
        render: (torrent) => (
          <Tooltip content={torrent.addedAt ? new Date(torrent.addedAt).toLocaleString() : 'Unknown'}>
            <span className="text-xs text-default-500 whitespace-nowrap">
              {formatRelativeTime(torrent.addedAt)}
            </span>
          </Tooltip>
        ),
        sortFn: (a, b) => {
          const aTime = a.addedAt ? new Date(a.addedAt).getTime() : 0
          const bTime = b.addedAt ? new Date(b.addedAt).getTime() : 0
          return bTime - aTime // Most recent first
        },
      },
    ],
    []
  )

  // Filter options with counts
  const stateFilterOptions: FilterOption[] = useMemo(
    () => [
      { key: 'DOWNLOADING', label: 'Downloading', color: 'primary', count: stateCounts['DOWNLOADING'] || 0 },
      { key: 'SEEDING', label: 'Seeding', color: 'success', count: stateCounts['SEEDING'] || 0 },
      { key: 'PAUSED', label: 'Paused', color: 'warning', count: stateCounts['PAUSED'] || 0 },
      { key: 'CHECKING', label: 'Checking', color: 'secondary', count: stateCounts['CHECKING'] || 0 },
      { key: 'QUEUED', label: 'Queued', color: 'default', count: stateCounts['QUEUED'] || 0 },
      { key: 'ERROR', label: 'Error', color: 'danger', count: stateCounts['ERROR'] || 0 },
    ],
    [stateCounts]
  )

  // Filter definitions
  const filters: DataTableFilter<Torrent>[] = useMemo(
    () => [
      {
        key: 'state',
        label: 'Status',
        type: 'select',
        options: stateFilterOptions,
        filterFn: (torrent, value) => {
          if (!value) return true
          return torrent.state === value
        },
        position: 'toolbar',
      },
    ],
    [stateFilterOptions]
  )

  // Bulk actions
  const bulkActions: BulkAction<Torrent>[] = useMemo(
    () => [
      {
        key: 'resume',
        label: 'Resume',
        icon: <IconPlayerPlay size={16} className="text-green-400" />,
        color: 'success',
        onAction: (items) => onBulkResume(items.map((t) => t.id)),
      },
      {
        key: 'pause',
        label: 'Pause',
        icon: <IconPlayerPause size={16} className="text-amber-400" />,
        color: 'warning',
        onAction: (items) => onBulkPause(items.map((t) => t.id)),
      },
      {
        key: 'remove',
        label: 'Remove',
        icon: <IconTrash size={16} className="text-red-400" />,
        color: 'danger',
        isDestructive: true,
        confirm: true,
        confirmMessage: 'Remove selected torrents?',
        onAction: (items) => onBulkRemove(items.map((t) => t.id)),
      },
    ],
    [onBulkPause, onBulkResume, onBulkRemove]
  )

  // Row actions
  const rowActions: RowAction<Torrent>[] = useMemo(
    () => [
      {
        key: 'resume',
        label: 'Resume',
        icon: <IconPlayerPlay size={16} className="text-green-400" />,
        color: 'success',
        inDropdown: false,
        isVisible: (torrent) => torrent.state === 'PAUSED',
        onAction: (torrent) => onResume(torrent.id),
      },
      {
        key: 'pause',
        label: 'Pause',
        icon: <IconPlayerPause size={16} className="text-amber-400" />,
        color: 'warning',
        inDropdown: false,
        isVisible: (torrent) => torrent.state === 'DOWNLOADING' || torrent.state === 'SEEDING',
        onAction: (torrent) => onPause(torrent.id),
      },
      {
        key: 'info',
        label: 'Info',
        icon: <IconInfoCircle size={16} />,
        inDropdown: true,
        onAction: (torrent) => onInfo(torrent.id),
      },
      {
        key: 'organize',
        label: 'Organize',
        icon: <IconFolder size={16} className="text-amber-400" />,
        inDropdown: true,
        isVisible: (torrent) => torrent.state === 'SEEDING' || torrent.progress >= 1,
        onAction: (torrent) => onOrganize(torrent.id),
      },
      {
        key: 'remove',
        label: 'Remove',
        icon: <IconTrash size={16} className="text-red-400" />,
        isDestructive: true,
        inDropdown: true,
        onAction: (torrent) => {
          setTorrentToRemove(torrent)
          onConfirmOpen()
        },
      },
    ],
    [onPause, onResume, onInfo, onOrganize, onConfirmOpen]
  )

  // Custom search function
  const searchFn = (torrent: Torrent, term: string) => {
    const lowerTerm = term.toLowerCase()
    return (
      torrent.name.toLowerCase().includes(lowerTerm) ||
      torrent.infoHash.toLowerCase().includes(lowerTerm)
    )
  }

  // Empty content - simpler message inside the table
  const emptyContent = (
    <div className="py-8 text-center">
      <p className="text-default-500 mb-2">No active downloads</p>
      <p className="text-xs text-default-400">
        Click the + button above to add a torrent
      </p>
    </div>
  )

  // Footer content - total size

  return (
    <>
    <DataTable
      stateKey="torrents"
      data={torrents}
      columns={columns}
      getRowKey={(torrent) => torrent.id}
      isLoading={isLoading}
      skeletonRowCount={12}
      selectionMode="multiple"
      filters={filters}
      searchFn={searchFn}
      searchPlaceholder="Search torrents..."
      defaultSortColumn="name"
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
          onRemove(torrentToRemove.id)
        }
        onConfirmClose()
      }}
      title="Remove Torrent"
      message={`Are you sure you want to remove "${torrentToRemove?.name}"?`}
      description="This will stop the download but will not delete any downloaded files."
      confirmLabel="Remove"
      confirmColor="danger"
    />
    </>
  )
}
