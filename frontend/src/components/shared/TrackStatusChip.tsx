import { Chip } from '@heroui/chip'
import type { TrackStatus } from '../../lib/graphql'

type ChipColor = 'success' | 'warning' | 'danger' | 'default' | 'primary' | 'secondary'

interface TrackStatusConfig {
  color: ChipColor
  label: string
}

const TRACK_STATUS_CONFIG: Record<TrackStatus, TrackStatusConfig> = {
  downloaded: { color: 'success', label: 'Downloaded' },
  downloading: { color: 'primary', label: 'Downloading' },
  wanted: { color: 'warning', label: 'Wanted' },
  missing: { color: 'danger', label: 'Missing' },
}

/**
 * Get the color for a track status (for use in other contexts)
 */
export function getTrackStatusColor(status: TrackStatus): ChipColor {
  return TRACK_STATUS_CONFIG[status]?.color ?? 'default'
}

/**
 * Get the label for a track status
 */
export function getTrackStatusLabel(status: TrackStatus): string {
  return TRACK_STATUS_CONFIG[status]?.label ?? status
}

interface TrackStatusChipProps {
  status: TrackStatus
  size?: 'sm' | 'md' | 'lg'
}

/**
 * A reusable chip for displaying track status consistently across the app.
 */
export function TrackStatusChip({ status, size = 'sm' }: TrackStatusChipProps) {
  const config = TRACK_STATUS_CONFIG[status] ?? { color: 'default' as ChipColor, label: status }
  
  return (
    <Chip size={size} color={config.color} variant="flat">
      {config.label}
    </Chip>
  )
}
