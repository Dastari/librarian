import { Chip } from '@heroui/chip'
import { Progress } from '@heroui/progress'

type ChipColor = 'success' | 'warning' | 'danger' | 'default' | 'primary' | 'secondary'

/** Derived status based on mediaFileId presence */
export type DerivedChapterStatus = 'downloaded' | 'downloading' | 'wanted'

interface StatusConfig {
  color: ChipColor
  label: string
}

const STATUS_CONFIG: Record<DerivedChapterStatus, StatusConfig> = {
  downloaded: { color: 'success', label: 'Downloaded' },
  downloading: { color: 'primary', label: 'Downloading' },
  wanted: { color: 'warning', label: 'Wanted' },
}

/**
 * Derive chapter status from mediaFileId
 * - If mediaFileId is set, status is 'downloaded'
 * - If downloading (has progress), status is 'downloading'
 * - Otherwise status is 'wanted'
 */
export function deriveChapterStatus(
  mediaFileId: string | null | undefined,
  downloadProgress?: number | null
): DerivedChapterStatus {
  if (mediaFileId) return 'downloaded'
  if (downloadProgress != null && downloadProgress > 0) return 'downloading'
  return 'wanted'
}

/**
 * Get the color for a chapter status (for use in other contexts)
 */
export function getChapterStatusColor(status: DerivedChapterStatus): ChipColor {
  return STATUS_CONFIG[status]?.color ?? 'default'
}

/**
 * Get the label for a chapter status
 */
export function getChapterStatusLabel(status: DerivedChapterStatus): string {
  return STATUS_CONFIG[status]?.label ?? status
}

interface ChapterStatusChipProps {
  /** Media file ID - if set, chapter is downloaded */
  mediaFileId?: string | null
  size?: 'sm' | 'md' | 'lg'
  /** Download progress (0.0 to 1.0) when downloading */
  downloadProgress?: number | null
}

/**
 * A reusable chip for displaying chapter status consistently across the app.
 * Status is derived from mediaFileId: present = Downloaded, absent = Wanted.
 * Shows a progress bar when downloading with progress info.
 */
export function ChapterStatusChip({ mediaFileId, size = 'sm', downloadProgress }: ChapterStatusChipProps) {
  const status = deriveChapterStatus(mediaFileId, downloadProgress)
  const config = STATUS_CONFIG[status]

  // Show progress bar when downloading with progress info
  if (status === 'downloading' && downloadProgress != null) {
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
