/**
 * Playback Context
 * 
 * Provides shared playback state across the application.
 * Manages persistent video playback with database sync.
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
  type Episode,
  type TvShow,
} from '../lib/graphql';

interface PlaybackContextValue {
  session: PlaybackSession | null;
  isLoading: boolean;
  currentEpisode: Episode | null;
  currentShow: TvShow | null;
  /** When true, the player should expand to dialog view (set when startPlayback is called) */
  shouldExpand: boolean;
  startPlayback: (input: StartPlaybackInput, episode?: Episode, show?: TvShow) => Promise<boolean>;
  updatePlayback: (input: UpdatePlaybackInput) => Promise<boolean>;
  stopPlayback: () => Promise<boolean>;
  refreshSession: () => Promise<void>;
  setCurrentEpisode: (episode: Episode | null) => void;
  setCurrentShow: (show: TvShow | null) => void;
  /** Call this after expanding to clear the expand flag */
  clearExpandFlag: () => void;
}

const PlaybackContext = createContext<PlaybackContextValue | null>(null);

export function PlaybackProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<PlaybackSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [currentEpisode, setCurrentEpisode] = useState<Episode | null>(null);
  const [currentShow, setCurrentShow] = useState<TvShow | null>(null);
  const [shouldExpand, setShouldExpand] = useState(false);
  
  const lastSyncedPosition = useRef<number>(0);

  const clearExpandFlag = useCallback(() => {
    setShouldExpand(false);
  }, []);

  // Fetch current session on mount
  const refreshSession = useCallback(async () => {
    try {
      const result = await graphqlClient
        .query<{ playbackSession: PlaybackSession | null }>(PLAYBACK_SESSION_QUERY, {})
        .toPromise();
      
      if (result.data?.playbackSession) {
        setSession(result.data.playbackSession);
      } else {
        setSession(null);
        setCurrentEpisode(null);
        setCurrentShow(null);
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

  // Start playback
  const startPlayback = useCallback(async (
    input: StartPlaybackInput,
    episode?: Episode,
    show?: TvShow
  ): Promise<boolean> => {
    try {
      const result = await graphqlClient
        .mutation<{ startPlayback: PlaybackResult }>(START_PLAYBACK_MUTATION, { input })
        .toPromise();
      
      if (result.data?.startPlayback.success && result.data.startPlayback.session) {
        setSession(result.data.startPlayback.session);
        if (episode) setCurrentEpisode(episode);
        if (show) setCurrentShow(show);
        lastSyncedPosition.current = input.startPosition || 0;
        // Signal the player to expand to dialog view
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

  // Update playback
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

  // Stop playback
  const stopPlayback = useCallback(async (): Promise<boolean> => {
    try {
      const result = await graphqlClient
        .mutation<{ stopPlayback: PlaybackResult }>(STOP_PLAYBACK_MUTATION, {})
        .toPromise();
      
      if (result.data?.stopPlayback.success) {
        setSession(null);
        setCurrentEpisode(null);
        setCurrentShow(null);
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
        currentEpisode,
        currentShow,
        shouldExpand,
        startPlayback,
        updatePlayback,
        stopPlayback,
        refreshSession,
        setCurrentEpisode,
        setCurrentShow,
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
