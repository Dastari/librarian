import { useRef } from 'react'
import { CastButton } from './cast'
import { useHlsMediaSource } from '../hooks/useHlsMediaSource'
export { getMediaStreamUrl, resolveMediaPlaybackUrl, resolveMediaPlaybackSource } from '../lib/api/mediaPlayback'

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

  // Handles both HLS (.m3u8, via hls.js/native Safari) and direct-play src.
  useHlsMediaSource(videoRef, src, onError)

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
            onCastStart={() => {
              if (videoRef.current) {
                videoRef.current.pause();
              }
            }}
            size="sm"
          />
        </div>
      )}
    </div>
  )
}

