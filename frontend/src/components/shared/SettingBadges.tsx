import { Chip } from '@heroui/chip'
import { Tooltip } from '@heroui/tooltip'
import { IconLink, IconCheck, IconX } from '@tabler/icons-react'

// ============================================================================
// Common Types
// ============================================================================

interface BaseBadgeProps {
  /** Whether the value is inherited from a parent (library) */
  isInherited?: boolean
  /** Whether the setting is enabled */
  isEnabled: boolean
  /** Size of the badge */
  size?: 'sm' | 'md' | 'lg'
  /** Optional click handler */
  onClick?: () => void
  /** Whether the badge is in a loading state */
  isLoading?: boolean
}

// ============================================================================
// Helper: Status Icon
// ============================================================================

/**
 * Returns the appropriate start content for a badge based on enabled/inherited state.
 * Shows check icon if enabled, X icon if disabled, with link icon for inherited values.
 */
function getStatusIcon(isEnabled: boolean, isInherited: boolean) {
  if (isInherited) {
    return (
      <span className="flex items-center gap-0.5">
        <IconLink size={12} className="text-default-400" />
        {isEnabled ? (
          <IconCheck size={12} className="text-success" />
        ) : (
          <IconX size={12} className="text-default-400" />
        )}
      </span>
    )
  }
  
  return isEnabled ? (
    <IconCheck size={12} className="text-success" />
  ) : (
    <IconX size={12} className="text-default-400" />
  )
}

// ============================================================================
// Auto Download Badge (Deprecated - use AutoHuntBadge)
// ============================================================================

interface AutoDownloadBadgeProps extends BaseBadgeProps {}

/**
 * @deprecated Use AutoHuntBadge instead. Auto Download and Auto Hunt are now consolidated.
 * This badge is kept for backwards compatibility but renders the same as AutoHuntBadge.
 */
export function AutoDownloadBadge(props: AutoDownloadBadgeProps) {
  // Redirect to AutoHuntBadge since they're now consolidated
  return <AutoHuntBadge {...props} />
}

// ============================================================================
// File Organization Badge
// ============================================================================

interface FileOrganizationBadgeProps extends BaseBadgeProps {}

/**
 * Badge showing file organization status.
 * When enabled, files will be automatically renamed and placed into folders.
 */
export function FileOrganizationBadge({
  isInherited = false,
  isEnabled,
  size = 'sm',
  onClick,
}: FileOrganizationBadgeProps) {
  const tooltip = isEnabled
    ? 'Files will be automatically renamed and organized into folders'
    : 'File organization is disabled - files will remain in download location'

  return (
    <Tooltip content={tooltip}>
      <Chip
        size={size}
        variant="flat"
        color={isEnabled ? 'success' : 'default'}
        className={onClick ? 'cursor-pointer' : ''}
        onClick={onClick}
        startContent={getStatusIcon(isEnabled, isInherited)}
      >
        File Organization
      </Chip>
    </Tooltip>
  )
}

// ============================================================================
// Monitored Badge
// ============================================================================

export type MonitorType = 'ALL' | 'FUTURE' | 'NONE' | string

interface MonitoredBadgeProps {
  /** Which episodes are monitored */
  monitorType: MonitorType
  /** Size of the badge */
  size?: 'sm' | 'md' | 'lg'
  /** Optional click handler */
  onClick?: () => void
}

/**
 * Badge showing which episodes are being monitored.
 * Monitored episodes are matched against RSS feeds. When a match is found,
 * the episode becomes "available" for download.
 */
export function MonitoredBadge({
  monitorType,
  size = 'sm',
  onClick,
}: MonitoredBadgeProps) {
  let label: string
  let tooltip: string
  let color: 'success' | 'primary' | 'default' | 'warning'

  switch (monitorType) {
    case 'ALL':
      label = 'All Episodes'
      tooltip = 'All episodes are automatically searched and downloaded when available'
      color = 'success'
      break
    case 'FUTURE':
      label = 'Future Only'
      tooltip = 'Only episodes that air after the show was added will be searched and downloaded'
      color = 'primary'
      break
    case 'NONE':
      label = 'Not Monitored'
      tooltip = 'No episodes will be automatically searched or downloaded'
      color = 'default'
      break
    default:
      label = monitorType
      tooltip = `Monitor type: ${monitorType}`
      color = 'default'
  }

  return (
    <Tooltip content={tooltip}>
      <Chip
        size={size}
        variant="flat"
        color={color}
        className={onClick ? 'cursor-pointer' : ''}
        onClick={onClick}
      >
        {label}
      </Chip>
    </Tooltip>
  )
}

// ============================================================================
// Auto Hunt Badge
// ============================================================================

interface AutoHuntBadgeProps extends BaseBadgeProps {}

/**
 * Badge showing auto-hunt status.
 * When enabled, the system will automatically search indexers and RSS feeds
 * for missing content and download the best matches.
 */
export function AutoHuntBadge({
  isInherited = false,
  isEnabled,
  size = 'sm',
  onClick,
  isLoading = false,
}: AutoHuntBadgeProps) {
  const tooltip = isEnabled
    ? 'Missing content will be automatically searched and downloaded from indexers and RSS feeds'
    : 'Auto-hunt is disabled - content must be searched and downloaded manually'

  return (
    <Tooltip content={tooltip}>
      <Chip
        size={size}
        variant="flat"
        color={isEnabled ? 'success' : 'default'}
        className={onClick ? 'cursor-pointer' : ''}
        onClick={onClick}
        startContent={getStatusIcon(isEnabled, isInherited)}
      >
        {isLoading ? 'Updating...' : 'Auto Hunt'}
      </Chip>
    </Tooltip>
  )
}

// ============================================================================
// Quality Filter Badge
// ============================================================================

interface QualityFilterBadgeProps {
  /** Allowed resolutions */
  resolutions: string[]
  /** Allowed video codecs */
  codecs?: string[]
  /** Whether HDR is required */
  requireHdr?: boolean
  /** Whether the value is inherited from a parent (library) */
  isInherited?: boolean
  /** Size of the badge */
  size?: 'sm' | 'md' | 'lg'
  /** Optional click handler */
  onClick?: () => void
}

/**
 * Badge showing quality filter settings.
 * Only releases matching these quality criteria will be downloaded.
 */
export function QualityFilterBadge({
  resolutions,
  codecs = [],
  requireHdr = false,
  isInherited = false,
  size = 'sm',
  onClick,
}: QualityFilterBadgeProps) {
  // Build display string
  const parts: string[] = []
  
  if (resolutions.length > 0) {
    // Show multiple resolutions if selected
    const resParts: string[] = []
    if (resolutions.includes('2160p')) resParts.push('4K')
    if (resolutions.includes('1080p')) resParts.push('1080p')
    if (resolutions.includes('720p')) resParts.push('720p')
    if (resolutions.includes('480p')) resParts.push('480p')
    
    if (resParts.length > 0) {
      // Show first resolution, or "X+" if multiple
      if (resParts.length === 1) {
        parts.push(resParts[0])
      } else if (resParts.length === resolutions.length && resParts.length > 2) {
        // All common resolutions selected - show the highest with "+"
        parts.push(`${resParts[0]}+`)
      } else {
        // Show all selected
        parts.push(resParts.join('/'))
      }
    } else {
      parts.push(resolutions.join('/'))
    }
  }
  
  if (codecs.length > 0 && codecs.length <= 2) {
    parts.push(codecs.map(c => c.toUpperCase()).join('/'))
  }
  
  if (requireHdr) {
    parts.push('HDR')
  }
  
  const qualitySummary = parts.length > 0 ? parts.join(' ') : 'Any Quality'
  
  // Build detailed tooltip
  let tooltip: string
  if (parts.length === 0) {
    tooltip = 'No quality restrictions - any release will be accepted'
  } else {
    const details: string[] = []
    if (resolutions.length > 0) {
      const resNames = resolutions.map(r => r === '2160p' ? '4K' : r)
      details.push(`Resolution: ${resNames.join(', ')}`)
    }
    if (codecs.length > 0) {
      details.push(`Codec: ${codecs.join(', ')}`)
    }
    if (requireHdr) {
      details.push('HDR required')
    }
    tooltip = details.join(' | ')
  }

  return (
    <Tooltip content={tooltip}>
      <Chip
        size={size}
        variant="flat"
        color="secondary"
        className={onClick ? 'cursor-pointer' : ''}
        onClick={onClick}
        startContent={isInherited ? <IconLink size={12} className="text-default-400" /> : undefined}
      >
        {qualitySummary}
      </Chip>
    </Tooltip>
  )
}
