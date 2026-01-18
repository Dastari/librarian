import { Chip } from '@heroui/chip'
import type { EpisodeStatus } from '../../lib/graphql'

type ChipColor = 'success' | 'warning' | 'danger' | 'default' | 'primary' | 'secondary'

interface EpisodeStatusConfig {
  color: ChipColor
  label: string
}

const EPISODE_STATUS_CONFIG: Record<EpisodeStatus, EpisodeStatusConfig> = {
  DOWNLOADED: { color: 'success', label: 'Downloaded' },
  DOWNLOADING: { color: 'primary', label: 'Downloading' },
  AVAILABLE: { color: 'secondary', label: 'Available' },
  WANTED: { color: 'warning', label: 'Wanted' },
  MISSING: { color: 'danger', label: 'Missing' },
  IGNORED: { color: 'default', label: 'Ignored' },
}

/**
 * Get the color for an episode status (for use in other contexts)
 */
export function getEpisodeStatusColor(status: EpisodeStatus): ChipColor {
  return EPISODE_STATUS_CONFIG[status]?.color ?? 'default'
}

/**
 * Get the label for an episode status
 */
export function getEpisodeStatusLabel(status: EpisodeStatus): string {
  return EPISODE_STATUS_CONFIG[status]?.label ?? status
}

interface EpisodeStatusChipProps {
  status: EpisodeStatus
  size?: 'sm' | 'md' | 'lg'
}

/**
 * A reusable chip for displaying episode status consistently across the app.
 */
export function EpisodeStatusChip({ status, size = 'sm' }: EpisodeStatusChipProps) {
  const config = EPISODE_STATUS_CONFIG[status] ?? { color: 'default' as ChipColor, label: status }
  
  return (
    <Chip size={size} color={config.color} variant="flat">
      {config.label}
    </Chip>
  )
}
