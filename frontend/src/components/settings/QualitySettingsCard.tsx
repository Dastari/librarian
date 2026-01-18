import { Card, CardHeader, CardBody } from '@heroui/card'
import { Checkbox, CheckboxGroup } from '@heroui/checkbox'
import { Divider } from '@heroui/divider'
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

export const AUDIO_FORMAT_OPTIONS = [
  { value: 'atmos', label: 'Dolby Atmos' },
  { value: 'truehd', label: 'TrueHD' },
  { value: 'dtshd', label: 'DTS-HD MA' },
  { value: 'dts', label: 'DTS' },
  { value: 'dd+', label: 'Dolby Digital Plus' },
  { value: 'dd', label: 'Dolby Digital 5.1' },
  { value: 'aac', label: 'AAC' },
]

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
}

export function QualitySettingsCard({
  settings,
  onChange,
  isOverrideMode = false,
  isInheriting = false,
  onInheritChange,
  title = 'Quality Filters',
  description = 'Configure which releases to accept. Empty = accept any.',
}: QualitySettingsCardProps) {
  const isDisabled = isOverrideMode && isInheriting

  return (
    <Card className="bg-content2  ">
      <CardHeader className="flex flex-col items-start gap-1">
        <div className="flex items-center gap-2 w-full justify-between">
          <div>
            <h3 className="font-semibold">{title}</h3>
            <p className="text-small text-default-500">{description}</p>
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
      </CardHeader>
      <CardBody className={`space-y-6 ${isDisabled ? 'opacity-50 pointer-events-none' : ''}`}>
        {/* Resolution */}
        <div>
          <div className="flex items-center gap-2 mb-3">
            <IconDeviceTv size={18} className="text-blue-400" />
            <span className="text-sm font-medium">Resolution</span>
            {settings.allowedResolutions.length === 0 && (
              <span className="text-xs text-default-400">(Any)</span>
            )}
          </div>
          <CheckboxGroup
            orientation="horizontal"
            value={settings.allowedResolutions}
            onValueChange={(val) => onChange({ ...settings, allowedResolutions: val })}
            classNames={{ wrapper: 'gap-3 flex-wrap' }}
          >
            {RESOLUTION_OPTIONS.map((opt) => (
              <Checkbox key={opt.value} value={opt.value} size="sm">
                {opt.label}
              </Checkbox>
            ))}
          </CheckboxGroup>
        </div>

        <Divider />

        {/* Video Codec */}
        <div>
          <div className="flex items-center gap-2 mb-3">
            <IconVideo size={18} className="text-purple-400" />
            <span className="text-sm font-medium">Video Codec</span>
            {settings.allowedVideoCodecs.length === 0 && (
              <span className="text-xs text-default-400">(Any)</span>
            )}
          </div>
          <CheckboxGroup
            orientation="horizontal"
            value={settings.allowedVideoCodecs}
            onValueChange={(val) => onChange({ ...settings, allowedVideoCodecs: val })}
            classNames={{ wrapper: 'gap-3 flex-wrap' }}
          >
            {VIDEO_CODEC_OPTIONS.map((opt) => (
              <Checkbox key={opt.value} value={opt.value} size="sm">
                {opt.label}
              </Checkbox>
            ))}
          </CheckboxGroup>
        </div>

        <Divider />

        {/* Audio Format */}
        <div>
          <div className="flex items-center gap-2 mb-3">
            <IconMusic size={18} className="text-green-400" />
            <span className="text-sm font-medium">Audio Format</span>
            {settings.allowedAudioFormats.length === 0 && (
              <span className="text-xs text-default-400">(Any)</span>
            )}
          </div>
          <CheckboxGroup
            orientation="horizontal"
            value={settings.allowedAudioFormats}
            onValueChange={(val) => onChange({ ...settings, allowedAudioFormats: val })}
            classNames={{ wrapper: 'gap-3 flex-wrap' }}
          >
            {AUDIO_FORMAT_OPTIONS.map((opt) => (
              <Checkbox key={opt.value} value={opt.value} size="sm">
                {opt.label}
              </Checkbox>
            ))}
          </CheckboxGroup>
        </div>

        <Divider />

        {/* HDR Settings */}
        <div>
          <div className="flex items-center gap-2 mb-3">
            <IconSun size={18} className="text-amber-400" />
            <span className="text-sm font-medium">HDR</span>
          </div>
          <div className="flex flex-col gap-3">
            <Switch
              isSelected={settings.requireHdr}
              onValueChange={(val) => onChange({ ...settings, requireHdr: val })}
              size="sm"
            >
              Require HDR
            </Switch>
            {settings.requireHdr && (
              <div className="ml-6">
                <p className="text-xs text-default-400 mb-2">
                  Allowed HDR Types (empty = any HDR)
                </p>
                <CheckboxGroup
                  orientation="horizontal"
                  value={settings.allowedHdrTypes}
                  onValueChange={(val) => onChange({ ...settings, allowedHdrTypes: val })}
                  classNames={{ wrapper: 'gap-3 flex-wrap' }}
                >
                  {HDR_TYPE_OPTIONS.map((opt) => (
                    <Checkbox key={opt.value} value={opt.value} size="sm">
                      {opt.label}
                    </Checkbox>
                  ))}
                </CheckboxGroup>
              </div>
            )}
          </div>
        </div>

        <Divider />

        {/* Source */}
        <div>
          <div className="flex items-center gap-2 mb-3">
            <IconDeviceTv size={18} className="text-cyan-400" />
            <span className="text-sm font-medium">Source / Release Type</span>
            {settings.allowedSources.length === 0 && (
              <span className="text-xs text-default-400">(Any)</span>
            )}
          </div>
          <CheckboxGroup
            orientation="horizontal"
            value={settings.allowedSources}
            onValueChange={(val) => onChange({ ...settings, allowedSources: val })}
            classNames={{ wrapper: 'gap-3 flex-wrap' }}
          >
            {SOURCE_OPTIONS.map((opt) => (
              <Checkbox key={opt.value} value={opt.value} size="sm">
                {opt.label}
              </Checkbox>
            ))}
          </CheckboxGroup>
        </div>

        <Divider />

        {/* Release Groups */}
        <div>
          <div className="flex items-center gap-2 mb-3">
            <IconUsers size={18} className="text-default-400" />
            <span className="text-sm font-medium">Release Groups</span>
          </div>
          <div className="flex flex-col gap-4">
            <Input
              label="Blacklist"
              placeholder="e.g., YIFY, EVO (comma-separated)"
              size="sm"
              value={settings.releaseGroupBlacklist.join(', ')}
              onChange={(e) => {
                const groups = e.target.value
                  .split(',')
                  .map((g) => g.trim())
                  .filter((g) => g.length > 0)
                onChange({ ...settings, releaseGroupBlacklist: groups })
              }}
              description="Block releases from these groups"
            />
            <Input
              label="Whitelist"
              placeholder="e.g., NTb, FLUX (comma-separated)"
              size="sm"
              value={settings.releaseGroupWhitelist.join(', ')}
              onChange={(e) => {
                const groups = e.target.value
                  .split(',')
                  .map((g) => g.trim())
                  .filter((g) => g.length > 0)
                onChange({ ...settings, releaseGroupWhitelist: groups })
              }}
              description="Only allow releases from these groups (leave empty for any)"
            />
          </div>
        </div>
      </CardBody>
    </Card>
  )
}

export default QualitySettingsCard
