import { useEffect, useRef } from 'react'
import Hls from 'hls.js'

interface VideoPlayerProps {
  src: string
  poster?: string
  onError?: (error: Error) => void
}

export function VideoPlayer({ src, poster, onError }: VideoPlayerProps) {
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
      } else {
        onError?.(new Error('HLS not supported'))
      }
    } else {
      // Direct play
      video.src = src
    }
  }, [src, onError])

  return (
    <div className="relative bg-black rounded-lg overflow-hidden">
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
    </div>
  )
}
