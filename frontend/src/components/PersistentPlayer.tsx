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
import { graphqlClient, TV_SHOW_QUERY, EPISODES_QUERY, PLAYBACK_SETTINGS_QUERY, type TvShow, type Episode, type PlaybackSettings } from '../lib/graphql';

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

  // Fetch playback settings on mount
  useEffect(() => {
    graphqlClient
      .query<{ playbackSettings: PlaybackSettings }>(PLAYBACK_SETTINGS_QUERY, {})
      .toPromise()
      .then((result) => {
        if (result.data?.playbackSettings) {
          // Convert seconds to milliseconds
          setSyncInterval(result.data.playbackSettings.syncIntervalSeconds * 1000);
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
        graphqlClient.query<{ tvShow: TvShow | null }>(TV_SHOW_QUERY, { id: session.tvShowId }).toPromise(),
        graphqlClient.query<{ episodes: Episode[] }>(EPISODES_QUERY, { tvShowId: session.tvShowId }).toPromise(),
      ]).then(([showRes, epRes]) => {
        if (showRes.data?.tvShow) setCurrentShow(showRes.data.tvShow);
        const ep = epRes.data?.episodes?.find(e => e.id === session.episodeId);
        if (ep) setCurrentEpisode(ep);
      });
    }
  }, [session?.tvShowId, session?.episodeId, currentShow, setCurrentShow, setCurrentEpisode]);

  // Resume from session position
  useEffect(() => {
    if (session && videoRef.current && session.currentPosition > 0) {
      videoRef.current.currentTime = session.currentPosition;
      setCurrentTime(session.currentPosition);
      if (session.duration) setDuration(session.duration);
      setIsMuted(session.isMuted);
    }
  }, [session?.id]);

  // Auto-play when video is ready and session indicates playing
  useEffect(() => {
    if (session?.isPlaying && videoReady && videoRef.current && videoRef.current.paused) {
      videoRef.current.play().catch(() => {});
    }
  }, [session?.isPlaying, videoReady]);

  // Sync position periodically (using configurable interval from settings)
  useEffect(() => {
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
  }, [session, isPaused, updatePlayback, syncInterval]);

  // Cleanup video stream when mediaFileId changes or component unmounts
  // This is critical to abort the HTTP connection and stop network traffic
  useEffect(() => {
    const video = videoRef.current;
    // Cleanup runs when mediaFileId changes or on unmount
    return () => {
      if (video) {
        video.pause();
        video.src = '';
        video.load(); // Abort any in-flight network requests
      }
    };
  }, [session?.mediaFileId]);

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
    setIsPaused(false);
    updatePlayback({ isPlaying: true });
  }, [updatePlayback]);

  const handlePause = useCallback(() => {
    setIsPaused(true);
    if (videoRef.current) {
      updatePlayback({ isPlaying: false, currentPosition: videoRef.current.currentTime });
    }
  }, [updatePlayback]);

  const togglePlay = useCallback(() => {
    if (!videoRef.current) return;
    if (videoRef.current.paused) {
      videoRef.current.play().catch(() => {});
    } else {
      videoRef.current.pause();
    }
  }, []);

  const handleStop = useCallback(async () => {
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
  }, [stopPlayback]);

  const toggleMute = useCallback(() => {
    if (!videoRef.current) return;
    videoRef.current.muted = !videoRef.current.muted;
    setIsMuted(videoRef.current.muted);
    updatePlayback({ isMuted: videoRef.current.muted });
  }, [updatePlayback]);

  const handleVolumeChange = useCallback((newVolume: number) => {
    if (!videoRef.current) return;
    videoRef.current.volume = newVolume;
    setVolume(newVolume);
    // If adjusting volume, unmute
    if (newVolume > 0 && isMuted) {
      videoRef.current.muted = false;
      setIsMuted(false);
      updatePlayback({ isMuted: false });
    }
  }, [isMuted, updatePlayback]);

  const handleSeek = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (!videoRef.current || duration <= 0) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const percent = (e.clientX - rect.left) / rect.width;
    const newTime = percent * duration;
    videoRef.current.currentTime = newTime;
    setCurrentTime(newTime);
    updatePlayback({ currentPosition: newTime });
  }, [duration, updatePlayback]);

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

  // Keyboard controls when expanded
  useEffect(() => {
    if (!isExpanded) return;

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
  }, [isExpanded, togglePlay, toggleMute, handleFullscreen, duration]);

  if (isLoading || !session?.mediaFileId) return null;

  const progress = duration > 0 ? (currentTime / duration) * 100 : 0;
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

            {/* Expanded player: controls overlay */}
            {isExpanded && (
              <>
                {/* Top bar */}
                <div className={`absolute inset-x-0 top-0 transition-opacity duration-300 ${showControls ? 'opacity-100' : 'opacity-0 pointer-events-none'}`}>
                  <div className="bg-gradient-to-b from-black/70 to-transparent p-4 flex items-center justify-between">
                    <div className="flex-1 min-w-0">
                      <h2 className="text-white text-lg font-semibold truncate">{currentShow?.name || 'Show'}</h2>
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
                      <Button isIconOnly variant="light" className="text-white" onPress={handleFullscreen}>
                        <IconMaximize size={20} />
                      </Button>
                      <Button isIconOnly variant="light" className="text-white" onPress={() => setIsExpanded(false)}>
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
                        <Button isIconOnly variant="light" className="text-white" onPress={togglePlay}>
                          {isPaused ? <IconPlayerPlay size={24} /> : <IconPlayerPause size={24} />}
                        </Button>
                        <Button isIconOnly variant="light" className="text-white" onPress={handleStop}>
                          <IconPlayerStop size={20} />
                        </Button>
                        <VolumeControl
                          volume={volume}
                          isMuted={isMuted}
                          onVolumeChange={handleVolumeChange}
                          onMuteToggle={toggleMute}
                          size="sm"
                          iconSize={20}
                          className="text-white"
                        />
                      </div>

                      {/* Time display */}
                      <span className="text-sm font-mono ml-2">
                        {formatTime(currentTime)} / {formatTime(duration)}
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
              <Button isIconOnly size="sm" variant="light" onPress={togglePlay}>
                {isPaused ? <IconPlayerPlay size={18} /> : <IconPlayerPause size={18} />}
              </Button>
              <VolumeControl
                volume={volume}
                isMuted={isMuted}
                onVolumeChange={handleVolumeChange}
                onMuteToggle={toggleMute}
                size="sm"
                iconSize={18}
              />
              <Button isIconOnly size="sm" variant="light" onPress={handleStop}>
                <IconPlayerStop size={18} />
              </Button>
            </div>
            <div className="flex items-center gap-2 min-w-0">
              <span className="text-xs text-default-400 truncate">{currentShow?.name || 'Playing'}</span>
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
