import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Progress } from '@heroui/progress'
import { Chip } from '@heroui/chip'
import { Tooltip } from '@heroui/tooltip'
import { useDisclosure } from '@heroui/modal'
import type { Torrent, TorrentState } from '../../lib/graphql'
import { formatBytes, formatSpeed } from '../../lib/format'
import { IconPlayerPlay, IconPlayerPause, IconTrash, IconArrowDown, IconArrowUp } from '@tabler/icons-react'
import { ConfirmModal } from '../ConfirmModal'

// ============================================================================
// State Configuration
// ============================================================================

export const TORRENT_STATE_INFO: Record<
  TorrentState,
  { label: string; color: 'default' | 'primary' | 'success' | 'warning' | 'danger' | 'secondary' }
> = {
  QUEUED: { label: 'Queued', color: 'default' },
  CHECKING: { label: 'Checking', color: 'secondary' },
  DOWNLOADING: { label: 'Downloading', color: 'primary' },
  SEEDING: { label: 'Seeding', color: 'success' },
  PAUSED: { label: 'Paused', color: 'warning' },
  ERROR: { label: 'Error', color: 'danger' },
}

// ============================================================================
// Component Props
// ============================================================================

export interface TorrentCardProps {
  torrent: Torrent
  onPause: (id: number) => void
  onResume: (id: number) => void
  onRemove: (id: number) => void
  /** Whether to show checkbox space on the left (for alignment with DataTable) */
  showCheckboxSpace?: boolean
}

// ============================================================================
// Component
// ============================================================================

export function TorrentCard({
  torrent,
  onPause,
  onResume,
  onRemove,
  showCheckboxSpace = false,
}: TorrentCardProps) {
  const isPaused = torrent.state === 'PAUSED'
  const isSeeding = torrent.state === 'SEEDING'
  const isError = torrent.state === 'ERROR'
  const isDownloading = torrent.state === 'DOWNLOADING'

  const progressColor = isSeeding
    ? 'success'
    : isError
      ? 'danger'
      : isPaused
        ? 'warning'
        : 'primary'

  const stateInfo = TORRENT_STATE_INFO[torrent.state] || TORRENT_STATE_INFO.QUEUED

  // Confirm modal state
  const { isOpen: isConfirmOpen, onOpen: onConfirmOpen, onClose: onConfirmClose } = useDisclosure()

  const handleRemove = () => {
    onConfirmOpen()
  }

  return (
    <>
    <ConfirmModal
      isOpen={isConfirmOpen}
      onClose={onConfirmClose}
      onConfirm={() => {
        onRemove(torrent.id)
        onConfirmClose()
      }}
      title="Remove Torrent"
      message={`Are you sure you want to remove "${torrent.name}"?`}
      description="This will stop the download but will not delete any downloaded files."
      confirmLabel="Remove"
      confirmColor="danger"
    />
    <Card>
      <CardBody>
        <div className="flex items-start justify-between mb-3">
          <div className={`flex-1 min-w-0 mr-4 ${showCheckboxSpace ? 'ml-8' : ''}`}>
            <h3 className="font-semibold truncate" title={torrent.name}>
              {torrent.name}
            </h3>
            <div className="flex items-center gap-2 mt-1">
              <span className="text-sm text-default-500">
                {torrent.sizeFormatted || formatBytes(torrent.size)}
              </span>
              <Chip size="sm" color={stateInfo.color} variant="flat">
                {stateInfo.label}
              </Chip>
            </div>
          </div>
          <div className="flex gap-1">
            {isPaused ? (
              <Tooltip content="Resume">
                <Button
                  isIconOnly
                  size="sm"
                  variant="light"
                  color="success"
                  onPress={() => onResume(torrent.id)}
                  aria-label="Resume torrent"
                >
                  <IconPlayerPlay size={16} />
                </Button>
              </Tooltip>
            ) : isDownloading ? (
              <Tooltip content="Pause">
                <Button
                  isIconOnly
                  size="sm"
                  variant="light"
                  color="warning"
                  onPress={() => onPause(torrent.id)}
                  aria-label="Pause torrent"
                >
                  <IconPlayerPause size={16} />
                </Button>
              </Tooltip>
            ) : null}
            <Tooltip content="Remove">
              <Button
                isIconOnly
                size="sm"
                variant="light"
                color="danger"
                onPress={handleRemove}
                aria-label="Remove torrent"
              >
                <IconTrash size={16} />
              </Button>
            </Tooltip>
          </div>
        </div>

        <Progress
          value={torrent.progress * 100}
          color={progressColor}
          size="md"
          className="mb-2"
          aria-label="Download progress"
        />

        <div className="flex justify-between text-sm text-default-500">
          <span>{(torrent.progress * 100).toFixed(1)}%</span>
          <span>
            {isDownloading && (
              <span className="flex items-center gap-1">
                <IconArrowDown size={12} className="text-blue-400" /> {formatSpeed(torrent.downloadSpeed)} • <IconArrowUp size={12} className="text-green-400" /> {formatSpeed(torrent.uploadSpeed)}
                {torrent.peers > 0 && ` • ${torrent.peers} peers`}
              </span>
            )}
            {isSeeding && (
              <span className="flex items-center gap-1">
                <IconArrowUp size={12} className="text-green-400" /> {formatSpeed(torrent.uploadSpeed)}
                {torrent.peers > 0 && ` • ${torrent.peers} peers`}
              </span>
            )}
          </span>
        </div>
      </CardBody>
    </Card>
    </>
  )
}
