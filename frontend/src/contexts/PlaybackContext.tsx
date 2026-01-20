/**
 * Playback Context
 * 
 * Provides shared playback state across the application.
 * Manages persistent video/audio playback with database sync.
 * Supports all content types: episodes, movies, tracks, and audiobooks.
 */

import { createContext, useContext, useState, useEffect, useCallback, useRef, type ReactNode } from 'react';
import {
  graphqlClient,
  PLAYBACK_SESSION_QUERY,
  START_PLAYBACK_MUTATION,
  UPDATE_PLAYBACK_MUTATION,
  STOP_PLAYBACK_MUTATION,
  type PlaybackSession,
  type PlaybackResult,
  type StartPlaybackInput,
  type UpdatePlaybackInput,
  type PlaybackContentType,
  type Episode,
  type TvShow,
  type Movie,
} from '../lib/graphql';

/** Metadata for the currently playing content */
export interface CurrentContentMetadata {
  contentType: PlaybackContentType;
  title: string;
  subtitle?: string;
  posterUrl?: string | null;
  backdropUrl?: string | null;
}

interface PlaybackContextValue {
  session: PlaybackSession | null;
  isLoading: boolean;
  currentContent: CurrentContentMetadata | null;
  currentEpisode: Episode | null;
  currentShow: TvShow | null;
  currentMovie: Movie | null;
  shouldExpand: boolean;
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
  const [session, setSession] = useState<PlaybackSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [currentContent, setCurrentContent] = useState<CurrentContentMetadata | null>(null);
  const [currentEpisode, setCurrentEpisode] = useState<Episode | null>(null);
  const [currentShow, setCurrentShow] = useState<TvShow | null>(null);
  const [currentMovie, setCurrentMovie] = useState<Movie | null>(null);
  const [shouldExpand, setShouldExpand] = useState(false);
  
  const lastSyncedPosition = useRef<number>(0);

  const clearExpandFlag = useCallback(() => {
    setShouldExpand(false);
  }, []);

  const refreshSession = useCallback(async () => {
    try {
      const result = await graphqlClient
        .query<{ playbackSession: PlaybackSession | null }>(PLAYBACK_SESSION_QUERY, {})
        .toPromise();
      
      if (result.data?.playbackSession) {
        setSession(result.data.playbackSession);
      } else {
        setSession(null);
        setCurrentContent(null);
        setCurrentEpisode(null);
        setCurrentShow(null);
        setCurrentMovie(null);
      }
    } catch (err) {
      console.error('Failed to fetch playback session:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshSession();
  }, [refreshSession]);

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
        setSession(null);
        setCurrentContent(null);
        setCurrentEpisode(null);
        setCurrentShow(null);
        setCurrentMovie(null);
        lastSyncedPosition.current = 0;
        return true;
      }
      
      return false;
    } catch (err) {
      console.error('Failed to stop playback:', err);
      return false;
    }
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
        startPlayback,
        startEpisodePlayback,
        startMoviePlayback,
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
