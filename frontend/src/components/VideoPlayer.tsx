import { useEffect, useRef } from 'react'
import Hls from 'hls.js'
import { CastButton } from './cast'

interface VideoPlayerProps {
  /** Video source URL (direct file or HLS m3u8) */
  src: string
  /** Poster image URL */
  poster?: string
  /** Media file ID for casting */
  mediaFileId?: string
  /** Episode ID for tracking */
  episodeId?: string
  /** Called on playback error */
  onError?: (error: Error) => void
  /** Show cast button */
  showCastButton?: boolean
}

/**
 * Video player component with HLS support and Chromecast casting
 * 
 * Supports:
 * - Direct playback of MP4, WebM, and other browser-native formats
 * - HLS streaming via hls.js (m3u8 playlists)
 * - Native Safari HLS support
 * - AirPlay on supported devices
 * - Chromecast/Google Cast via CastButton
 */
export function VideoPlayer({ 
  src, 
  poster, 
  mediaFileId,
  episodeId,
  onError,
  showCastButton = true,
}: VideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const hlsRef = useRef<Hls | null>(null)

  useEffect(() => {
    const video = videoRef.current
    if (!video || !src) return

    // Check if HLS is needed
    if (src.includes('.m3u8')) {
      if (Hls.isSupported()) {
        const hls = new Hls({
          enableWorker: true,
          lowLatencyMode: false,
        })
        
        hls.loadSource(src)
        hls.attachMedia(video)
        
        hls.on(Hls.Events.ERROR, (_event, data) => {
          if (data.fatal) {
            onError?.(new Error(`HLS fatal error: ${data.type}`))
          }
        })
        
        hlsRef.current = hls
        
        return () => {
          hls.destroy()
          hlsRef.current = null
        }
      } else if (video.canPlayType('application/vnd.apple.mpegurl')) {
        // Native HLS support (Safari)
        video.src = src
        return () => {
          // Cleanup for native HLS: abort network requests
          video.pause()
          video.src = ''
          video.load()
        }
      } else {
        onError?.(new Error('HLS not supported'))
      }
    } else {
      // Direct play
      video.src = src
      return () => {
        // Cleanup for direct play: abort network requests
        video.pause()
        video.src = ''
        video.load()
      }
    }
  }, [src, onError])

  return (
    <div className="relative bg-black rounded-lg overflow-hidden group">
      <video
        ref={videoRef}
        className="w-full aspect-video"
        controls
        poster={poster}
        playsInline
        // Enable AirPlay for Safari
        // @ts-ignore
        x-webkit-airplay="allow"
      >
        Your browser does not support the video tag.
      </video>
      
      {/* Cast button overlay */}
      {showCastButton && mediaFileId && (
        <div className="absolute top-3 right-3 opacity-0 group-hover:opacity-100 transition-opacity">
          <CastButton
            mediaFileId={mediaFileId}
            episodeId={episodeId}
            startPosition={videoRef.current?.currentTime}
            size="sm"
          />
        </div>
      )}
    </div>
  )
}

/**
 * Helper to generate the stream URL for a media file
 */
export function getMediaStreamUrl(mediaFileId: string): string {
  const apiUrl = import.meta.env.VITE_API_URL || '';
  return `${apiUrl}/api/media/${mediaFileId}/stream`;
}
