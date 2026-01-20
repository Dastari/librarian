import { Card, CardHeader, CardBody } from '@heroui/card'
import { Checkbox, CheckboxGroup } from '@heroui/checkbox'
import { Switch } from '@heroui/switch'
import { Input } from '@heroui/input'
import { IconVideo, IconMusic, IconSun, IconDeviceTv, IconUsers } from '@tabler/icons-react'

// Quality option definitions
export const RESOLUTION_OPTIONS = [
  { value: '2160p', label: '4K / 2160p' },
  { value: '1080p', label: 'Full HD / 1080p' },
  { value: '720p', label: 'HD / 720p' },
  { value: '480p', label: 'SD / 480p' },
]

export const VIDEO_CODEC_OPTIONS = [
  { value: 'hevc', label: 'HEVC / H.265' },
  { value: 'h264', label: 'H.264 / AVC' },
  { value: 'av1', label: 'AV1' },
  { value: 'xvid', label: 'XviD' },
]

// Audio formats for video content (surround sound)
export const VIDEO_AUDIO_FORMAT_OPTIONS = [
  { value: 'atmos', label: 'Dolby Atmos' },
  { value: 'truehd', label: 'TrueHD' },
  { value: 'dtshd', label: 'DTS-HD MA' },
  { value: 'dts', label: 'DTS' },
  { value: 'dd+', label: 'Dolby Digital Plus' },
  { value: 'dd', label: 'Dolby Digital 5.1' },
  { value: 'aac', label: 'AAC' },
]

// Audio formats for music content
export const MUSIC_AUDIO_FORMAT_OPTIONS = [
  { value: 'flac', label: 'FLAC (Lossless)' },
  { value: 'alac', label: 'ALAC (Apple Lossless)' },
  { value: 'wav', label: 'WAV' },
  { value: 'aiff', label: 'AIFF' },
  { value: '320', label: 'MP3 320kbps' },
  { value: 'v0', label: 'MP3 V0 (VBR)' },
  { value: 'aac', label: 'AAC' },
  { value: 'ogg', label: 'OGG Vorbis' },
]

// Bit depth options for music
export const BIT_DEPTH_OPTIONS = [
  { value: '24bit', label: '24-bit' },
  { value: '16bit', label: '16-bit' },
]

// Sample rate options for music
export const SAMPLE_RATE_OPTIONS = [
  { value: '192', label: '192 kHz' },
  { value: '96', label: '96 kHz' },
  { value: '48', label: '48 kHz' },
  { value: '44.1', label: '44.1 kHz' },
]

// Legacy export for backwards compatibility
export const AUDIO_FORMAT_OPTIONS = VIDEO_AUDIO_FORMAT_OPTIONS

export const HDR_TYPE_OPTIONS = [
  { value: 'dolbyvision', label: 'Dolby Vision' },
  { value: 'hdr10plus', label: 'HDR10+' },
  { value: 'hdr10', label: 'HDR10' },
  { value: 'hlg', label: 'HLG' },
]

export const SOURCE_OPTIONS = [
  { value: 'webdl', label: 'WEB-DL' },
  { value: 'webrip', label: 'WEBRip' },
  { value: 'bluray', label: 'BluRay' },
  { value: 'hdtv', label: 'HDTV' },
]

// Quick presets for easy setup
export const QUALITY_PRESETS = [
  {
    name: 'Any Quality',
    description: 'Accept all qualities',
    settings: {
      allowedResolutions: [],
      allowedVideoCodecs: [],
      allowedAudioFormats: [],
      requireHdr: false,
      allowedHdrTypes: [],
      allowedSources: [],
    },
  },
  {
    name: '4K HDR',
    description: '4K with HDR only',
    settings: {
      allowedResolutions: ['2160p'],
      allowedVideoCodecs: ['hevc', 'av1'],
      allowedAudioFormats: [],
      requireHdr: true,
      allowedHdrTypes: [],
      allowedSources: [],
    },
  },
  {
    name: '4K or 1080p',
    description: 'High quality (4K or 1080p)',
    settings: {
      allowedResolutions: ['2160p', '1080p'],
      allowedVideoCodecs: [],
      allowedAudioFormats: [],
      requireHdr: false,
      allowedHdrTypes: [],
      allowedSources: [],
    },
  },
  {
    name: '1080p Only',
    description: 'Full HD only',
    settings: {
      allowedResolutions: ['1080p'],
      allowedVideoCodecs: [],
      allowedAudioFormats: [],
      requireHdr: false,
      allowedHdrTypes: [],
      allowedSources: [],
    },
  },
  {
    name: 'HEVC Preferred',
    description: 'HEVC/H.265 codec only',
    settings: {
      allowedResolutions: [],
      allowedVideoCodecs: ['hevc'],
      allowedAudioFormats: [],
      requireHdr: false,
      allowedHdrTypes: [],
      allowedSources: [],
    },
  },
]

export interface QualitySettings {
  allowedResolutions: string[]
  allowedVideoCodecs: string[]
  allowedAudioFormats: string[]
  requireHdr: boolean
  allowedHdrTypes: string[]
  allowedSources: string[]
  releaseGroupBlacklist: string[]
  releaseGroupWhitelist: string[]
}

/** Default quality settings (accept any quality) */
export const DEFAULT_QUALITY_SETTINGS: QualitySettings = {
  allowedResolutions: [],
  allowedVideoCodecs: [],
  allowedAudioFormats: [],
  requireHdr: false,
  allowedHdrTypes: [],
  allowedSources: [],
  releaseGroupBlacklist: [],
  releaseGroupWhitelist: [],
}

export interface QualitySettingsCardProps {
  settings: QualitySettings
  onChange: (settings: QualitySettings) => void
  /** Show as override mode (shows "Inherit from Library" option) */
  isOverrideMode?: boolean
  /** If true, show inherit checkbox and disable all settings when inherited */
  isInheriting?: boolean
  /** Callback when inherit toggle changes */
  onInheritChange?: (inherit: boolean) => void
  /** Title for the card */
  title?: string
  /** Description for the card */
  description?: string
  /** If true, render without the Card wrapper (for use inside modals) */
  noCard?: boolean
  /** Library type - determines which quality options are shown */
  libraryType?: 'TV' | 'MOVIES' | 'MUSIC' | 'AUDIOBOOKS' | 'OTHER'
}

export function QualitySettingsCard({
  settings,
  onChange,
  isOverrideMode = false,
  isInheriting = false,
  onInheritChange,
  title = 'Quality Filters',
  description,
  noCard = false,
  libraryType = 'TV',
}: QualitySettingsCardProps) {
  const isDisabled = isOverrideMode && isInheriting
  const isAudioLibrary = libraryType === 'MUSIC' || libraryType === 'AUDIOBOOKS'

  const header = (
    <div className="flex items-center gap-2 w-full justify-between">
      <div>
        <h3 className="font-semibold">{title}</h3>
        {description && <p className="text-small text-default-500">{description}</p>}
      </div>
      {isOverrideMode && onInheritChange && (
        <Switch
          isSelected={!isInheriting}
          onValueChange={(val) => onInheritChange(!val)}
          size="sm"
        >
          Override
        </Switch>
      )}
    </div>
  )

  // Render video quality filters (TV/Movies)
  const renderVideoFilters = () => (
    <div className="grid grid-cols-2 md:grid-cols-3 gap-6">
      {/* Resolution */}
      <CheckboxGroup
        label={
          <div className="flex items-center gap-2">
            <IconDeviceTv size={16} className="text-blue-400" />
            <span className="text-sm font-medium">Resolution</span>
            {settings.allowedResolutions.length === 0 && (
              <span className="text-xs text-default-400">(Any)</span>
            )}
          </div>
        }
        value={settings.allowedResolutions}
        onValueChange={(val) => onChange({ ...settings, allowedResolutions: val })}
        classNames={{ wrapper: 'gap-1.5 mt-2' }}
      >
        {RESOLUTION_OPTIONS.map((opt) => (
          <Checkbox key={opt.value} value={opt.value} size="sm">
            {opt.label}
          </Checkbox>
        ))}
      </CheckboxGroup>

      {/* Video Codec */}
      <CheckboxGroup
        label={
          <div className="flex items-center gap-2">
            <IconVideo size={16} className="text-purple-400" />
            <span className="text-sm font-medium">Video Codec</span>
            {settings.allowedVideoCodecs.length === 0 && (
              <span className="text-xs text-default-400">(Any)</span>
            )}
          </div>
        }
        value={settings.allowedVideoCodecs}
        onValueChange={(val) => onChange({ ...settings, allowedVideoCodecs: val })}
        classNames={{ wrapper: 'gap-1.5 mt-2' }}
      >
        {VIDEO_CODEC_OPTIONS.map((opt) => (
          <Checkbox key={opt.value} value={opt.value} size="sm">
            {opt.label}
          </Checkbox>
        ))}
      </CheckboxGroup>

      {/* Audio Format (for video) */}
      <CheckboxGroup
        label={
          <div className="flex items-center gap-2">
            <IconMusic size={16} className="text-green-400" />
            <span className="text-sm font-medium">Audio Format</span>
            {settings.allowedAudioFormats.length === 0 && (
              <span className="text-xs text-default-400">(Any)</span>
            )}
          </div>
        }
        value={settings.allowedAudioFormats}
        onValueChange={(val) => onChange({ ...settings, allowedAudioFormats: val })}
        classNames={{ wrapper: 'gap-1.5 mt-2' }}
      >
        {VIDEO_AUDIO_FORMAT_OPTIONS.map((opt) => (
          <Checkbox key={opt.value} value={opt.value} size="sm">
            {opt.label}
          </Checkbox>
        ))}
      </CheckboxGroup>

      {/* Source */}
      <CheckboxGroup
        label={
          <div className="flex items-center gap-2">
            <IconDeviceTv size={16} className="text-cyan-400" />
            <span className="text-sm font-medium">Source</span>
            {settings.allowedSources.length === 0 && (
              <span className="text-xs text-default-400">(Any)</span>
            )}
          </div>
        }
        value={settings.allowedSources}
        onValueChange={(val) => onChange({ ...settings, allowedSources: val })}
        classNames={{ wrapper: 'gap-1.5 mt-2' }}
      >
        {SOURCE_OPTIONS.map((opt) => (
          <Checkbox key={opt.value} value={opt.value} size="sm">
            {opt.label}
          </Checkbox>
        ))}
      </CheckboxGroup>

      {/* HDR Settings */}
      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-2">
          <IconSun size={16} className="text-amber-400" />
          <span className="text-sm font-medium">HDR</span>
        </div>
        <Switch
          isSelected={settings.requireHdr}
          onValueChange={(val) => onChange({ ...settings, requireHdr: val })}
          size="sm"
        >
          Require HDR
        </Switch>
        {settings.requireHdr && (
          <CheckboxGroup
            value={settings.allowedHdrTypes}
            onValueChange={(val) => onChange({ ...settings, allowedHdrTypes: val })}
            classNames={{ wrapper: 'gap-1.5 ml-1' }}
            size="sm"
            description="Empty = any HDR type"
          >
            {HDR_TYPE_OPTIONS.map((opt) => (
              <Checkbox key={opt.value} value={opt.value} size="sm">
                {opt.label}
              </Checkbox>
            ))}
          </CheckboxGroup>
        )}
      </div>
    </div>
  )

  // Render audio quality filters (Music/Audiobooks)
  const renderAudioFilters = () => (
    <div className="grid grid-cols-2 md:grid-cols-3 gap-6">
      {/* Audio Format (for music) */}
      <CheckboxGroup
        label={
          <div className="flex items-center gap-2">
            <IconMusic size={16} className="text-green-400" />
            <span className="text-sm font-medium">Audio Format</span>
            {settings.allowedAudioFormats.length === 0 && (
              <span className="text-xs text-default-400">(Any)</span>
            )}
          </div>
        }
        value={settings.allowedAudioFormats}
        onValueChange={(val) => onChange({ ...settings, allowedAudioFormats: val })}
        classNames={{ wrapper: 'gap-1.5 mt-2' }}
      >
        {MUSIC_AUDIO_FORMAT_OPTIONS.map((opt) => (
          <Checkbox key={opt.value} value={opt.value} size="sm">
            {opt.label}
          </Checkbox>
        ))}
      </CheckboxGroup>

      {/* Bit Depth */}
      <CheckboxGroup
        label={
          <div className="flex items-center gap-2">
            <IconVideo size={16} className="text-purple-400" />
            <span className="text-sm font-medium">Bit Depth</span>
            {settings.allowedResolutions.length === 0 && (
              <span className="text-xs text-default-400">(Any)</span>
            )}
          </div>
        }
        value={settings.allowedResolutions}
        onValueChange={(val) => onChange({ ...settings, allowedResolutions: val })}
        classNames={{ wrapper: 'gap-1.5 mt-2' }}
      >
        {BIT_DEPTH_OPTIONS.map((opt) => (
          <Checkbox key={opt.value} value={opt.value} size="sm">
            {opt.label}
          </Checkbox>
        ))}
      </CheckboxGroup>

      {/* Sample Rate */}
      <CheckboxGroup
        label={
          <div className="flex items-center gap-2">
            <IconDeviceTv size={16} className="text-cyan-400" />
            <span className="text-sm font-medium">Sample Rate</span>
            {settings.allowedSources.length === 0 && (
              <span className="text-xs text-default-400">(Any)</span>
            )}
          </div>
        }
        value={settings.allowedSources}
        onValueChange={(val) => onChange({ ...settings, allowedSources: val })}
        classNames={{ wrapper: 'gap-1.5 mt-2' }}
      >
        {SAMPLE_RATE_OPTIONS.map((opt) => (
          <Checkbox key={opt.value} value={opt.value} size="sm">
            {opt.label}
          </Checkbox>
        ))}
      </CheckboxGroup>
    </div>
  )

  const content = (
    <div className={`flex flex-col gap-6 ${isDisabled ? 'opacity-50 pointer-events-none' : ''}`}>
      {isAudioLibrary ? renderAudioFilters() : renderVideoFilters()}

      {/* Release Groups - full width row */}
      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-2">
          <IconUsers size={16} className="text-default-400" />
          <span className="text-sm font-medium">Release Groups</span>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Input
            label="Blacklist"
            labelPlacement="inside"
            variant="flat"
            placeholder="e.g., YIFY, EVO (comma-separated)"
            description="Release groups to reject"
            size="sm"
            value={settings.releaseGroupBlacklist.join(', ')}
            onChange={(e) => {
              const groups = e.target.value
                .split(',')
                .map((g) => g.trim())
                .filter((g) => g.length > 0)
              onChange({ ...settings, releaseGroupBlacklist: groups })
            }}
            classNames={{
              label: 'text-sm font-medium text-primary!',
            }}
          />
          <Input
            label="Whitelist"
            labelPlacement="inside"
            variant="flat"
            placeholder="e.g., NTb, FLUX (comma-separated)"
            description="Release groups to prefer"
            size="sm"
            value={settings.releaseGroupWhitelist.join(', ')}
            onChange={(e) => {
              const groups = e.target.value
                .split(',')
                .map((g) => g.trim())
                .filter((g) => g.length > 0)
              onChange({ ...settings, releaseGroupWhitelist: groups })
            }}
            classNames={{
              label: 'text-sm font-medium text-primary!',
            }}
          />
        </div>
      </div>
    </div>
  )

  if (noCard) {
    return (
      <div className="flex flex-col gap-4">
        {header}
        {content}
      </div>
    )
  }

  return (
    <Card className="bg-content1">
      <CardHeader className="flex flex-col items-start gap-1">
        {header}
      </CardHeader>
      <CardBody>
        {content}
      </CardBody>
    </Card>
  )
}

export default QualitySettingsCard
