import { useState, useEffect, useMemo } from 'react'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'
import { Spinner } from '@heroui/spinner'
import { Progress } from '@heroui/progress'
import { Chip } from '@heroui/chip'
import { Card, CardBody } from '@heroui/card'
import { Tooltip } from '@heroui/tooltip'
import {
  graphqlClient,
  TORRENT_DETAILS_QUERY,
  type TorrentDetails,
  type TorrentFileInfo,
} from '../../lib/graphql'
import { formatBytes } from '../../lib/format'
import { TORRENT_STATE_INFO } from './TorrentCard'
import { DataTable, type DataTableColumn } from '../data-table'

interface TorrentInfoModalProps {
  torrentId: number | null
  isOpen: boolean
  onClose: () => void
}

// File columns for the DataTable
const fileColumns: DataTableColumn<TorrentFileInfo>[] = [
  {
    key: 'path',
    label: 'File',
    render: (file) => {
      const fileName = file.path.split('/').pop() || file.path
      const directory = file.path.includes('/') 
        ? file.path.substring(0, file.path.lastIndexOf('/'))
        : null
      return (
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
        </div>
      )
    },
  },
  {
    key: 'size',
    label: 'Size',
    width: 100,
    align: 'end',
    render: (file) => (
      <span className="text-sm tabular-nums text-default-500">
        {formatBytes(file.size)}
      </span>
    ),
    sortFn: (a, b) => a.size - b.size,
  },
  {
    key: 'progress',
    label: 'Progress',
    width: 160,
    align: 'center',
    render: (file) => (
      <div className="flex items-center gap-2">
        <Progress
          value={file.progress * 100}
          size="sm"
          color={file.progress >= 1 ? 'success' : 'primary'}
          className="w-20"
          aria-label={`${file.path} progress`}
        />
        <span className={`text-xs tabular-nums w-10 text-right ${file.progress >= 1 ? 'text-success' : 'text-default-500'}`}>
          {(file.progress * 100).toFixed(0)}%
        </span>
      </div>
    ),
    sortFn: (a, b) => a.progress - b.progress,
  },
]

export function TorrentInfoModal({ torrentId, isOpen, onClose }: TorrentInfoModalProps) {
  const [details, setDetails] = useState<TorrentDetails | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (isOpen && torrentId !== null) {
      setIsLoading(true)
      setError(null)
      graphqlClient
        .query<{ torrentDetails: TorrentDetails }>(TORRENT_DETAILS_QUERY, { id: torrentId })
        .toPromise()
        .then((result) => {
          if (result.data?.torrentDetails) {
            setDetails(result.data.torrentDetails)
          } else if (result.error) {
            setError(result.error.message)
          } else {
            setError('Torrent not found')
          }
        })
        .catch((err) => setError(err.message))
        .finally(() => setIsLoading(false))
    }
  }, [isOpen, torrentId])

  // Memoize completed files count
  const completedFilesCount = useMemo(() => {
    if (!details?.files) return 0
    return details.files.filter((f) => f.progress >= 1).length
  }, [details?.files])

  return (
    <Modal 
      isOpen={isOpen} 
      onClose={onClose} 
      size="5xl" 
      scrollBehavior="inside"
      classNames={{
        wrapper: 'overflow-hidden',
        base: 'max-h-[90vh]',
        header: 'border-b border-default-100',
        footer: 'border-t border-default-100',
      }}
    >
      <ModalContent>
        <ModalHeader className="flex flex-col gap-2 pb-4">
          <div className="flex items-start justify-between gap-4">
            <div className="min-w-0 flex-1">
              <h2 className="text-xl font-semibold truncate pr-4">
                {details?.name || 'Torrent Details'}
              </h2>
              {details && (
                <code className="text-xs text-default-400 font-mono mt-1 block">
                  {details.infoHash}
                </code>
              )}
            </div>
            {details && (
              <div className="flex items-center gap-2 flex-shrink-0">
                <Chip
                  size="sm"
                  color={TORRENT_STATE_INFO[details.state]?.color || 'default'}
                  variant="flat"
                >
                  {TORRENT_STATE_INFO[details.state]?.label || details.state}
                </Chip>
                {details.finished && (
                  <Chip size="sm" color="success" variant="flat" startContent="✓">
                    Complete
                  </Chip>
                )}
              </div>
            )}
          </div>
        </ModalHeader>

        <ModalBody className="py-6">
          {isLoading && (
            <div className="flex flex-col items-center justify-center py-16 gap-3">
              <Spinner size="lg" />
              <span className="text-default-500 text-sm">Loading torrent details...</span>
            </div>
          )}

          {error && (
            <div className="flex flex-col items-center justify-center py-16 gap-3">
              <div className="text-4xl">⚠️</div>
              <div className="text-danger text-center">{error}</div>
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
                      color={details.state === 'ERROR' ? 'danger' : details.finished ? 'success' : 'primary'}
                      size="md"
                      aria-label="Download progress"
                      classNames={{
                        track: 'h-3',
                        indicator: 'h-3',
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
                  subtitle={details.timeRemainingFormatted ? `ETA: ${details.timeRemainingFormatted}` : undefined}
                  icon="⬇️"
                  valueColor="primary"
                />
                <StatCard 
                  title="Upload" 
                  value={details.uploadSpeedFormatted} 
                  subtitle={`Ratio: ${details.ratio.toFixed(2)}`}
                  icon="⬆️"
                  valueColor={details.ratio >= 1 ? 'success' : undefined}
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
                  subtitle={details.averagePieceDownloadMs ? `Avg: ${details.averagePieceDownloadMs}ms` : undefined}
                  icon="🧩"
                />
              </div>

              {/* Detailed Stats Row */}
              <div className="grid grid-cols-3 lg:grid-cols-6 gap-3 text-sm">
                <MiniStat label="Downloaded" value={details.downloadedFormatted} />
                <MiniStat label="Uploaded" value={details.uploadedFormatted} />
                <MiniStat label="Peers Queued" value={details.peerStats.queued.toString()} />
                <MiniStat label="Peers Seen" value={details.peerStats.seen.toString()} />
                <MiniStat label="Peers Dead" value={details.peerStats.dead.toString()} color="danger" />
                <MiniStat label="Not Needed" value={details.peerStats.notNeeded.toString()} />
              </div>

              {/* Save Path */}
              <div className="bg-content2/30 rounded-lg p-3">
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-sm">📁</span>
                  <span className="text-xs font-medium text-default-500 uppercase tracking-wide">Save Location</span>
                </div>
                <code className="text-sm text-default-600 break-all">
                  {details.savePath}
                </code>
              </div>

              {/* Files Table */}
              {details.files.length > 0 && (
                <div className="space-y-3">
                  <DataTable
                    data={details.files}
                    columns={fileColumns}
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
                      wrapper: 'max-h-80',
                      table: 'min-w-full',
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
  )
}

// Stat card component for main metrics
function StatCard({
  title,
  value,
  subtitle,
  icon,
  valueColor,
}: {
  title: string
  value: string
  subtitle?: string
  icon: string
  valueColor?: 'primary' | 'success' | 'danger'
}) {
  const colorClass = valueColor
    ? valueColor === 'success'
      ? 'text-success'
      : valueColor === 'danger'
        ? 'text-danger'
        : 'text-primary'
    : 'text-foreground'

  return (
    <Card className="bg-content2/50">
      <CardBody className="p-3">
        <div className="flex items-start justify-between">
          <div>
            <span className="text-xs text-default-400 uppercase tracking-wide">{title}</span>
            <div className={`text-lg font-bold tabular-nums ${colorClass}`}>{value}</div>
            {subtitle && (
              <span className="text-xs text-default-400">{subtitle}</span>
            )}
          </div>
          <span className="text-xl opacity-60">{icon}</span>
        </div>
      </CardBody>
    </Card>
  )
}

// Mini stat for secondary metrics
function MiniStat({
  label,
  value,
  color,
}: {
  label: string
  value: string
  color?: 'success' | 'danger' | 'primary'
}) {
  const colorClass = color
    ? color === 'success'
      ? 'text-success'
      : color === 'danger'
        ? 'text-danger'
        : 'text-primary'
    : 'text-foreground'

  return (
    <div className="bg-content2/30 rounded-lg p-2 text-center">
      <div className="text-xs text-default-400 mb-0.5">{label}</div>
      <div className={`font-semibold tabular-nums ${colorClass}`}>{value}</div>
    </div>
  )
}
