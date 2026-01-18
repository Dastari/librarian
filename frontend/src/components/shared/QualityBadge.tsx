import { Chip } from '@heroui/chip'

interface QualityBadgeProps {
  resolution?: string | null
  codec?: string | null
  hdr?: boolean | null
  hdrType?: string | null
  /** Show all parts in a single badge or as separate badges */
  combined?: boolean
  size?: 'sm' | 'md' | 'lg'
}

/**
 * Format resolution for display
 */
function formatResolution(resolution: string): string {
  if (resolution === '2160p') return '4K'
  return resolution.toUpperCase()
}

/**
 * Format codec for display
 */
function formatCodec(codec: string): string {
  return codec.toUpperCase()
}

/**
 * Format HDR type for display
 */
function formatHdrType(hdrType: string): string {
  switch (hdrType.toLowerCase()) {
    case 'dolbyvision':
    case 'dolby_vision':
      return 'DV'
    case 'hdr10plus':
    case 'hdr10+':
      return 'HDR10+'
    case 'hdr10':
      return 'HDR10'
    case 'hlg':
      return 'HLG'
    default:
      return 'HDR'
  }
}

/**
 * A reusable component for displaying quality information (resolution, codec, HDR).
 * Can render as combined or separate badges.
 */
export function QualityBadge({ 
  resolution, 
  codec, 
  hdr, 
  hdrType, 
  combined = false,
  size = 'sm' 
}: QualityBadgeProps) {
  if (combined) {
    const parts: string[] = []
    if (resolution) parts.push(formatResolution(resolution))
    if (codec) parts.push(formatCodec(codec))
    if (hdr || hdrType) parts.push(hdrType ? formatHdrType(hdrType) : 'HDR')
    
    if (parts.length === 0) return null
    
    return (
      <Chip size={size} variant="flat" color="primary">
        {parts.join(' • ')}
      </Chip>
    )
  }
  
  // Separate badges
  return (
    <div className="flex gap-1">
      {resolution && (
        <Chip size={size} variant="flat" color="primary">
          {formatResolution(resolution)}
        </Chip>
      )}
      {codec && (
        <Chip size={size} variant="flat" color="default">
          {formatCodec(codec)}
        </Chip>
      )}
      {(hdr || hdrType) && (
        <Chip size={size} variant="flat" color="warning">
          {hdrType ? formatHdrType(hdrType) : 'HDR'}
        </Chip>
      )}
    </div>
  )
}
