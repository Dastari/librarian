/**
 * Persistent Audio Player
 * 
 * Spotify-style bottom bar for music tracks and audiobooks.
 * Shows album art, track info, playback controls, and progress.
 */

import { useState, useRef, useEffect, useCallback } from 'react';
import { Button } from '@heroui/button';
import { Image } from '@heroui/image';
import { Slider } from '@heroui/slider';
import { Tooltip } from '@heroui/tooltip';
import {
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerTrackPrev,
  IconPlayerTrackNext,
  IconRepeat,
  IconRepeatOnce,
  IconArrowsShuffle,
  IconX,
  IconMusic,
} from '@tabler/icons-react';
import { usePlaybackContext, type RepeatMode } from '../contexts/PlaybackContext';
import { VolumeControl } from './VolumeControl';
import { getMediaStreamUrl } from './VideoPlayer';
import { graphqlClient, PlaybackSyncIntervalDocument } from '../lib/graphql';

// Default sync interval (will be overridden by settings)
const DEFAULT_SYNC_INTERVAL = 15000;

function formatTime(seconds: number): string {
  if (!isFinite(seconds) || isNaN(seconds)) return '0:00';
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

export function PersistentAudioPlayer() {
  const {
    session,
    isLoading,
    currentContent,
    currentTrack,
    currentAlbum,
    currentAudiobook,
    currentChapter,
    queue,
    queueIndex,
    shuffleEnabled,
    repeatMode,
    updatePlayback,
    stopPlayback,
    playNext,
    playPrevious,
    toggleShuffle,
    setRepeatMode,
  } = usePlaybackContext();

  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(1);
  const [isMuted, setIsMuted] = useState(false);
  const [isPaused, setIsPaused] = useState(true);
  const [isReady, setIsReady] = useState(false);
  const [syncInterval, setSyncInterval] = useState(DEFAULT_SYNC_INTERVAL);

  const audioRef = useRef<HTMLAudioElement>(null);
  const syncIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const shouldAutoPlayRef = useRef<boolean>(false);

  // Determine if this is an audio session
  const isAudioSession = session?.contentType === 'TRACK' || session?.contentType === 'AUDIOBOOK';

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

  // Resume from session position when session changes
  useEffect(() => {
    if (session && audioRef.current && session.currentPosition > 0 && isAudioSession) {
      audioRef.current.currentTime = session.currentPosition;
      setCurrentTime(session.currentPosition);
      if (session.duration) setDuration(session.duration);
      setIsMuted(session.isMuted);
    }
  }, [session?.id, isAudioSession]);

  // Sync audio element play/pause state with session.isPlaying
  // This handles external play/pause commands (from datatable, artwork overlay, etc.)
  useEffect(() => {
    if (!isAudioSession || !isReady || !audioRef.current) return;
    
    if (session?.isPlaying && audioRef.current.paused) {
      // Session says play, but audio is paused - start playing
      audioRef.current.play().catch(() => {});
    } else if (!session?.isPlaying && !audioRef.current.paused) {
      // Session says paused, but audio is playing - pause it
      audioRef.current.pause();
    }
  }, [session?.isPlaying, isReady, isAudioSession]);

  // Sync position periodically
  useEffect(() => {
    if (session && !isPaused && isAudioSession) {
      syncIntervalRef.current = setInterval(() => {
        if (audioRef.current) {
          updatePlayback({
            currentPosition: audioRef.current.currentTime,
            duration: audioRef.current.duration || undefined,
            isPlaying: !audioRef.current.paused,
          });
        }
      }, syncInterval);
    }
    return () => {
      if (syncIntervalRef.current) clearInterval(syncIntervalRef.current);
    };
  }, [session, isPaused, updatePlayback, syncInterval, isAudioSession]);

  // Reset ready state when track changes to prevent race conditions with sync effect
  // Also set auto-play flag if we were actively playing (not just if session says playing)
  const prevMediaFileIdRef = useRef<string | null>(null);
  useEffect(() => {
    const currentMediaFileId = session?.mediaFileId ?? null;
    const previousMediaFileId = prevMediaFileIdRef.current;
    
    if (previousMediaFileId && currentMediaFileId && previousMediaFileId !== currentMediaFileId) {
      // Track changed - reset ready state so sync effect waits for new track
      setIsReady(false);
      // If we were playing (audio was not paused), continue playing the new track
      if (audioRef.current && !audioRef.current.paused) {
        shouldAutoPlayRef.current = true;
      }
    }
    
    prevMediaFileIdRef.current = currentMediaFileId;
  }, [session?.mediaFileId]);

  // Cleanup audio when component unmounts
  useEffect(() => {
    const audio = audioRef.current;
    return () => {
      if (audio) {
        audio.pause();
        audio.src = '';
        audio.load();
      }
    };
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    if (!isAudioSession) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't handle if typing in an input
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

      switch (e.key) {
        case ' ':
        case 'k':
          e.preventDefault();
          if (audioRef.current) {
            if (audioRef.current.paused) {
              audioRef.current.play().catch(() => {});
            } else {
              audioRef.current.pause();
            }
          }
          break;
        case 'n':
        case 'N':
          e.preventDefault();
          playNext();
          break;
        case 'p':
        case 'P':
          e.preventDefault();
          playPrevious();
          break;
        case 'm':
          e.preventDefault();
          if (audioRef.current) {
            audioRef.current.muted = !audioRef.current.muted;
            setIsMuted(audioRef.current.muted);
            updatePlayback({ isMuted: audioRef.current.muted });
          }
          break;
        case 'ArrowLeft':
          e.preventDefault();
          if (audioRef.current) {
            audioRef.current.currentTime = Math.max(0, audioRef.current.currentTime - 10);
          }
          break;
        case 'ArrowRight':
          e.preventDefault();
          if (audioRef.current) {
            audioRef.current.currentTime = Math.min(duration, audioRef.current.currentTime + 10);
          }
          break;
        case 'ArrowUp':
          e.preventDefault();
          if (audioRef.current) {
            const newVol = Math.min(1, audioRef.current.volume + 0.1);
            audioRef.current.volume = newVol;
            setVolume(newVol);
          }
          break;
        case 'ArrowDown':
          e.preventDefault();
          if (audioRef.current) {
            const newVol = Math.max(0, audioRef.current.volume - 0.1);
            audioRef.current.volume = newVol;
            setVolume(newVol);
          }
          break;
        case 's':
        case 'S':
          e.preventDefault();
          toggleShuffle();
          break;
        case 'r':
        case 'R':
          e.preventDefault();
          const modes: RepeatMode[] = ['off', 'all', 'one'];
          const currentIdx = modes.indexOf(repeatMode);
          const nextIdx = (currentIdx + 1) % modes.length;
          setRepeatMode(modes[nextIdx]);
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isAudioSession, duration, playNext, playPrevious, toggleShuffle, repeatMode, setRepeatMode, updatePlayback]);

  const handleTimeUpdate = useCallback(() => {
    if (audioRef.current) {
      setCurrentTime(audioRef.current.currentTime);
    }
  }, []);

  const handleLoadedMetadata = useCallback(() => {
    if (audioRef.current) {
      setDuration(audioRef.current.duration);
      if (session?.currentPosition) {
        audioRef.current.currentTime = session.currentPosition;
      }
    }
  }, [session?.currentPosition]);

  const handleCanPlay = useCallback(() => {
    setIsReady(true);
    setIsPaused(audioRef.current?.paused ?? true);
    
    // Auto-play if we're transitioning tracks (e.g., after previous track ended)
    if (shouldAutoPlayRef.current && audioRef.current) {
      shouldAutoPlayRef.current = false;
      audioRef.current.play().catch(() => {});
    }
  }, []);

  const handlePlay = useCallback(() => {
    setIsPaused(false);
    updatePlayback({ isPlaying: true });
  }, [updatePlayback]);

  const handlePause = useCallback(() => {
    setIsPaused(true);
    if (audioRef.current) {
      updatePlayback({ isPlaying: false, currentPosition: audioRef.current.currentTime });
    }
  }, [updatePlayback]);

  const handleEnded = useCallback(async () => {
    // Handle repeat one
    if (repeatMode === 'one' && audioRef.current) {
      audioRef.current.currentTime = 0;
      audioRef.current.play().catch(() => {});
      return;
    }

    // Set flag to auto-play when the next track is ready
    shouldAutoPlayRef.current = true;

    // Try to play next
    const success = await playNext();
    if (!success && repeatMode !== 'all') {
      // End of queue, stop playback
      shouldAutoPlayRef.current = false;
      await stopPlayback();
    }
  }, [repeatMode, playNext, stopPlayback]);

  const togglePlay = useCallback(() => {
    if (!audioRef.current) return;
    if (audioRef.current.paused) {
      audioRef.current.play().catch(() => {});
    } else {
      audioRef.current.pause();
    }
  }, []);

  const handleStop = useCallback(async () => {
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.src = '';
      audioRef.current.load();
    }
    setIsReady(false);
    setIsPaused(true);
    setCurrentTime(0);
    setDuration(0);
    await stopPlayback();
  }, [stopPlayback]);

  const toggleMute = useCallback(() => {
    if (!audioRef.current) return;
    audioRef.current.muted = !audioRef.current.muted;
    setIsMuted(audioRef.current.muted);
    updatePlayback({ isMuted: audioRef.current.muted });
  }, [updatePlayback]);

  const handleVolumeChange = useCallback((newVolume: number) => {
    if (!audioRef.current) return;
    audioRef.current.volume = newVolume;
    setVolume(newVolume);
    if (newVolume > 0 && isMuted) {
      audioRef.current.muted = false;
      setIsMuted(false);
      updatePlayback({ isMuted: false });
    }
  }, [isMuted, updatePlayback]);

  const handleSeek = useCallback((value: number | number[]) => {
    if (!audioRef.current || duration <= 0) return;
    const newTime = Array.isArray(value) ? value[0] : value;
    audioRef.current.currentTime = newTime;
    setCurrentTime(newTime);
    updatePlayback({ currentPosition: newTime });
  }, [duration, updatePlayback]);

  const cycleRepeatMode = useCallback(() => {
    const modes: RepeatMode[] = ['off', 'all', 'one'];
    const currentIdx = modes.indexOf(repeatMode);
    const nextIdx = (currentIdx + 1) % modes.length;
    setRepeatMode(modes[nextIdx]);
  }, [repeatMode, setRepeatMode]);

  // Don't render if no audio session or still loading
  if (isLoading || !session?.mediaFileId || !isAudioSession) {
    return null;
  }

  const progress = duration > 0 ? (currentTime / duration) * 100 : 0;

  // Get display info
  const title = currentContent?.title || currentTrack?.title || currentChapter?.title || 'Unknown';
  const subtitle = currentContent?.subtitle || 
    (currentTrack?.artistName ? `${currentTrack.artistName} - ${currentAlbum?.name || ''}` : null) ||
    (currentAudiobook?.title ? `${currentAudiobook.title}` : null) ||
    '';
  const coverUrl = currentContent?.posterUrl || currentAlbum?.coverUrl || currentAudiobook?.coverUrl;
  const hasQueue = queue.length > 1;
  const isFirst = queueIndex === 0;
  const isLast = queueIndex >= queue.length - 1;

  return (
    <div className="fixed bottom-0 inset-x-0 z-40 bg-content1 border-t border-default-200 shadow-2xs">
      {/* Progress bar - thin line at top */}
      <div className="absolute top-0 inset-x-0 h-1 bg-default-200">
        <div 
          className="h-full bg-primary transition-all duration-150"
          style={{ width: `${progress}%` }}
        />
      </div>

      {/* Main content */}
      <div className="h-20 px-4 flex items-center gap-4">
        {/* Left section: Now Playing */}
        <div className="flex items-center gap-3 w-72 min-w-0">
          {/* Album art */}
          <div className="w-14 h-14 shrink-0 rounded overflow-hidden bg-default-200">
            {coverUrl ? (
              <Image
                src={coverUrl}
                alt={title}
                classNames={{
                  wrapper: 'w-full h-full',
                  img: 'w-full h-full object-cover',
                }}
              />
            ) : (
              <div className="w-full h-full flex items-center justify-center">
                <IconMusic size={24} className="text-default-400" />
              </div>
            )}
          </div>

          {/* Track info */}
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium truncate">{title}</p>
            <p className="text-xs text-default-500 truncate">{subtitle}</p>
          </div>
        </div>

        {/* Center section: Playback controls */}
        <div className="flex-1 flex flex-col items-center gap-1">
          {/* Control buttons */}
          <div className="flex items-center gap-1">
            {/* Shuffle */}
            <Tooltip content={shuffleEnabled ? 'Shuffle on' : 'Shuffle off'}>
              <Button
                isIconOnly
                size="sm"
                variant="light"
                className={shuffleEnabled ? 'text-primary' : 'text-default-500'}
                onPress={toggleShuffle}
                isDisabled={!hasQueue}
                aria-label={shuffleEnabled ? 'Shuffle on' : 'Shuffle off'}
              >
                <IconArrowsShuffle size={18} />
              </Button>
            </Tooltip>

            {/* Previous */}
            <Tooltip content="Previous">
              <Button
                isIconOnly
                size="sm"
                variant="light"
                onPress={() => playPrevious()}
                isDisabled={!hasQueue && isFirst}
                aria-label="Previous track"
              >
                <IconPlayerTrackPrev size={20} />
              </Button>
            </Tooltip>

            {/* Play/Pause */}
            <Button
              isIconOnly
              size="md"
              color="primary"
              variant="solid"
              className="rounded-full"
              onPress={togglePlay}
              aria-label={isPaused ? 'Play' : 'Pause'}
            >
              {isPaused ? <IconPlayerPlay size={24} /> : <IconPlayerPause size={24} />}
            </Button>

            {/* Next */}
            <Tooltip content="Next">
              <Button
                isIconOnly
                size="sm"
                variant="light"
                onPress={() => playNext()}
                isDisabled={!hasQueue && isLast && repeatMode !== 'all'}
                aria-label="Next track"
              >
                <IconPlayerTrackNext size={20} />
              </Button>
            </Tooltip>

            {/* Repeat */}
            <Tooltip content={
              repeatMode === 'off' ? 'Repeat off' :
              repeatMode === 'all' ? 'Repeat all' : 'Repeat one'
            }>
              <Button
                isIconOnly
                size="sm"
                variant="light"
                className={repeatMode !== 'off' ? 'text-primary' : 'text-default-500'}
                onPress={cycleRepeatMode}
                aria-label={repeatMode === 'off' ? 'Repeat off' : repeatMode === 'all' ? 'Repeat all' : 'Repeat one'}
              >
                {repeatMode === 'one' ? <IconRepeatOnce size={18} /> : <IconRepeat size={18} />}
              </Button>
            </Tooltip>
          </div>

          {/* Seek slider */}
          <div className="w-full max-w-md flex items-center gap-2">
            <span className="text-xs text-default-500 w-10 text-right font-mono">
              {formatTime(currentTime)}
            </span>
            <Slider
              size="sm"
              step={1}
              minValue={0}
              maxValue={Math.max(duration, 1)}
              value={currentTime}
              onChange={handleSeek}
              className="flex-1"
              aria-label="Seek"
            />
            <span className="text-xs text-default-500 w-10 font-mono">
              {formatTime(duration)}
            </span>
          </div>
        </div>

        {/* Right section: Volume and close */}
        <div className="flex items-center gap-2 w-40 justify-end">
          {/* Queue indicator */}
          {hasQueue && (
            <span className="text-xs text-default-500">
              {queueIndex + 1}/{queue.length}
            </span>
          )}

          {/* Volume */}
          <VolumeControl
            volume={volume}
            isMuted={isMuted}
            onVolumeChange={handleVolumeChange}
            onMuteToggle={toggleMute}
            size="sm"
            iconSize={18}
          />

          {/* Close */}
          <Tooltip content="Stop playback">
            <Button
              isIconOnly
              size="sm"
              variant="light"
              className="text-default-500 hover:text-danger"
              onPress={handleStop}
              aria-label="Stop playback"
            >
              <IconX size={18} />
            </Button>
          </Tooltip>
        </div>
      </div>

      {/* Hidden audio element */}
      <audio
        ref={audioRef}
        src={getMediaStreamUrl(session.mediaFileId)}
        onTimeUpdate={handleTimeUpdate}
        onLoadedMetadata={handleLoadedMetadata}
        onCanPlay={handleCanPlay}
        onPlay={handlePlay}
        onPause={handlePause}
        onEnded={handleEnded}
        onError={() => setIsReady(false)}
        muted={isMuted}
        preload="metadata"
      />
    </div>
  );
}
