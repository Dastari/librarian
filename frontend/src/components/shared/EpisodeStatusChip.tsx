import { Chip } from '@heroui/chip'
import { Progress } from '@heroui/progress'

type ChipColor = 'success' | 'warning' | 'danger' | 'default' | 'primary' | 'secondary'

/** Derived status based on mediaFileId presence */
export type DerivedEpisodeStatus = 'DOWNLOADED' | 'DOWNLOADING' | 'WANTED'

interface StatusConfig {
  color: ChipColor
  label: string
}

const STATUS_CONFIG: Record<DerivedEpisodeStatus, StatusConfig> = {
  DOWNLOADED: { color: 'success', label: 'Downloaded' },
  DOWNLOADING: { color: 'primary', label: 'Downloading' },
  WANTED: { color: 'warning', label: 'Wanted' },
}

/**
 * Derive episode status from mediaFileId
 * - If mediaFileId is set, status is 'DOWNLOADED'
 * - If downloading (has progress), status is 'DOWNLOADING'
 * - Otherwise status is 'WANTED'
 */
export function deriveEpisodeStatus(
  mediaFileId: string | null | undefined,
  downloadProgress?: number | null
): DerivedEpisodeStatus {
  if (mediaFileId) return 'DOWNLOADED'
  if (downloadProgress != null && downloadProgress > 0) return 'DOWNLOADING'
  return 'WANTED'
}

/**
 * Get the color for an episode status (for use in other contexts)
 */
export function getEpisodeStatusColor(status: DerivedEpisodeStatus): ChipColor {
  return STATUS_CONFIG[status]?.color ?? 'default'
}

/**
 * Get the label for an episode status
 */
export function getEpisodeStatusLabel(status: DerivedEpisodeStatus): string {
  return STATUS_CONFIG[status]?.label ?? status
}

interface EpisodeStatusChipProps {
  /** Media file ID - if set, episode is downloaded */
  mediaFileId?: string | null
  size?: 'sm' | 'md' | 'lg'
  /** Download progress (0.0 to 1.0) when downloading */
  downloadProgress?: number | null
}

/**
 * A reusable chip for displaying episode status consistently across the app.
 * Status is derived from mediaFileId: present = Downloaded, absent = Wanted.
 * Shows a progress bar when downloading with progress info.
 */
export function EpisodeStatusChip({ mediaFileId, size = 'sm', downloadProgress }: EpisodeStatusChipProps) {
  const status = deriveEpisodeStatus(mediaFileId, downloadProgress)
  const config = STATUS_CONFIG[status]

  // Show progress bar when downloading with progress info
  if (status === 'DOWNLOADING' && downloadProgress != null) {
    const percent = Math.round(downloadProgress * 100)
    return (
      <div className="flex items-center gap-2 min-w-[100px]">
        <Progress
          size="sm"
          value={percent}
          color="primary"
          classNames={{
            track: 'h-2',
            indicator: 'h-2',
          }}
        />
        <span className="text-xs text-default-500 whitespace-nowrap">{percent}%</span>
      </div>
    )
  }

  return (
    <Chip size={size} color={config.color} variant="flat">
      {config.label}
    </Chip>
  )
}
