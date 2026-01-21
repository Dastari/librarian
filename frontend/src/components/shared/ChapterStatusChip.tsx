import { Chip } from '@heroui/chip'
import type { ChapterStatus } from '../../lib/graphql'

type ChipColor = 'success' | 'warning' | 'danger' | 'default' | 'primary' | 'secondary'

interface ChapterStatusConfig {
  color: ChipColor
  label: string
}

const CHAPTER_STATUS_CONFIG: Record<ChapterStatus, ChapterStatusConfig> = {
  downloaded: { color: 'success', label: 'Downloaded' },
  downloading: { color: 'primary', label: 'Downloading' },
  wanted: { color: 'warning', label: 'Wanted' },
  missing: { color: 'danger', label: 'Missing' },
}

/**
 * Get the color for a chapter status (for use in other contexts)
 */
export function getChapterStatusColor(status: ChapterStatus): ChipColor {
  return CHAPTER_STATUS_CONFIG[status]?.color ?? 'default'
}

/**
 * Get the label for a chapter status
 */
export function getChapterStatusLabel(status: ChapterStatus): string {
  return CHAPTER_STATUS_CONFIG[status]?.label ?? status
}

interface ChapterStatusChipProps {
  status: ChapterStatus
  size?: 'sm' | 'md' | 'lg'
}

/**
 * A reusable chip for displaying chapter status consistently across the app.
 */
export function ChapterStatusChip({ status, size = 'sm' }: ChapterStatusChipProps) {
  const config = CHAPTER_STATUS_CONFIG[status] ?? { color: 'default' as ChipColor, label: status }
  
  return (
    <Chip size={size} color={config.color} variant="flat">
      {config.label}
    </Chip>
  )
}
