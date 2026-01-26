/**
 * Persistent video player component
 * 
 * Shows as a floating mini-player in the corner when something is playing.
 * Expands to a 16:9 modal on click. Uses a SINGLE video element that
 * transitions between states so playback continues seamlessly.
 */

import { useState, useRef, useEffect, useCallback } from 'react';
import { Button } from '@heroui/button';
import { Spinner } from '@heroui/spinner';
import {
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerStop,
  IconMinimize,
  IconMaximize,
} from '@tabler/icons-react';
import { usePlaybackContext } from '../contexts/PlaybackContext';
import { CastButton } from './cast';
import { VolumeControl } from './VolumeControl';
import { getMediaStreamUrl } from './VideoPlayer';
import type { Show } from '../lib/graphql/generated/graphql';
import { graphqlClient, TV_SHOW_QUERY, EPISODES_QUERY, PlaybackSyncIntervalDocument, type Episode } from '../lib/graphql';
import { useCast } from '../hooks/useCast';

// Default sync interval (will be overridden by settings)
const DEFAULT_SYNC_INTERVAL = 15000;

function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }
  return `${m}:${s.toString().padStart(2, '0')}`;
}

export function PersistentPlayer() {
  const {
    session, isLoading, updatePlayback, stopPlayback,
    currentEpisode, currentShow, setCurrentEpisode, setCurrentShow,
    shouldExpand, clearExpandFlag,
  } = usePlaybackContext();
  const {
    activeSession: castSession,
    play: castPlay,
    pause: castPause,
    stop: castStop,
    seek: castSeek,
    setVolume: castSetVolume,
    setMuted: castSetMuted,
  } = useCast();

  const [isExpanded, setIsExpanded] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(1);
  const [isMuted, setIsMuted] = useState(false);
  const [showControls, setShowControls] = useState(true);
  const [videoReady, setVideoReady] = useState(false);
  const [syncInterval, setSyncInterval] = useState(DEFAULT_SYNC_INTERVAL);
  const [isPaused, setIsPaused] = useState(true);
  
  const videoRef = useRef<HTMLVideoElement>(null);
  const syncIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const controlsTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  // Only handle video content (EPISODE, MOVIE), not audio (TRACK, AUDIOBOOK)
  const isVideoSession = session?.contentType === 'EPISODE' || session?.contentType === 'MOVIE';
  const isCastingThisMedia = Boolean(
    castSession?.mediaFileId && session?.mediaFileId && castSession.mediaFileId === session.mediaFileId
  );
  const isCastingPlaying = isCastingThisMedia && castSession?.playerState === 'PLAYING';

  // Fetch playback sync interval from app settings
  useEffect(() => {
    graphqlClient
      .query(PlaybackSyncIntervalDocument, { Key: 'playback_sync_interval' })
      .toPromise()
      .then((result) => {
        const value = result.data?.AppSettings?.Edges?.[0]?.Node?.Value;
        if (value != null) {
          const seconds = Number(value);
          if (Number.isFinite(seconds)) setSyncInterval(seconds * 1000);
        }
      })
      .catch(() => {
        // Use default on error
      });
  }, []);

  // Expand to dialog view when shouldExpand is set (triggered by startPlayback)
  useEffect(() => {
    if (shouldExpand) {
      setIsExpanded(true);
      clearExpandFlag();
    }
  }, [shouldExpand, clearExpandFlag]);

  // Load metadata when session changes
  useEffect(() => {
    if (session?.tvShowId && session?.episodeId && !currentShow) {
      Promise.all([
        graphqlClient.query<{ Show: Show | null }>(TV_SHOW_QUERY, { Id: session.tvShowId }).toPromise(),
        graphqlClient.query<{ episodes: Episode[] }>(EPISODES_QUERY, { tvShowId: session.tvShowId }).toPromise(),
      ]).then(([showRes, epRes]) => {
        if (showRes.data?.Show) setCurrentShow(showRes.data.Show as unknown as Parameters<typeof setCurrentShow>[0]);
        const ep = epRes.data?.episodes?.find(e => e.id === session.episodeId);
        if (ep) setCurrentEpisode(ep);
      });
    }
  }, [session?.tvShowId, session?.episodeId, currentShow, setCurrentShow, setCurrentEpisode]);

  // Resume from session position
  useEffect(() => {
    if (!isVideoSession) return;
    if (session && videoRef.current && session.currentPosition > 0) {
      videoRef.current.currentTime = session.currentPosition;
      setCurrentTime(session.currentPosition);
      if (session.duration) setDuration(session.duration);
      setIsMuted(session.isMuted);
    }
  }, [session?.id, isVideoSession]);

  // Auto-play when video is ready and session indicates playing
  useEffect(() => {
    if (!isVideoSession) return;
    if (isCastingThisMedia && videoRef.current && !videoRef.current.paused) {
      videoRef.current.pause();
      setIsPaused(true);
      return;
    }
    if (session?.isPlaying && videoReady && videoRef.current && videoRef.current.paused) {
      videoRef.current.play().catch(() => {});
    }
  }, [session?.isPlaying, videoReady, isVideoSession, isCastingThisMedia]);

  // Sync position periodically (using configurable interval from settings)
  useEffect(() => {
    if (!isVideoSession) return;
    if (isCastingThisMedia) return;
    if (session && !isPaused) {
      syncIntervalRef.current = setInterval(() => {
        if (videoRef.current) {
          updatePlayback({
            currentPosition: videoRef.current.currentTime,
            duration: videoRef.current.duration || undefined,
            isPlaying: !videoRef.current.paused,
          });
        }
      }, syncInterval);
    }
    return () => { if (syncIntervalRef.current) clearInterval(syncIntervalRef.current); };
  }, [session, isPaused, updatePlayback, syncInterval, isVideoSession, isCastingThisMedia]);

  // Cleanup video stream when mediaFileId changes or component unmounts
  // This is critical to abort the HTTP connection and stop network traffic
  useEffect(() => {
    if (!isVideoSession) return;
    const video = videoRef.current;
    // Cleanup runs when mediaFileId changes or on unmount
    return () => {
      if (video) {
        video.pause();
        video.src = '';
        video.load(); // Abort any in-flight network requests
      }
    };
  }, [session?.mediaFileId, isVideoSession]);

  // Sync isPaused state with actual video state
  const syncPausedState = useCallback(() => {
    if (videoRef.current) {
      setIsPaused(videoRef.current.paused);
    }
  }, []);

  const handleTimeUpdate = useCallback(() => {
    if (videoRef.current) {
      setCurrentTime(videoRef.current.currentTime);
      syncPausedState();
    }
  }, [syncPausedState]);

  const handleLoadedMetadata = useCallback(() => {
    if (videoRef.current) {
      setDuration(videoRef.current.duration);
      if (session?.currentPosition) videoRef.current.currentTime = session.currentPosition;
      syncPausedState();
    }
  }, [session?.currentPosition, syncPausedState]);

  const handleCanPlay = useCallback(() => {
    setVideoReady(true);
    syncPausedState();
  }, [syncPausedState]);

  const handlePlay = useCallback(() => {
    if (isCastingThisMedia) {
      castPlay().catch(() => {});
      return;
    }
    setIsPaused(false);
    updatePlayback({ isPlaying: true });
  }, [updatePlayback, isCastingThisMedia, castPlay]);

  const handlePause = useCallback(() => {
    if (isCastingThisMedia) {
      castPause().catch(() => {});
      return;
    }
    setIsPaused(true);
    if (videoRef.current) {
      updatePlayback({ isPlaying: false, currentPosition: videoRef.current.currentTime });
    }
  }, [updatePlayback, isCastingThisMedia, castPause]);

  const togglePlay = useCallback(() => {
    if (isCastingThisMedia) {
      if (isCastingPlaying) {
        castPause().catch(() => {});
      } else {
        castPlay().catch(() => {});
      }
      return;
    }
    if (!videoRef.current) return;
    if (videoRef.current.paused) {
      videoRef.current.play().catch(() => {});
    } else {
      videoRef.current.pause();
    }
  }, [isCastingThisMedia, isCastingPlaying, castPause, castPlay]);

  const handleStop = useCallback(async () => {
    if (isCastingThisMedia) {
      await castStop();
      return;
    }
    // Stop the video and release resources
    if (videoRef.current) {
      videoRef.current.pause();
      videoRef.current.src = '';
      videoRef.current.load(); // Release the stream
    }
    // Reset local state
    setIsExpanded(false);
    setIsPaused(true);
    setVideoReady(false);
    setCurrentTime(0);
    setDuration(0);
    // Stop the playback session
    await stopPlayback();
  }, [stopPlayback, isCastingThisMedia, castStop]);

  const toggleMute = useCallback(() => {
    if (isCastingThisMedia) {
      castSetMuted(!castSession?.isMuted).catch(() => {});
      return;
    }
    if (!videoRef.current) return;
    videoRef.current.muted = !videoRef.current.muted;
    setIsMuted(videoRef.current.muted);
    updatePlayback({ isMuted: videoRef.current.muted });
  }, [updatePlayback, isCastingThisMedia, castSetMuted, castSession?.isMuted]);

  const handleVolumeChange = useCallback((newVolume: number) => {
    if (isCastingThisMedia) {
      castSetVolume(newVolume).catch(() => {});
      return;
    }
    if (!videoRef.current) return;
    videoRef.current.volume = newVolume;
    setVolume(newVolume);
    // If adjusting volume, unmute
    if (newVolume > 0 && isMuted) {
      videoRef.current.muted = false;
      setIsMuted(false);
      updatePlayback({ isMuted: false });
    }
  }, [isMuted, updatePlayback, isCastingThisMedia, castSetVolume]);

  const handleSeek = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (isCastingThisMedia) {
      const castDuration = castSession?.duration || 0;
      if (castDuration <= 0) return;
      const rect = e.currentTarget.getBoundingClientRect();
      const percent = (e.clientX - rect.left) / rect.width;
      const newTime = percent * castDuration;
      castSeek(newTime).catch(() => {});
      return;
    }
    if (!videoRef.current || duration <= 0) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const percent = (e.clientX - rect.left) / rect.width;
    const newTime = percent * duration;
    videoRef.current.currentTime = newTime;
    setCurrentTime(newTime);
    updatePlayback({ currentPosition: newTime });
  }, [duration, updatePlayback, isCastingThisMedia, castSeek, castSession?.duration]);

  const handleFullscreen = useCallback(() => {
    if (videoRef.current) {
      if (videoRef.current.requestFullscreen) {
        videoRef.current.requestFullscreen();
      }
    }
  }, []);

  const showControlsTemp = useCallback(() => {
    setShowControls(true);
    if (controlsTimeoutRef.current) clearTimeout(controlsTimeoutRef.current);
    controlsTimeoutRef.current = setTimeout(() => { if (!isPaused) setShowControls(false); }, 3000);
  }, [isPaused]);

  useEffect(() => () => { if (controlsTimeoutRef.current) clearTimeout(controlsTimeoutRef.current); }, []);

  // Keyboard controls when expanded (only for video content)
  useEffect(() => {
    if (!isExpanded || !isVideoSession) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't handle if typing in an input
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

      switch (e.key) {
        case ' ':
        case 'k':
          e.preventDefault();
          togglePlay();
          break;
        case 'Escape':
          e.preventDefault();
          setIsExpanded(false);
          break;
        case 'm':
          e.preventDefault();
          toggleMute();
          break;
        case 'f':
          e.preventDefault();
          handleFullscreen();
          break;
        case 'ArrowLeft':
          e.preventDefault();
          if (videoRef.current) {
            videoRef.current.currentTime = Math.max(0, videoRef.current.currentTime - 10);
          }
          break;
        case 'ArrowRight':
          e.preventDefault();
          if (videoRef.current) {
            videoRef.current.currentTime = Math.min(duration, videoRef.current.currentTime + 10);
          }
          break;
        case 'ArrowUp':
          e.preventDefault();
          if (videoRef.current) {
            videoRef.current.volume = Math.min(1, videoRef.current.volume + 0.1);
          }
          break;
        case 'ArrowDown':
          e.preventDefault();
          if (videoRef.current) {
            videoRef.current.volume = Math.max(0, videoRef.current.volume - 0.1);
          }
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isExpanded, isVideoSession, togglePlay, toggleMute, handleFullscreen, duration]);

  // Don't render for audio content or when no session
  if (isLoading || !session?.mediaFileId || !isVideoSession) return null;

  const displayCurrentTime = isCastingThisMedia ? castSession?.currentTime || 0 : currentTime;
  const displayDuration = isCastingThisMedia ? castSession?.duration || 0 : duration;
  const progress = displayDuration > 0 ? (displayCurrentTime / displayDuration) * 100 : 0;
  const epTitle = currentEpisode 
    ? `S${String(currentEpisode.season).padStart(2, '0')}E${String(currentEpisode.episode).padStart(2, '0')}${currentEpisode.title ? ` - ${currentEpisode.title}` : ''}`
    : 'Episode';

  return (
    <>
      {/* Backdrop when expanded */}
      {isExpanded && (
        <div 
          className="fixed inset-0 bg-black/80 z-40 backdrop-blur-sm"
          onClick={() => setIsExpanded(false)}
        />
      )}

      {/* Single container that transitions between mini and expanded */}
      <div 
        className={`fixed z-50 transition-all duration-300 ease-in-out ${
          isExpanded 
            ? 'top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[90vw] max-w-[1280px]'
            : 'bottom-4 right-4 w-72 translate-x-0 translate-y-0'
        }`}
        onMouseMove={isExpanded ? showControlsTemp : undefined}
        onMouseLeave={isExpanded && !isPaused ? () => setShowControls(false) : undefined}
      >
        {/* Aspect ratio container */}
        <div 
          className={`relative w-full bg-black rounded-t-lg overflow-hidden shadow-2xl ${
            isExpanded ? '' : ''
          }`}
          style={isExpanded ? { paddingBottom: '56.25%' } : {}}
        >
          {/* Video wrapper */}
          <div className={isExpanded ? 'absolute inset-0' : ''}>
            {/* Video element - SINGLE instance */}
            <video
              ref={videoRef}
              src={getMediaStreamUrl(session.mediaFileId!)}
              className={`w-full ${isExpanded ? 'h-full object-contain' : 'h-40 object-cover cursor-pointer'}`}
              onTimeUpdate={handleTimeUpdate}
              onLoadedMetadata={handleLoadedMetadata}
              onCanPlay={handleCanPlay}
              onPlay={handlePlay}
              onPause={handlePause}
              onEnded={handleStop}
              onError={() => setVideoReady(false)}
              onClick={isExpanded ? undefined : () => setIsExpanded(true)}
              playsInline
              muted={isMuted}
            />

            {/* Loading spinner overlay */}
            {!videoReady && (
              <div className={`absolute inset-0 flex items-center justify-center bg-black/60 ${isExpanded ? '' : 'h-40'}`}>
                <Spinner size="lg" color="white" />
              </div>
            )}
            {isCastingThisMedia && (
              <div className={`absolute inset-0 flex items-center justify-center bg-black/60 ${isExpanded ? '' : 'h-40'}`}>
                <div className="text-white text-sm font-medium">
                  Casting to {castSession?.deviceName || 'device'}
                </div>
              </div>
            )}

            {/* Expanded player: controls overlay */}
            {isExpanded && (
              <>
                {/* Top bar */}
                <div className={`absolute inset-x-0 top-0 transition-opacity duration-300 ${showControls ? 'opacity-100' : 'opacity-0 pointer-events-none'}`}>
                  <div className="bg-gradient-to-b from-black/70 to-transparent p-4 flex items-center justify-between">
                    <div className="flex-1 min-w-0">
                      <h2 className="text-white text-lg font-semibold truncate">{(currentShow as Show | null)?.Name || 'Show'}</h2>
                      <p className="text-white/70 text-sm truncate">{epTitle}</p>
                    </div>
                    <div className="flex items-center">
                      {session.mediaFileId && (
                        <CastButton 
                          mediaFileId={session.mediaFileId} 
                          episodeId={session.episodeId || undefined} 
                          startPosition={currentTime} 
                          size="sm"
                          variant="light"
                          className="text-white"
                        />
                      )}
                      <Button isIconOnly variant="light" className="text-white" onPress={handleFullscreen} aria-label="Fullscreen">
                        <IconMaximize size={20} />
                      </Button>
                      <Button isIconOnly variant="light" className="text-white" onPress={() => setIsExpanded(false)} aria-label="Minimize player">
                        <IconMinimize size={20} />
                      </Button>
                    </div>
                  </div>
                </div>

                {/* Bottom bar */}
                <div className={`absolute inset-x-0 bottom-0 transition-opacity duration-300 ${showControls ? 'opacity-100' : 'opacity-0 pointer-events-none'}`}>
                  <div className="bg-gradient-to-t from-black/70 to-transparent p-4">
                    {/* Progress bar */}
                    <div 
                      className="w-full h-1.5 bg-white/30 rounded-full cursor-pointer mb-3 group hover:h-2 transition-all" 
                      onClick={handleSeek}
                    >
                      <div className="h-full bg-primary rounded-full relative" style={{ width: `${progress}%` }}>
                        <div className="absolute right-0 top-1/2 -translate-y-1/2 w-3 h-3 bg-white rounded-full opacity-0 group-hover:opacity-100 transition-opacity" />
                      </div>
                    </div>

                    {/* Controls row */}
                    <div className="flex items-center gap-2 text-white">
                      {/* Playback controls group */}
                      <div className="flex items-center">
                        <Button isIconOnly variant="light" className="text-white" onPress={togglePlay} aria-label={isCastingThisMedia ? (isCastingPlaying ? "Pause" : "Play") : (isPaused ? "Play" : "Pause")}>
                          {isCastingThisMedia ? (isCastingPlaying ? <IconPlayerPause size={24} /> : <IconPlayerPlay size={24} />) : (isPaused ? <IconPlayerPlay size={24} /> : <IconPlayerPause size={24} />)}
                        </Button>
                        <Button isIconOnly variant="light" className="text-white" onPress={handleStop} aria-label="Stop">
                          <IconPlayerStop size={20} />
                        </Button>
                        <VolumeControl
                          volume={isCastingThisMedia ? castSession?.volume || 0 : volume}
                          isMuted={isCastingThisMedia ? castSession?.isMuted || false : isMuted}
                          onVolumeChange={handleVolumeChange}
                          onMuteToggle={toggleMute}
                          size="sm"
                          iconSize={20}
                          className="text-white"
                        />
                      </div>

                      {/* Time display */}
                      <span className="text-sm font-mono ml-2">
                        {formatTime(displayCurrentTime)} / {formatTime(displayDuration)}
                      </span>
                    </div>
                  </div>
                </div>
              </>
            )}
          </div>
        </div>

        {/* Mini player: controls bar below video */}
        {!isExpanded && (
          <div className="flex items-center justify-between px-2 py-1.5 bg-default-100 rounded-b-lg">
            <div className="flex items-center">
              <Button isIconOnly size="sm" variant="light" onPress={togglePlay} aria-label={isCastingThisMedia ? (isCastingPlaying ? "Pause" : "Play") : (isPaused ? "Play" : "Pause")}>
                {isCastingThisMedia ? (isCastingPlaying ? <IconPlayerPause size={18} /> : <IconPlayerPlay size={18} />) : (isPaused ? <IconPlayerPlay size={18} /> : <IconPlayerPause size={18} />)}
              </Button>
              <VolumeControl
                volume={isCastingThisMedia ? castSession?.volume || 0 : volume}
                isMuted={isCastingThisMedia ? castSession?.isMuted || false : isMuted}
                onVolumeChange={handleVolumeChange}
                onMuteToggle={toggleMute}
                size="sm"
                iconSize={18}
              />
              <Button isIconOnly size="sm" variant="light" onPress={handleStop} aria-label="Stop">
                <IconPlayerStop size={18} />
              </Button>
            </div>
            <div className="flex items-center gap-2 min-w-0">
              <span className="text-xs text-default-400 truncate">{(currentShow as Show | null)?.Name || 'Playing'}</span>
              <span className="text-xs text-default-500 font-mono flex-shrink-0">
                -{formatTime(Math.max(0, duration - currentTime))}
              </span>
            </div>
          </div>
        )}
      </div>
    </>
  );
}
