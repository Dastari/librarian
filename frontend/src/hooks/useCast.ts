/**
 * Cast hook for managing Chromecast/media casting state.
 * Uses codegen CastDevices, CastSessions, CastSettings queries.
 * Legacy mutation strings kept for discover/castMedia/play/pause (custom backend ops).
 */

import { useState, useEffect, useCallback } from 'react';
import { graphqlClient } from '../lib/graphql';
import {
  CastDevicesDocument,
  CastSessionsDocument,
  CastSettingsDocument,
} from '../lib/graphql/generated/graphql';
import type {
  CastDevicesQuery,
  CastSessionsQuery,
  CastSettingsQuery,
} from '../lib/graphql/generated/graphql';
import {
  DISCOVER_CAST_DEVICES_MUTATION,
  CAST_MEDIA_MUTATION,
  CAST_PLAY_MUTATION,
  CAST_PAUSE_MUTATION,
  CAST_STOP_MUTATION,
  CAST_SEEK_MUTATION,
  CAST_SET_VOLUME_MUTATION,
  CAST_SET_MUTED_MUTATION,
  type CastDevice,
  type CastSession,
  type CastSettings,
  type CastMediaInput,
  type CastSessionResult,
} from '../lib/graphql';

type DeviceNode = CastDevicesQuery['CastDevices']['Edges'][0]['Node'];
type SessionNode = CastSessionsQuery['CastSessions']['Edges'][0]['Node'];
type SettingNode = CastSettingsQuery['CastSettings']['Edges'][0]['Node'];

function deviceNodeToApp(node: DeviceNode): CastDevice {
  return {
    id: node.Id,
    name: node.Name,
    address: node.Address,
    port: node.Port,
    model: node.Model ?? null,
    deviceType: node.DeviceType as CastDevice['deviceType'],
    isFavorite: node.IsFavorite,
    isManual: node.IsManual,
    isConnected: false,
    lastSeenAt: node.LastSeenAt ?? null,
  };
}

function sessionNodeToApp(node: SessionNode): CastSession {
  return {
    id: node.Id,
    deviceId: node.DeviceId ?? null,
    deviceName: null,
    mediaFileId: node.MediaFileId ?? null,
    episodeId: node.EpisodeId ?? null,
    streamUrl: node.StreamUrl,
    playerState: node.PlayerState as CastSession['playerState'],
    currentTime: node.CurrentPosition,
    duration: node.Duration ?? null,
    volume: node.Volume,
    isMuted: node.IsMuted,
    startedAt: node.StartedAt,
  };
}

function settingNodeToApp(node: SettingNode): CastSettings {
  return {
    autoDiscoveryEnabled: node.AutoDiscoveryEnabled,
    discoveryIntervalSeconds: node.DiscoveryIntervalSeconds,
    defaultVolume: node.DefaultVolume,
    transcodeIncompatible: node.TranscodeIncompatible,
    preferredQuality: node.PreferredQuality ?? null,
  };
}

export interface UseCastResult {
  devices: CastDevice[];
  activeSession: CastSession | null;
  settings: CastSettings | null;
  isLoading: boolean;
  isDiscovering: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  discoverDevices: () => Promise<void>;
  castMedia: (input: CastMediaInput) => Promise<CastSessionResult>;
  play: () => Promise<void>;
  pause: () => Promise<void>;
  stop: () => Promise<void>;
  seek: (position: number) => Promise<void>;
  setVolume: (volume: number) => Promise<void>;
  setMuted: (muted: boolean) => Promise<void>;
}

export function useCast(): UseCastResult {
  const [devices, setDevices] = useState<CastDevice[]>([]);
  const [activeSession, setActiveSession] = useState<CastSession | null>(null);
  const [settings, setSettings] = useState<CastSettings | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isDiscovering, setIsDiscovering] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setError(null);
      const [devicesRes, sessionsRes, settingsRes] = await Promise.all([
        graphqlClient.query(CastDevicesDocument, {}).toPromise(),
        graphqlClient.query(CastSessionsDocument, {}).toPromise(),
        graphqlClient.query(CastSettingsDocument, { Page: { Limit: 1, Offset: 0 } }).toPromise(),
      ]);

      if (devicesRes.data?.CastDevices?.Edges) {
        setDevices(devicesRes.data.CastDevices.Edges.map((e) => deviceNodeToApp(e.Node)));
      }
      if (sessionsRes.data?.CastSessions?.Edges) {
        const sessions = sessionsRes.data.CastSessions.Edges.map((e) => sessionNodeToApp(e.Node));
        setActiveSession(sessions[0] ?? null);
      }
      if (settingsRes.data?.CastSettings?.Edges?.length) {
        setSettings(settingNodeToApp(settingsRes.data.CastSettings.Edges[0].Node));
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load cast data');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const discoverDevices = useCallback(async () => {
    setIsDiscovering(true);
    try {
      const result = await graphqlClient
        .mutation<{ discoverCastDevices: CastDevice[] }>(DISCOVER_CAST_DEVICES_MUTATION, {})
        .toPromise();

      if (result.data?.discoverCastDevices) {
        setDevices(result.data.discoverCastDevices);
      } else {
        await refresh();
      }
    } catch {
      await refresh();
    } finally {
      setIsDiscovering(false);
    }
  }, [refresh]);

  const castMedia = useCallback(async (input: CastMediaInput): Promise<CastSessionResult> => {
    try {
      const result = await graphqlClient
        .mutation<{ castMedia: CastSessionResult }>(CAST_MEDIA_MUTATION, { input })
        .toPromise();

      if (result.data?.castMedia.success && result.data.castMedia.session) {
        setActiveSession(result.data.castMedia.session);
      }

      return result.data?.castMedia ?? { success: false, session: null, error: 'Unknown error' };
    } catch (e) {
      return {
        success: false,
        session: null,
        error: e instanceof Error ? e.message : 'Failed to cast',
      };
    }
  }, []);

  const play = useCallback(async () => {
    if (!activeSession) return;
    try {
      const result = await graphqlClient
        .mutation<{ castPlay: CastSessionResult }>(CAST_PLAY_MUTATION, {
          sessionId: activeSession.id,
        })
        .toPromise();
      if (result.data?.castPlay?.session) {
        setActiveSession((prev) => (prev ? { ...prev, ...result.data!.castPlay!.session! } : null));
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to play');
    }
  }, [activeSession]);

  const pause = useCallback(async () => {
    if (!activeSession) return;
    try {
      const result = await graphqlClient
        .mutation<{ castPause: CastSessionResult }>(CAST_PAUSE_MUTATION, {
          sessionId: activeSession.id,
        })
        .toPromise();
      if (result.data?.castPause?.session) {
        setActiveSession((prev) => (prev ? { ...prev, ...result.data!.castPause!.session! } : null));
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to pause');
    }
  }, [activeSession]);

  const stop = useCallback(async () => {
    if (!activeSession) return;
    try {
      await graphqlClient
        .mutation(CAST_STOP_MUTATION, { sessionId: activeSession.id })
        .toPromise();
      setActiveSession(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to stop');
    }
  }, [activeSession]);

  const seek = useCallback(
    async (position: number) => {
      if (!activeSession) return;
      try {
        const result = await graphqlClient
          .mutation<{ castSeek: CastSessionResult }>(CAST_SEEK_MUTATION, {
            sessionId: activeSession.id,
            position,
          })
          .toPromise();
        if (result.data?.castSeek?.session) {
          setActiveSession((prev) =>
            prev ? { ...prev, ...result.data!.castSeek!.session! } : null,
          );
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to seek');
      }
    },
    [activeSession],
  );

  const setVolume = useCallback(
    async (volume: number) => {
      if (!activeSession) return;
      try {
        const result = await graphqlClient
          .mutation<{ castSetVolume: CastSessionResult }>(CAST_SET_VOLUME_MUTATION, {
            sessionId: activeSession.id,
            volume,
          })
          .toPromise();
        if (result.data?.castSetVolume?.session) {
          setActiveSession((prev) =>
            prev ? { ...prev, ...result.data!.castSetVolume!.session! } : null,
          );
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to set volume');
      }
    },
    [activeSession],
  );

  const setMuted = useCallback(
    async (muted: boolean) => {
      if (!activeSession) return;
      try {
        const result = await graphqlClient
          .mutation<{ castSetMuted: CastSessionResult }>(CAST_SET_MUTED_MUTATION, {
            sessionId: activeSession.id,
            muted,
          })
          .toPromise();
        if (result.data?.castSetMuted?.session) {
          setActiveSession((prev) =>
            prev ? { ...prev, ...result.data!.castSetMuted!.session! } : null,
          );
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to toggle mute');
      }
    },
    [activeSession],
  );

  return {
    devices,
    activeSession,
    settings,
    isLoading,
    isDiscovering,
    error,
    refresh,
    discoverDevices,
    castMedia,
    play,
    pause,
    stop,
    seek,
    setVolume,
    setMuted,
  };
}
