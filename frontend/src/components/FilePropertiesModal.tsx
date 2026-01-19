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
import { Divider } from '@heroui/divider'
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
    <div className="flex justify-between py-1.5 border-b border-default-100 last:border-0">
      <span className="text-default-500 text-sm">{label}</span>
      <span className={`text-sm text-right max-w-[60%] truncate ${mono ? 'font-mono text-xs' : ''}`}>
        {value || <span className="text-default-400">-</span>}
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
    <div className="bg-default-50 rounded-lg p-3 mb-2 last:mb-0">
      <div className="flex items-start gap-2 mb-2">
        <div className="text-default-400 mt-0.5">{icon}</div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-medium text-sm">{title}</span>
            {badges}
          </div>
          {subtitle && <span className="text-xs text-default-400">{subtitle}</span>}
        </div>
      </div>
      <div className="pl-6">{children}</div>
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
    >
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <IconFile size={20} className="text-default-400" />
            <span className="truncate">{overrideTitle || filename}</span>
          </div>
          {overrideTitle && file && (
            <span className="text-xs text-default-400 font-normal truncate">
              {filename}
            </span>
          )}
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
              classNames={{
                tabList: 'gap-4',
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
                <div className="pt-3">
                  {/* File Info */}
                  <h4 className="text-sm font-semibold text-default-600 mb-2">File Information</h4>
                  <PropertyRow label="File Name" value={file?.originalName || filename} />
                  <PropertyRow label="Size" value={file?.sizeFormatted} />
                  <PropertyRow label="Container" value={file?.container?.toUpperCase()} />
                  <PropertyRow label="Duration" value={formatDuration(file?.duration ?? null)} />
                  <PropertyRow label="Overall Bitrate" value={formatBitrate(file?.bitrate ?? null)} />
                  <PropertyRow label="Added" value={file?.addedAt ? new Date(file.addedAt).toLocaleString() : '-'} />
                  
                  <div className="flex items-center gap-2 mt-2">
                    <span className="text-default-500 text-sm">Path</span>
                    <Tooltip content={copied ? 'Copied!' : 'Copy path'}>
                      <Button
                        size="sm"
                        variant="light"
                        isIconOnly
                        onPress={handleCopyPath}
                      >
                        {copied ? <IconCheck size={14} className="text-success" /> : <IconCopy size={14} />}
                      </Button>
                    </Tooltip>
                  </div>
                  <code className="text-xs text-default-400 break-all block mt-1 bg-default-100 p-2 rounded">
                    {file?.path}
                  </code>

                  <Divider className="my-4" />

                  {/* Quick Summary */}
                  <h4 className="text-sm font-semibold text-default-600 mb-2">Media Summary</h4>
                  <div className="flex flex-wrap gap-2 mb-3">
                    {file?.resolution && (
                      <Chip size="sm" variant="flat" color="primary">{file.resolution}</Chip>
                    )}
                    {file?.videoCodec && (
                      <Chip size="sm" variant="flat" color="secondary">
                        {formatVideoCodec(file.videoCodec)}
                      </Chip>
                    )}
                    {file?.hdrType && (
                      <Chip size="sm" variant="flat" color="warning">{file.hdrType}</Chip>
                    )}
                    {file?.audioCodec && (
                      <Chip size="sm" variant="flat" color="default">
                        {formatAudioCodec(file.audioCodec)}
                      </Chip>
                    )}
                  </div>
                  
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <span className="text-default-500">Video Streams:</span>{' '}
                      <span className="font-medium">{details.videoStreams.length}</span>
                    </div>
                    <div>
                      <span className="text-default-500">Audio Streams:</span>{' '}
                      <span className="font-medium">{details.audioStreams.length}</span>
                    </div>
                    <div>
                      <span className="text-default-500">Subtitles:</span>{' '}
                      <span className="font-medium">{details.subtitles.length}</span>
                    </div>
                    <div>
                      <span className="text-default-500">Chapters:</span>{' '}
                      <span className="font-medium">{details.chapters.length}</span>
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
                <div className="pt-3">
                  {details.videoStreams.length === 0 ? (
                    <p className="text-default-400 text-center py-8">No video streams found</p>
                  ) : (
                    details.videoStreams.map((stream) => (
                      <VideoStreamCard key={stream.id} stream={stream} />
                    ))
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
                <div className="pt-3">
                  {details.audioStreams.length === 0 ? (
                    <p className="text-default-400 text-center py-8">No audio streams found</p>
                  ) : (
                    details.audioStreams.map((stream) => (
                      <AudioStreamCard key={stream.id} stream={stream} />
                    ))
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
                <div className="pt-3">
                  {details.subtitles.length === 0 ? (
                    <p className="text-default-400 text-center py-8">No subtitles found</p>
                  ) : (
                    details.subtitles.map((sub) => (
                      <SubtitleCard key={sub.id} subtitle={sub} />
                    ))
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
                <div className="pt-3">
                  {details.chapters.length === 0 ? (
                    <p className="text-default-400 text-center py-8">No chapters found</p>
                  ) : (
                    <div className="space-y-1">
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
    <div className="flex items-center gap-3 py-2 px-3 bg-default-50 rounded hover:bg-default-100 transition-colors">
      <span className="text-default-400 text-xs w-6">{chapter.chapterIndex + 1}</span>
      <span className="flex-1 text-sm truncate">
        {chapter.title || `Chapter ${chapter.chapterIndex + 1}`}
      </span>
      <span className="text-xs text-default-400 font-mono">
        {formatDuration(chapter.startSecs)}
      </span>
      <span className="text-xs text-default-300">→</span>
      <span className="text-xs text-default-400 font-mono">
        {formatDuration(chapter.endSecs)}
      </span>
      <span className="text-xs text-default-300 w-16 text-right">
        ({formatDuration(duration)})
      </span>
    </div>
  )
}

export default FilePropertiesModal
