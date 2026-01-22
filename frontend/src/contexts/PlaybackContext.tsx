/**
 * Playback Context
 * 
 * Provides shared playback state across the application.
 * Manages persistent video/audio playback with database sync.
 * Supports all content types: episodes, movies, tracks, and audiobooks.
 */

import { createContext, useContext, useState, useEffect, useCallback, useRef, type ReactNode } from 'react';
import { useRouteContext } from '@tanstack/react-router';
import {
  graphqlClient,
  PLAYBACK_SESSION_QUERY,
  START_PLAYBACK_MUTATION,
  UPDATE_PLAYBACK_MUTATION,
  STOP_PLAYBACK_MUTATION,
  ALBUM_WITH_TRACKS_QUERY,
  AUDIOBOOK_WITH_CHAPTERS_QUERY,
  type PlaybackSession,
  type PlaybackResult,
  type StartPlaybackInput,
  type UpdatePlaybackInput,
  type PlaybackContentType,
  type Episode,
  type TvShow,
  type Movie,
  type Track,
  type Album,
  type Audiobook,
  type AudiobookChapter,
  type AlbumWithTracks,
  type AudiobookWithChapters,
} from '../lib/graphql';

/** Metadata for the currently playing content */
export interface CurrentContentMetadata {
  contentType: PlaybackContentType;
  title: string;
  subtitle?: string;
  posterUrl?: string | null;
  backdropUrl?: string | null;
}

/** Queue item for audio playback */
export interface QueueItem {
  id: string;
  mediaFileId: string;
  title: string;
  artist?: string;
  duration?: number;
  coverUrl?: string | null;
  // For tracks
  track?: Track;
  // For audiobook chapters
  chapter?: AudiobookChapter;
}

/** Repeat mode for audio playback */
export type RepeatMode = 'off' | 'all' | 'one';

interface PlaybackContextValue {
  session: PlaybackSession | null;
  isLoading: boolean;
  currentContent: CurrentContentMetadata | null;
  currentEpisode: Episode | null;
  currentShow: TvShow | null;
  currentMovie: Movie | null;
  shouldExpand: boolean;
  
  // Audio-specific state
  currentTrack: Track | null;
  currentAlbum: Album | null;
  currentAudiobook: Audiobook | null;
  currentChapter: AudiobookChapter | null;
  queue: QueueItem[];
  queueIndex: number;
  shuffleEnabled: boolean;
  repeatMode: RepeatMode;
  
  // Base playback methods
  startPlayback: (input: StartPlaybackInput, metadata?: CurrentContentMetadata) => Promise<boolean>;
  startEpisodePlayback: (
    episodeId: string,
    mediaFileId: string,
    tvShowId: string,
    episode?: Episode,
    show?: TvShow,
    startPosition?: number,
    duration?: number
  ) => Promise<boolean>;
  startMoviePlayback: (
    movieId: string,
    mediaFileId: string,
    movie?: Movie,
    startPosition?: number,
    duration?: number
  ) => Promise<boolean>;
  
  // Audio-specific playback methods
  startTrackPlayback: (
    track: Track,
    album: Album,
    allTracks: Track[],
    startPosition?: number
  ) => Promise<boolean>;
  startAudiobookPlayback: (
    audiobook: Audiobook,
    chapter: AudiobookChapter,
    allChapters: AudiobookChapter[],
    startPosition?: number
  ) => Promise<boolean>;
  playNext: () => Promise<boolean>;
  playPrevious: () => Promise<boolean>;
  playQueueItem: (index: number) => Promise<boolean>;
  toggleShuffle: () => void;
  setRepeatMode: (mode: RepeatMode) => void;
  
  // Common methods
  updatePlayback: (input: UpdatePlaybackInput) => Promise<boolean>;
  stopPlayback: () => Promise<boolean>;
  refreshSession: () => Promise<void>;
  setCurrentEpisode: (episode: Episode | null) => void;
  setCurrentShow: (show: TvShow | null) => void;
  setCurrentMovie: (movie: Movie | null) => void;
  clearExpandFlag: () => void;
}

const PlaybackContext = createContext<PlaybackContextValue | null>(null);

export function PlaybackProvider({ children }: { children: ReactNode }) {
  // Get auth context from router - only fetch playback session if authenticated
  const { auth } = useRouteContext({ from: '__root__' });
  
  const [session, setSession] = useState<PlaybackSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [currentContent, setCurrentContent] = useState<CurrentContentMetadata | null>(null);
  const [currentEpisode, setCurrentEpisode] = useState<Episode | null>(null);
  const [currentShow, setCurrentShow] = useState<TvShow | null>(null);
  const [currentMovie, setCurrentMovie] = useState<Movie | null>(null);
  const [shouldExpand, setShouldExpand] = useState(false);
  
  // Audio-specific state
  const [currentTrack, setCurrentTrack] = useState<Track | null>(null);
  const [currentAlbum, setCurrentAlbum] = useState<Album | null>(null);
  const [currentAudiobook, setCurrentAudiobook] = useState<Audiobook | null>(null);
  const [currentChapter, setCurrentChapter] = useState<AudiobookChapter | null>(null);
  const [queue, setQueue] = useState<QueueItem[]>([]);
  const [queueIndex, setQueueIndex] = useState(0);
  const [shuffleEnabled, setShuffleEnabled] = useState(false);
  const [repeatMode, setRepeatModeState] = useState<RepeatMode>('off');
  
  const lastSyncedPosition = useRef<number>(0);

  const clearExpandFlag = useCallback(() => {
    setShouldExpand(false);
  }, []);

  const clearAllState = useCallback(() => {
    setSession(null);
    setCurrentContent(null);
    setCurrentEpisode(null);
    setCurrentShow(null);
    setCurrentMovie(null);
    setCurrentTrack(null);
    setCurrentAlbum(null);
    setCurrentAudiobook(null);
    setCurrentChapter(null);
    setQueue([]);
    setQueueIndex(0);
  }, []);

  const refreshSession = useCallback(async () => {
    try {
      const result = await graphqlClient
        .query<{ playbackSession: PlaybackSession | null }>(PLAYBACK_SESSION_QUERY, {})
        .toPromise();
      
      if (result.data?.playbackSession) {
        const session = result.data.playbackSession;
        setSession(session);
        
        // For audio sessions, fetch the track/album or audiobook data
        if (session.contentType === 'TRACK' && session.albumId && session.trackId) {
          try {
            const albumResult = await graphqlClient
              .query<{ albumWithTracks: AlbumWithTracks | null }>(ALBUM_WITH_TRACKS_QUERY, { id: session.albumId })
              .toPromise();
            
            if (albumResult.data?.albumWithTracks) {
              const { album, tracks } = albumResult.data.albumWithTracks;
              setCurrentAlbum(album);
              
              // Find the current track
              const trackData = tracks.find(t => t.track.id === session.trackId);
              if (trackData) {
                setCurrentTrack(trackData.track);
                
                // Build queue from all available tracks
                const allTracks = tracks.filter(t => t.track.mediaFileId).map(t => t.track);
                const newQueue = allTracks.map(track => ({
                  id: track.id,
                  mediaFileId: track.mediaFileId!,
                  title: track.title,
                  artist: track.artistName || undefined,
                  duration: track.durationSecs || undefined,
                  coverUrl: album.coverUrl,
                  track,
                }));
                setQueue(newQueue);
                
                const idx = newQueue.findIndex(q => q.id === session.trackId);
                setQueueIndex(idx >= 0 ? idx : 0);
                
                // Set content metadata for display
                setCurrentContent({
                  contentType: 'TRACK',
                  title: trackData.track.title,
                  subtitle: trackData.track.artistName || album.name,
                  posterUrl: album.coverUrl,
                });
              }
            }
          } catch (err) {
            console.error('Failed to fetch track/album data:', err);
          }
        } else if (session.contentType === 'AUDIOBOOK' && session.audiobookId) {
          try {
            const audiobookResult = await graphqlClient
              .query<{ audiobookWithChapters: AudiobookWithChapters | null }>(AUDIOBOOK_WITH_CHAPTERS_QUERY, { id: session.audiobookId })
              .toPromise();
            
            if (audiobookResult.data?.audiobookWithChapters) {
              const { audiobook, chapters } = audiobookResult.data.audiobookWithChapters;
              setCurrentAudiobook(audiobook);
              
              // Find the current chapter (use mediaFileId to match since we may not have chapterId directly)
              const currentChapter = chapters.find(c => c.mediaFileId === session.mediaFileId);
              if (currentChapter) {
                setCurrentChapter(currentChapter);
                
                // Build queue from all available chapters
                const availableChapters = chapters.filter(c => c.mediaFileId);
                const newQueue = availableChapters.map(chapter => ({
                  id: chapter.id,
                  mediaFileId: chapter.mediaFileId!,
                  title: chapter.title || `Chapter ${chapter.chapterNumber}`,
                  duration: chapter.durationSecs || undefined,
                  coverUrl: audiobook.coverUrl,
                  chapter,
                }));
                setQueue(newQueue);
                
                const idx = newQueue.findIndex(q => q.id === currentChapter.id);
                setQueueIndex(idx >= 0 ? idx : 0);
                
                // Set content metadata for display
                setCurrentContent({
                  contentType: 'AUDIOBOOK',
                  title: audiobook.title,
                  subtitle: currentChapter.title || `Chapter ${currentChapter.chapterNumber}`,
                  posterUrl: audiobook.coverUrl,
                });
              }
            }
          } catch (err) {
            console.error('Failed to fetch audiobook data:', err);
          }
        }
      } else {
        clearAllState();
      }
    } catch (err) {
      console.error('Failed to fetch playback session:', err);
    } finally {
      setIsLoading(false);
    }
  }, [clearAllState]);

  // Only fetch playback session when user is authenticated
  useEffect(() => {
    if (auth.isAuthenticated) {
      refreshSession();
    } else {
      // Not authenticated - clear state and stop loading
      setIsLoading(false);
      setSession(null);
    }
  }, [auth.isAuthenticated, refreshSession]);

  const startPlayback = useCallback(async (
    input: StartPlaybackInput,
    metadata?: CurrentContentMetadata
  ): Promise<boolean> => {
    try {
      const result = await graphqlClient
        .mutation<{ startPlayback: PlaybackResult }>(START_PLAYBACK_MUTATION, { input })
        .toPromise();
      
      if (result.data?.startPlayback.success && result.data.startPlayback.session) {
        setSession(result.data.startPlayback.session);
        if (metadata) setCurrentContent(metadata);
        lastSyncedPosition.current = input.startPosition || 0;
        setShouldExpand(true);
        return true;
      }
      
      console.error('Failed to start playback:', result.data?.startPlayback.error);
      return false;
    } catch (err) {
      console.error('Failed to start playback:', err);
      return false;
    }
  }, []);

  const startEpisodePlayback = useCallback(async (
    episodeId: string,
    mediaFileId: string,
    tvShowId: string,
    episode?: Episode,
    show?: TvShow,
    startPosition?: number,
    duration?: number
  ): Promise<boolean> => {
    const input: StartPlaybackInput = {
      contentType: 'EPISODE',
      contentId: episodeId,
      mediaFileId,
      parentId: tvShowId,
      startPosition,
      duration,
    };

    const metadata: CurrentContentMetadata | undefined = episode && show ? {
      contentType: 'EPISODE',
      title: show.name,
      subtitle: `S${String(episode.season).padStart(2, '0')}E${String(episode.episode).padStart(2, '0')} - ${episode.title}`,
      posterUrl: show.posterUrl,
      backdropUrl: show.backdropUrl,
    } : undefined;

    const success = await startPlayback(input, metadata);
    if (success) {
      if (episode) setCurrentEpisode(episode);
      if (show) setCurrentShow(show);
    }
    return success;
  }, [startPlayback]);

  const startMoviePlayback = useCallback(async (
    movieId: string,
    mediaFileId: string,
    movie?: Movie,
    startPosition?: number,
    duration?: number
  ): Promise<boolean> => {
    const input: StartPlaybackInput = {
      contentType: 'MOVIE',
      contentId: movieId,
      mediaFileId,
      startPosition,
      duration,
    };

    const metadata: CurrentContentMetadata | undefined = movie ? {
      contentType: 'MOVIE',
      title: movie.title,
      subtitle: movie.year ? `${movie.year}` : undefined,
      posterUrl: movie.posterUrl,
      backdropUrl: movie.backdropUrl,
    } : undefined;

    const success = await startPlayback(input, metadata);
    if (success && movie) {
      setCurrentMovie(movie);
    }
    return success;
  }, [startPlayback]);

  const updatePlayback = useCallback(async (input: UpdatePlaybackInput): Promise<boolean> => {
    if (input.currentPosition !== undefined) {
      const diff = Math.abs(input.currentPosition - lastSyncedPosition.current);
      if (diff < 1) {
        return true;
      }
      lastSyncedPosition.current = input.currentPosition;
    }

    // Optimistically update isPlaying state immediately for responsive UI
    if (input.isPlaying !== undefined) {
      setSession(prev => prev ? { ...prev, isPlaying: input.isPlaying! } : prev);
    }

    try {
      const result = await graphqlClient
        .mutation<{ updatePlayback: PlaybackResult }>(UPDATE_PLAYBACK_MUTATION, { input })
        .toPromise();
      
      if (result.data?.updatePlayback.success && result.data.updatePlayback.session) {
        setSession(result.data.updatePlayback.session);
        return true;
      }
      
      return false;
    } catch (err) {
      console.error('Failed to update playback:', err);
      return false;
    }
  }, []);

  const stopPlayback = useCallback(async (): Promise<boolean> => {
    try {
      const result = await graphqlClient
        .mutation<{ stopPlayback: PlaybackResult }>(STOP_PLAYBACK_MUTATION, {})
        .toPromise();
      
      if (result.data?.stopPlayback.success) {
        clearAllState();
        lastSyncedPosition.current = 0;
        return true;
      }
      
      return false;
    } catch (err) {
      console.error('Failed to stop playback:', err);
      return false;
    }
  }, [clearAllState]);

  // Build queue from tracks or chapters
  const buildQueueFromTracks = useCallback((tracks: Track[], album: Album): QueueItem[] => {
    return tracks
      .filter(t => t.mediaFileId) // Only include tracks with media files
      .sort((a, b) => {
        // Sort by disc then track number
        if (a.discNumber !== b.discNumber) return a.discNumber - b.discNumber;
        return a.trackNumber - b.trackNumber;
      })
      .map(track => ({
        id: track.id,
        mediaFileId: track.mediaFileId!,
        title: track.title,
        artist: track.artistName || undefined,
        duration: track.durationSecs || undefined,
        coverUrl: album.coverUrl,
        track,
      }));
  }, []);

  const buildQueueFromChapters = useCallback((chapters: AudiobookChapter[], audiobook: Audiobook): QueueItem[] => {
    return chapters
      .filter(c => c.mediaFileId) // Only include chapters with media files
      .sort((a, b) => a.chapterNumber - b.chapterNumber)
      .map(chapter => ({
        id: chapter.id,
        mediaFileId: chapter.mediaFileId!,
        title: chapter.title || `Chapter ${chapter.chapterNumber}`,
        duration: chapter.durationSecs || undefined,
        coverUrl: audiobook.coverUrl,
        chapter,
      }));
  }, []);

  // Start track playback with queue
  const startTrackPlayback = useCallback(async (
    track: Track,
    album: Album,
    allTracks: Track[],
    startPosition?: number
  ): Promise<boolean> => {
    if (!track.mediaFileId) {
      console.error('Track has no media file');
      return false;
    }

    const input: StartPlaybackInput = {
      contentType: 'TRACK',
      contentId: track.id,
      mediaFileId: track.mediaFileId,
      parentId: album.id,
      startPosition,
      duration: track.durationSecs || undefined,
    };

    const metadata: CurrentContentMetadata = {
      contentType: 'TRACK',
      title: track.title,
      subtitle: track.artistName || album.name,
      posterUrl: album.coverUrl,
    };

    const success = await startPlayback(input, metadata);
    if (success) {
      setCurrentTrack(track);
      setCurrentAlbum(album);
      setCurrentAudiobook(null);
      setCurrentChapter(null);
      
      // Build queue from all tracks
      const newQueue = buildQueueFromTracks(allTracks, album);
      setQueue(newQueue);
      
      // Find current index in queue
      const idx = newQueue.findIndex(q => q.id === track.id);
      setQueueIndex(idx >= 0 ? idx : 0);
    }
    return success;
  }, [startPlayback, buildQueueFromTracks]);

  // Start audiobook playback with queue
  const startAudiobookPlayback = useCallback(async (
    audiobook: Audiobook,
    chapter: AudiobookChapter,
    allChapters: AudiobookChapter[],
    startPosition?: number
  ): Promise<boolean> => {
    if (!chapter.mediaFileId) {
      console.error('Chapter has no media file');
      return false;
    }

    const input: StartPlaybackInput = {
      contentType: 'AUDIOBOOK',
      contentId: audiobook.id,
      mediaFileId: chapter.mediaFileId,
      startPosition,
      duration: chapter.durationSecs || undefined,
    };

    const metadata: CurrentContentMetadata = {
      contentType: 'AUDIOBOOK',
      title: audiobook.title,
      subtitle: chapter.title || `Chapter ${chapter.chapterNumber}`,
      posterUrl: audiobook.coverUrl,
    };

    const success = await startPlayback(input, metadata);
    if (success) {
      setCurrentAudiobook(audiobook);
      setCurrentChapter(chapter);
      setCurrentTrack(null);
      setCurrentAlbum(null);
      
      // Build queue from all chapters
      const newQueue = buildQueueFromChapters(allChapters, audiobook);
      setQueue(newQueue);
      
      // Find current index in queue
      const idx = newQueue.findIndex(q => q.id === chapter.id);
      setQueueIndex(idx >= 0 ? idx : 0);
    }
    return success;
  }, [startPlayback, buildQueueFromChapters]);

  // Play a specific queue item by index
  const playQueueItem = useCallback(async (index: number): Promise<boolean> => {
    if (index < 0 || index >= queue.length) return false;
    
    const item = queue[index];
    
    const input: StartPlaybackInput = {
      contentType: item.track ? 'TRACK' : 'AUDIOBOOK',
      contentId: item.track?.id || currentAudiobook?.id || item.id,
      mediaFileId: item.mediaFileId,
      parentId: item.track ? currentAlbum?.id : undefined,
      duration: item.duration,
    };

    const metadata: CurrentContentMetadata = {
      contentType: item.track ? 'TRACK' : 'AUDIOBOOK',
      title: item.title,
      subtitle: item.artist || currentAudiobook?.title,
      posterUrl: item.coverUrl,
    };

    const success = await startPlayback(input, metadata);
    if (success) {
      setQueueIndex(index);
      if (item.track) {
        setCurrentTrack(item.track);
      } else if (item.chapter) {
        setCurrentChapter(item.chapter);
      }
    }
    return success;
  }, [queue, currentAlbum, currentAudiobook, startPlayback]);

  // Play next track/chapter in queue
  const playNext = useCallback(async (): Promise<boolean> => {
    if (queue.length === 0) return false;
    
    let nextIndex = queueIndex + 1;
    
    // Handle repeat modes
    if (nextIndex >= queue.length) {
      if (repeatMode === 'all') {
        nextIndex = 0;
      } else {
        return false; // End of queue
      }
    }
    
    return playQueueItem(nextIndex);
  }, [queue, queueIndex, repeatMode, playQueueItem]);

  // Play previous track/chapter in queue
  const playPrevious = useCallback(async (): Promise<boolean> => {
    if (queue.length === 0) return false;
    
    let prevIndex = queueIndex - 1;
    
    // Handle repeat modes
    if (prevIndex < 0) {
      if (repeatMode === 'all') {
        prevIndex = queue.length - 1;
      } else {
        prevIndex = 0; // Stay at beginning
      }
    }
    
    return playQueueItem(prevIndex);
  }, [queue, queueIndex, repeatMode, playQueueItem]);

  // Toggle shuffle
  const toggleShuffle = useCallback(() => {
    setShuffleEnabled(prev => !prev);
    // Note: Actually shuffling the queue would be handled in playNext
    // For simplicity, shuffle just randomizes the next pick
  }, []);

  // Set repeat mode
  const setRepeatMode = useCallback((mode: RepeatMode) => {
    setRepeatModeState(mode);
  }, []);

  return (
    <PlaybackContext.Provider
      value={{
        session,
        isLoading,
        currentContent,
        currentEpisode,
        currentShow,
        currentMovie,
        shouldExpand,
        
        // Audio-specific state
        currentTrack,
        currentAlbum,
        currentAudiobook,
        currentChapter,
        queue,
        queueIndex,
        shuffleEnabled,
        repeatMode,
        
        // Base playback methods
        startPlayback,
        startEpisodePlayback,
        startMoviePlayback,
        
        // Audio-specific methods
        startTrackPlayback,
        startAudiobookPlayback,
        playNext,
        playPrevious,
        playQueueItem,
        toggleShuffle,
        setRepeatMode,
        
        // Common methods
        updatePlayback,
        stopPlayback,
        refreshSession,
        setCurrentEpisode,
        setCurrentShow,
        setCurrentMovie,
        clearExpandFlag,
      }}
    >
      {children}
    </PlaybackContext.Provider>
  );
}

export function usePlaybackContext(): PlaybackContextValue {
  const context = useContext(PlaybackContext);
  if (!context) {
    throw new Error('usePlaybackContext must be used within a PlaybackProvider');
  }
  return context;
}
