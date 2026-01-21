import { useState, useEffect } from 'react'
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from '@heroui/modal'
import { Button } from '@heroui/button'
import { Chip } from '@heroui/chip'
import { Spinner } from '@heroui/spinner'
import { Tabs, Tab } from '@heroui/tabs'
import { Tooltip } from '@heroui/tooltip'
import {
  IconFile,
  IconVideo,
  IconVolume,
  IconFileText,
  IconList,
  IconCopy,
  IconCheck,
  IconAlertCircle,
  IconInfoCircle,
} from '@tabler/icons-react'
import {
  graphqlClient,
  MEDIA_FILE_DETAILS_QUERY,
  type MediaFileDetails,
  type VideoStreamInfo,
  type AudioStreamInfo,
  type SubtitleInfo,
  type ChapterInfo,
} from '../lib/graphql'

interface FilePropertiesModalProps {
  isOpen: boolean
  onClose: () => void
  /** Media file ID to fetch details for */
  mediaFileId: string | null
  /** Optional title override (e.g., episode name) */
  title?: string
}

/** Format duration from seconds to HH:MM:SS */
function formatDuration(seconds: number | null): string {
  if (seconds === null || seconds === undefined) return '-'
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = Math.floor(seconds % 60)
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
  }
  return `${m}:${s.toString().padStart(2, '0')}`
}

/** Format bitrate to human readable */
function formatBitrate(bps: number | null): string {
  if (bps === null || bps === undefined) return '-'
  if (bps >= 1000000) {
    return `${(bps / 1000000).toFixed(1)} Mbps`
  }
  if (bps >= 1000) {
    return `${(bps / 1000).toFixed(0)} Kbps`
  }
  return `${bps} bps`
}

/** Format sample rate */
function formatSampleRate(hz: number | null): string {
  if (hz === null || hz === undefined) return '-'
  if (hz >= 1000) {
    return `${(hz / 1000).toFixed(1)} kHz`
  }
  return `${hz} Hz`
}

/** Get language display name */
function getLanguageName(code: string | null): string {
  if (!code) return 'Unknown'
  try {
    const displayName = new Intl.DisplayNames(['en'], { type: 'language' })
    return displayName.of(code) || code
  } catch {
    return code
  }
}

/** Format video codec for display */
function formatVideoCodec(codec: string): string {
  const normalized = codec.toLowerCase()
  if (normalized.includes('hevc') || normalized === 'h265') return 'HEVC (H.265)'
  if (normalized.includes('h264') || normalized === 'avc') return 'H.264 (AVC)'
  if (normalized.includes('av1')) return 'AV1'
  if (normalized.includes('vp9')) return 'VP9'
  if (normalized.includes('mpeg4')) return 'MPEG-4'
  if (normalized.includes('mpeg2')) return 'MPEG-2'
  return codec.toUpperCase()
}

/** Format audio codec for display */
function formatAudioCodec(codec: string): string {
  const normalized = codec.toLowerCase()
  if (normalized.includes('truehd')) return 'Dolby TrueHD'
  if (normalized.includes('atmos')) return 'Dolby Atmos'
  if (normalized.includes('dts-hd')) return 'DTS-HD MA'
  if (normalized.includes('dts')) return 'DTS'
  if (normalized.includes('ac3') || normalized.includes('ac-3')) return 'Dolby Digital (AC3)'
  if (normalized.includes('eac3') || normalized.includes('e-ac-3')) return 'Dolby Digital Plus (EAC3)'
  if (normalized.includes('aac')) return 'AAC'
  if (normalized.includes('flac')) return 'FLAC'
  if (normalized.includes('opus')) return 'Opus'
  if (normalized.includes('pcm')) return 'PCM (Lossless)'
  if (normalized.includes('mp3')) return 'MP3'
  return codec.toUpperCase()
}

/** Property row component */
function PropertyRow({ label, value, mono = false }: { label: string; value: React.ReactNode; mono?: boolean }) {
  return (
    <div className="flex justify-between items-center py-2 border-b border-default-100/50 last:border-0">
      <span className="text-default-400 text-sm">{label}</span>
      <span className={`text-sm text-right max-w-[60%] truncate text-default-foreground ${mono ? 'font-mono text-xs' : ''}`}>
        {value || <span className="text-default-300 italic">—</span>}
      </span>
    </div>
  )
}

/** Stream card for video/audio streams */
function StreamCard({ 
  icon, 
  title, 
  subtitle, 
  badges, 
  children 
}: { 
  icon: React.ReactNode
  title: string
  subtitle?: string
  badges?: React.ReactNode
  children: React.ReactNode 
}) {
  return (
    <div className="bg-default-100/50 rounded-xl p-4 mb-3 last:mb-0 border border-default-200/30">
      <div className="flex items-start gap-3 mb-3">
        <div className="text-primary-400 mt-0.5">{icon}</div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-semibold text-sm text-default-foreground">{title}</span>
            {badges}
          </div>
          {subtitle && <span className="text-xs text-default-400 block mt-0.5">{subtitle}</span>}
        </div>
      </div>
      <div className="pl-7">{children}</div>
    </div>
  )
}

export function FilePropertiesModal({ 
  isOpen, 
  onClose, 
  mediaFileId,
  title: overrideTitle,
}: FilePropertiesModalProps) {
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [details, setDetails] = useState<MediaFileDetails | null>(null)
  const [copied, setCopied] = useState(false)
  const [selectedTab, setSelectedTab] = useState('overview')

  useEffect(() => {
    if (!isOpen || !mediaFileId) {
      setDetails(null)
      setError(null)
      return
    }

    const fetchDetails = async () => {
      setLoading(true)
      setError(null)

      const result = await graphqlClient
        .query<{ mediaFileDetails: MediaFileDetails | null }>(MEDIA_FILE_DETAILS_QUERY, { mediaFileId })
        .toPromise()

      if (result.error) {
        setError(result.error.message)
      } else if (result.data?.mediaFileDetails) {
        setDetails(result.data.mediaFileDetails)
      } else {
        setError('Media file not found or not yet analyzed')
      }
      setLoading(false)
    }

    fetchDetails()
  }, [isOpen, mediaFileId])

  const handleCopyPath = async () => {
    if (details?.file.path) {
      await navigator.clipboard.writeText(details.file.path)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }

  const file = details?.file
  const filename = file?.path.split('/').pop() || file?.originalName || 'Unknown'

  return (
    <Modal 
      isOpen={isOpen} 
      onClose={onClose} 
      size="2xl"
      scrollBehavior="inside"
      classNames={{
        base: 'bg-content1',
        header: 'border-b border-default-200/50',
        body: 'py-4',
        footer: 'border-t border-default-200/50',
      }}
    >
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-primary-500/10 rounded-lg">
              <IconFile size={20} className="text-primary-400" />
            </div>
            <div className="flex-1 min-w-0">
              <span className="truncate block font-semibold">{overrideTitle || filename}</span>
              {overrideTitle && file && (
                <span className="text-xs text-default-400 font-normal truncate block mt-0.5">
                  {filename}
                </span>
              )}
            </div>
          </div>
        </ModalHeader>

        <ModalBody>
          {loading ? (
            <div className="flex items-center justify-center py-12">
              <Spinner size="lg" />
            </div>
          ) : error ? (
            <div className="flex flex-col items-center justify-center py-12 gap-3">
              <IconAlertCircle size={48} className="text-warning-400" />
              <p className="text-default-500 text-center">{error}</p>
              <p className="text-default-400 text-sm text-center">
                The file may not have been analyzed yet. Try rescanning the library.
              </p>
            </div>
          ) : details ? (
            <Tabs 
              selectedKey={selectedTab} 
              onSelectionChange={(key) => setSelectedTab(key as string)}
              variant="underlined"
              color="primary"
              classNames={{
                tabList: 'gap-6 w-full relative rounded-none p-0 border-b border-default-200/50',
                cursor: 'w-full bg-primary-500',
                tab: 'max-w-fit px-0 h-10',
                tabContent: 'group-data-[selected=true]:text-primary-500',
              }}
            >
              <Tab
                key="overview"
                title={
                  <div className="flex items-center gap-1.5">
                    <IconInfoCircle size={16} />
                    <span>Overview</span>
                  </div>
                }
              >
                <div className="pt-4 space-y-6">
                  {/* Media Summary Badges - show first as visual highlight */}
                  <div className="flex flex-wrap gap-2">
                    {file?.resolution && (
                      <Chip size="md" variant="flat" color="primary" classNames={{ content: 'font-semibold' }}>
                        {file.resolution}
                      </Chip>
                    )}
                    {file?.videoCodec && (
                      <Chip size="md" variant="flat" color="secondary" classNames={{ content: 'font-semibold' }}>
                        {formatVideoCodec(file.videoCodec)}
                      </Chip>
                    )}
                    {file?.hdrType && (
                      <Chip size="md" variant="flat" color="warning" classNames={{ content: 'font-semibold' }}>
                        {file.hdrType}
                      </Chip>
                    )}
                    {file?.audioCodec && (
                      <Chip size="md" variant="flat" color="default" classNames={{ content: 'font-medium' }}>
                        {formatAudioCodec(file.audioCodec)}
                      </Chip>
                    )}
                  </div>

                  {/* File Info Section */}
                  <div className="bg-default-100/30 rounded-xl p-4 border border-default-200/30">
                    <h4 className="text-sm font-semibold text-primary-400 mb-3 uppercase tracking-wide">File Information</h4>
                    <PropertyRow label="File Name" value={file?.originalName || filename} />
                    <PropertyRow label="Size" value={file?.sizeFormatted} />
                    <PropertyRow label="Container" value={file?.container?.toUpperCase()} />
                    <PropertyRow label="Duration" value={formatDuration(file?.duration ?? null)} />
                    <PropertyRow label="Overall Bitrate" value={formatBitrate(file?.bitrate ?? null)} />
                    <PropertyRow label="Added" value={file?.addedAt ? new Date(file.addedAt).toLocaleString() : null} />
                  </div>
                  
                  {/* Path Section */}
                  <div className="bg-default-100/30 rounded-xl p-4 border border-default-200/30">
                    <div className="flex items-center justify-between mb-2">
                      <h4 className="text-sm font-semibold text-primary-400 uppercase tracking-wide">File Path</h4>
                      <Tooltip content={copied ? 'Copied!' : 'Copy path'}>
                        <Button
                          size="sm"
                          variant="flat"
                          color={copied ? 'success' : 'default'}
                          isIconOnly
                          onPress={handleCopyPath}
                        >
                          {copied ? <IconCheck size={14} /> : <IconCopy size={14} />}
                        </Button>
                      </Tooltip>
                    </div>
                    <code className="text-xs text-default-400 break-all block bg-default-50 p-3 rounded-lg font-mono">
                      {file?.path}
                    </code>
                  </div>

                  {/* Stream Counts */}
                  <div className="grid grid-cols-4 gap-3">
                    <div className="bg-default-100/30 rounded-lg p-3 text-center border border-default-200/30">
                      <div className="text-2xl font-bold text-default-foreground">{details.videoStreams.length}</div>
                      <div className="text-xs text-default-400 mt-1">Video</div>
                    </div>
                    <div className="bg-default-100/30 rounded-lg p-3 text-center border border-default-200/30">
                      <div className="text-2xl font-bold text-default-foreground">{details.audioStreams.length}</div>
                      <div className="text-xs text-default-400 mt-1">Audio</div>
                    </div>
                    <div className="bg-default-100/30 rounded-lg p-3 text-center border border-default-200/30">
                      <div className="text-2xl font-bold text-default-foreground">{details.subtitles.length}</div>
                      <div className="text-xs text-default-400 mt-1">Subtitles</div>
                    </div>
                    <div className="bg-default-100/30 rounded-lg p-3 text-center border border-default-200/30">
                      <div className="text-2xl font-bold text-default-foreground">{details.chapters.length}</div>
                      <div className="text-xs text-default-400 mt-1">Chapters</div>
                    </div>
                  </div>
                </div>
              </Tab>

              <Tab
                key="video"
                title={
                  <div className="flex items-center gap-1.5">
                    <IconVideo size={16} />
                    <span>Video ({details.videoStreams.length})</span>
                  </div>
                }
              >
                <div className="pt-4">
                  {details.videoStreams.length === 0 ? (
                    <div className="flex flex-col items-center justify-center py-12 text-default-400">
                      <IconVideo size={48} className="mb-2 opacity-50" />
                      <p>No video streams found</p>
                    </div>
                  ) : (
                    <div className="space-y-3">
                      {details.videoStreams.map((stream) => (
                        <VideoStreamCard key={stream.id} stream={stream} />
                      ))}
                    </div>
                  )}
                </div>
              </Tab>

              <Tab
                key="audio"
                title={
                  <div className="flex items-center gap-1.5">
                    <IconVolume size={16} />
                    <span>Audio ({details.audioStreams.length})</span>
                  </div>
                }
              >
                <div className="pt-4">
                  {details.audioStreams.length === 0 ? (
                    <div className="flex flex-col items-center justify-center py-12 text-default-400">
                      <IconVolume size={48} className="mb-2 opacity-50" />
                      <p>No audio streams found</p>
                    </div>
                  ) : (
                    <div className="space-y-3">
                      {details.audioStreams.map((stream) => (
                        <AudioStreamCard key={stream.id} stream={stream} />
                      ))}
                    </div>
                  )}
                </div>
              </Tab>

              <Tab
                key="subtitles"
                title={
                  <div className="flex items-center gap-1.5">
                    <IconFileText size={16} />
                    <span>Subtitles ({details.subtitles.length})</span>
                  </div>
                }
              >
                <div className="pt-4">
                  {details.subtitles.length === 0 ? (
                    <div className="flex flex-col items-center justify-center py-12 text-default-400">
                      <IconFileText size={48} className="mb-2 opacity-50" />
                      <p>No subtitles found</p>
                    </div>
                  ) : (
                    <div className="space-y-3">
                      {details.subtitles.map((sub) => (
                        <SubtitleCard key={sub.id} subtitle={sub} />
                      ))}
                    </div>
                  )}
                </div>
              </Tab>

              <Tab
                key="chapters"
                title={
                  <div className="flex items-center gap-1.5">
                    <IconList size={16} />
                    <span>Chapters ({details.chapters.length})</span>
                  </div>
                }
              >
                <div className="pt-4">
                  {details.chapters.length === 0 ? (
                    <div className="flex flex-col items-center justify-center py-12 text-default-400">
                      <IconList size={48} className="mb-2 opacity-50" />
                      <p>No chapters found</p>
                    </div>
                  ) : (
                    <div className="space-y-2">
                      {details.chapters.map((chapter) => (
                        <ChapterRow key={chapter.id} chapter={chapter} />
                      ))}
                    </div>
                  )}
                </div>
              </Tab>
            </Tabs>
          ) : null}
        </ModalBody>

        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Close
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}

/** Video stream details card */
function VideoStreamCard({ stream }: { stream: VideoStreamInfo }) {
  return (
    <StreamCard
      icon={<IconVideo size={16} />}
      title={formatVideoCodec(stream.codec)}
      subtitle={stream.codecLongName || undefined}
      badges={
        <>
          {stream.isDefault && <Chip size="sm" variant="flat" color="primary">Default</Chip>}
          {stream.hdrType && <Chip size="sm" variant="flat" color="warning">{stream.hdrType}</Chip>}
        </>
      }
    >
      <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
        <PropertyRow label="Resolution" value={`${stream.width}×${stream.height}`} />
        <PropertyRow label="Aspect Ratio" value={stream.aspectRatio} />
        <PropertyRow label="Frame Rate" value={stream.frameRate} />
        <PropertyRow label="Bitrate" value={formatBitrate(stream.bitrate)} />
        <PropertyRow label="Pixel Format" value={stream.pixelFormat} />
        <PropertyRow label="Bit Depth" value={stream.bitDepth ? `${stream.bitDepth}-bit` : null} />
        {stream.language && <PropertyRow label="Language" value={getLanguageName(stream.language)} />}
        {stream.title && <PropertyRow label="Title" value={stream.title} />}
      </div>
    </StreamCard>
  )
}

/** Audio stream details card */
function AudioStreamCard({ stream }: { stream: AudioStreamInfo }) {
  return (
    <StreamCard
      icon={<IconVolume size={16} />}
      title={formatAudioCodec(stream.codec)}
      subtitle={stream.codecLongName || undefined}
      badges={
        <>
          {stream.isDefault && <Chip size="sm" variant="flat" color="primary">Default</Chip>}
          {stream.isCommentary && <Chip size="sm" variant="flat" color="secondary">Commentary</Chip>}
          {stream.language && (
            <Chip size="sm" variant="flat" color="default">{getLanguageName(stream.language)}</Chip>
          )}
        </>
      }
    >
      <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
        <PropertyRow label="Channels" value={stream.channelLayout || `${stream.channels} ch`} />
        <PropertyRow label="Sample Rate" value={formatSampleRate(stream.sampleRate)} />
        <PropertyRow label="Bitrate" value={formatBitrate(stream.bitrate)} />
        <PropertyRow label="Bit Depth" value={stream.bitDepth ? `${stream.bitDepth}-bit` : null} />
        {stream.title && <PropertyRow label="Title" value={stream.title} />}
      </div>
    </StreamCard>
  )
}

/** Subtitle track card */
function SubtitleCard({ subtitle }: { subtitle: SubtitleInfo }) {
  const sourceLabel = {
    EMBEDDED: 'Embedded',
    EXTERNAL: 'External File',
    DOWNLOADED: 'Downloaded',
  }[subtitle.sourceType] || subtitle.sourceType

  return (
    <StreamCard
      icon={<IconFileText size={16} />}
      title={subtitle.language ? getLanguageName(subtitle.language) : 'Unknown Language'}
      subtitle={subtitle.codec || undefined}
      badges={
        <>
          <Chip size="sm" variant="flat" color="default">{sourceLabel}</Chip>
          {subtitle.isDefault && <Chip size="sm" variant="flat" color="primary">Default</Chip>}
          {subtitle.isForced && <Chip size="sm" variant="flat" color="warning">Forced</Chip>}
          {subtitle.isHearingImpaired && <Chip size="sm" variant="flat" color="secondary">SDH</Chip>}
        </>
      }
    >
      <div className="text-xs">
        {subtitle.title && <PropertyRow label="Title" value={subtitle.title} />}
        {subtitle.filePath && (
          <PropertyRow label="File" value={subtitle.filePath.split('/').pop()} />
        )}
      </div>
    </StreamCard>
  )
}

/** Chapter row */
function ChapterRow({ chapter }: { chapter: ChapterInfo }) {
  const duration = chapter.endSecs - chapter.startSecs
  return (
    <div className="flex items-center gap-3 py-2.5 px-4 bg-default-100/30 rounded-lg hover:bg-default-100/50 transition-colors border border-default-200/20">
      <span className="text-default-400 text-xs w-6 font-medium">{chapter.chapterIndex + 1}</span>
      <span className="flex-1 text-sm truncate text-default-foreground">
        {chapter.title || `Chapter ${chapter.chapterIndex + 1}`}
      </span>
      <span className="text-xs text-default-400 font-mono tabular-nums">
        {formatDuration(chapter.startSecs)}
      </span>
      <span className="text-xs text-default-300">→</span>
      <span className="text-xs text-default-400 font-mono tabular-nums">
        {formatDuration(chapter.endSecs)}
      </span>
      <span className="text-xs text-primary-400 w-16 text-right font-medium tabular-nums">
        ({formatDuration(duration)})
      </span>
    </div>
  )
}

export default FilePropertiesModal
