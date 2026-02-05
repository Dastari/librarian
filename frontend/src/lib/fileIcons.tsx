import type { ReactNode } from 'react'
import {
  IconFile,
  IconFileMusic,
  IconFileText,
  IconFileZip,
  IconFolder,
  IconMovie,
  IconPhoto,
} from '@tabler/icons-react'

type FileIconOptions = {
  size?: number
  className?: string
}

const VIDEO_EXTENSIONS = new Set(['mkv', 'mp4', 'avi', 'mov', 'wmv', 'webm', 'm4v'])
const AUDIO_EXTENSIONS = new Set(['mp3', 'flac', 'aac', 'm4a', 'ogg', 'opus', 'wav'])
const IMAGE_EXTENSIONS = new Set(['jpg', 'jpeg', 'png', 'gif', 'webp', 'avif'])
const SUBTITLE_EXTENSIONS = new Set(['srt', 'sub', 'ass', 'ssa', 'vtt'])
const ARCHIVE_EXTENSIONS = new Set(['zip', 'rar', '7z', 'tar', 'gz', 'bz2'])
const TEXT_EXTENSIONS = new Set(['nfo', 'txt', 'log', 'md'])
const DOCUMENT_EXTENSIONS = new Set(['pdf', 'doc', 'docx', 'rtf'])

function getExtension(filePath: string): string | null {
  const base = filePath.split(/[/\\]/).pop()
  if (!base || !base.includes('.')) return null
  const ext = base.split('.').pop()
  return ext ? ext.toLowerCase() : null
}

export function getFileIcon(filePath: string, isDir = false, options: FileIconOptions = {}): ReactNode {
  const size = options.size ?? 18
  if (isDir) {
    return <IconFolder size={size} className={options.className ?? 'text-amber-400'} />
  }

  const ext = getExtension(filePath)
  if (ext && VIDEO_EXTENSIONS.has(ext)) {
    return <IconMovie size={size} className={options.className ?? 'text-purple-400'} />
  }
  if (ext && AUDIO_EXTENSIONS.has(ext)) {
    return <IconFileMusic size={size} className={options.className ?? 'text-cyan-400'} />
  }
  if (ext && IMAGE_EXTENSIONS.has(ext)) {
    return <IconPhoto size={size} className={options.className ?? 'text-green-400'} />
  }
  if (ext && SUBTITLE_EXTENSIONS.has(ext)) {
    return <IconFileText size={size} className={options.className ?? 'text-default-400'} />
  }
  if (ext && ARCHIVE_EXTENSIONS.has(ext)) {
    return <IconFileZip size={size} className={options.className ?? 'text-orange-400'} />
  }
  if (ext && (TEXT_EXTENSIONS.has(ext) || DOCUMENT_EXTENSIONS.has(ext))) {
    return <IconFileText size={size} className={options.className ?? 'text-blue-400'} />
  }

  return <IconFile size={size} className={options.className ?? 'text-default-400'} />
}
