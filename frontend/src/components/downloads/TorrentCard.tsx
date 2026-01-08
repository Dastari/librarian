import { Button, Card, CardBody, Progress, Chip, Tooltip } from '@heroui/react'
import type { Torrent } from '../../lib/graphql'

function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let unitIndex = 0
  let size = bytes
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }
  return `${size.toFixed(1)} ${units[unitIndex]}`
}

function formatSpeed(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`
}

export interface TorrentCardProps {
  torrent: Torrent
  onPause: () => void
  onResume: () => void
  onRemove: () => void
}

export function TorrentCard({ torrent, onPause, onResume, onRemove }: TorrentCardProps) {
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

  const stateLabels: Record<string, { label: string; color: 'default' | 'primary' | 'success' | 'warning' | 'danger' }> = {
    QUEUED: { label: 'Queued', color: 'default' },
    CHECKING: { label: 'Checking', color: 'primary' },
    DOWNLOADING: { label: 'Downloading', color: 'primary' },
    SEEDING: { label: 'Seeding', color: 'success' },
    PAUSED: { label: 'Paused', color: 'warning' },
    ERROR: { label: 'Error', color: 'danger' },
  }

  const stateInfo = stateLabels[torrent.state] || stateLabels.QUEUED

  return (
    <Card>
      <CardBody>
        <div className="flex items-start justify-between mb-3">
          <div className="flex-1 min-w-0 mr-4">
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
                <Button isIconOnly size="sm" variant="light" color="success" onPress={onResume}>
                  ▶️
                </Button>
              </Tooltip>
            ) : isDownloading ? (
              <Tooltip content="Pause">
                <Button isIconOnly size="sm" variant="light" color="warning" onPress={onPause}>
                  ⏸️
                </Button>
              </Tooltip>
            ) : null}
            <Tooltip content="Remove">
              <Button isIconOnly size="sm" variant="light" color="danger" onPress={onRemove}>
                🗑️
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
              <>
                ⬇️ {formatSpeed(torrent.downloadSpeed)} • ⬆️ {formatSpeed(torrent.uploadSpeed)}
                {torrent.peers > 0 && ` • ${torrent.peers} peers`}
              </>
            )}
            {isSeeding && (
              <>
                ⬆️ {formatSpeed(torrent.uploadSpeed)}
                {torrent.peers > 0 && ` • ${torrent.peers} peers`}
              </>
            )}
          </span>
        </div>
      </CardBody>
    </Card>
  )
}
