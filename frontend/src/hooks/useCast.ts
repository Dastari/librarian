/**
 * Cast hook for managing Chromecast/media casting state.
 * Uses codegen CastDevices, CastSessions, CastSettings queries.
 * Uses generated mutation documents for custom cast control operations.
 */

import { useState, useEffect, useCallback } from "react";
import {
  CastDevicesDocument,
  CastSessionsDocument,
  CastSettingsDocument,
  DiscoverCastDevicesOpDocument,
  CastMediaOpDocument,
  CastPlayOpDocument,
  CastPauseOpDocument,
  CastStopOpDocument,
  CastSeekOpDocument,
  CastSetVolumeOpDocument,
  CastSetMutedOpDocument,
} from "../lib/graphql/generated/graphql";
import type {
  CastMediaInput,
  CastMediaOpMutationVariables,
  CastPlayOpMutation,
  CastPlayOpMutationVariables,
  CastPauseOpMutation,
  CastPauseOpMutationVariables,
  CastStopOpMutation,
  CastStopOpMutationVariables,
  CastSeekOpMutation,
  CastSeekOpMutationVariables,
  CastSetVolumeOpMutation,
  CastSetVolumeOpMutationVariables,
  CastSetMutedOpMutation,
  CastSetMutedOpMutationVariables,
  CastMediaOpMutation,
  DiscoverCastDevicesOpMutation,
  DiscoverCastDevicesOpMutationVariables,
  CastDevicesQuery,
  CastSessionsQuery,
  CastSettingsQuery,
} from "../lib/graphql/generated/graphql";
import { apolloClient, useMutation } from "../lib/graphql/client";

type DeviceNode = CastDevicesQuery["CastDevices"]["Edges"][0]["Node"];
type SessionNode = CastSessionsQuery["CastSessions"]["Edges"][0]["Node"];
type SettingNode = CastSettingsQuery["CastSettings"]["Edges"][0]["Node"];
export type CastDevice =
  DiscoverCastDevicesOpMutation["DiscoverCastDevices"][number];
export type CastSession = NonNullable<
  CastMediaOpMutation["CastMedia"]["session"]
>;
export type CastSessionResult = {
  success: boolean;
  session: CastSession | null;
  error: string | null;
};
export type CastSettings = {
  autoDiscoveryEnabled: boolean;
  discoveryIntervalSeconds: number;
  defaultVolume: number;
  transcodeIncompatible: boolean;
  preferredQuality: string | null;
};

function deviceNodeToApp(node: DeviceNode): CastDevice {
  return {
    id: node.Id,
    name: node.Name,
    address: node.Address,
    port: node.Port,
    model: node.Model ?? null,
    deviceType: node.DeviceType as CastDevice["deviceType"],
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
    playerState: node.PlayerState as CastSession["playerState"],
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

function normalizeDiscoveredDevice(
  device: DiscoverCastDevicesOpMutation["DiscoverCastDevices"][number],
): CastDevice {
  return {
    id: device.id,
    name: device.name,
    address: device.address,
    port: device.port,
    model: device.model ?? null,
    deviceType: device.deviceType as CastDevice["deviceType"],
    isFavorite: device.isFavorite,
    isManual: device.isManual,
    isConnected: device.isConnected ?? false,
    lastSeenAt: device.lastSeenAt ?? null,
  };
}

function normalizeCastSession(
  session: NonNullable<CastMediaOpMutation["CastMedia"]["session"]>,
): CastSession {
  return {
    id: session.id,
    deviceId: session.deviceId ?? null,
    deviceName: session.deviceName ?? null,
    mediaFileId: session.mediaFileId ?? null,
    episodeId: session.episodeId ?? null,
    streamUrl: session.streamUrl,
    playerState: session.playerState as CastSession["playerState"],
    currentTime: session.currentTime,
    duration: session.duration ?? null,
    volume: session.volume,
    isMuted: session.isMuted,
    startedAt: session.startedAt,
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
  const [discoverCastDevices] = useMutation<
    DiscoverCastDevicesOpMutation,
    DiscoverCastDevicesOpMutationVariables
  >(DiscoverCastDevicesOpDocument);
  const [castMediaMutation] = useMutation<
    CastMediaOpMutation,
    CastMediaOpMutationVariables
  >(CastMediaOpDocument);
  const [castPlayMutation] = useMutation<
    CastPlayOpMutation,
    CastPlayOpMutationVariables
  >(CastPlayOpDocument);
  const [castPauseMutation] = useMutation<
    CastPauseOpMutation,
    CastPauseOpMutationVariables
  >(CastPauseOpDocument);
  const [castStopMutation] = useMutation<
    CastStopOpMutation,
    CastStopOpMutationVariables
  >(CastStopOpDocument);
  const [castSeekMutation] = useMutation<
    CastSeekOpMutation,
    CastSeekOpMutationVariables
  >(CastSeekOpDocument);
  const [castSetVolumeMutation] = useMutation<
    CastSetVolumeOpMutation,
    CastSetVolumeOpMutationVariables
  >(CastSetVolumeOpDocument);
  const [castSetMutedMutation] = useMutation<
    CastSetMutedOpMutation,
    CastSetMutedOpMutationVariables
  >(CastSetMutedOpDocument);

  const refresh = useCallback(async () => {
    try {
      setError(null);
      const [devicesRes, sessionsRes, settingsRes] = await Promise.all([
        apolloClient.query({
          query: CastDevicesDocument,
          fetchPolicy: "network-only",
        }),
        apolloClient.query({
          query: CastSessionsDocument,
          fetchPolicy: "network-only",
        }),
        apolloClient.query({
          query: CastSettingsDocument,
          variables: { Page: { limit: 1, offset: 0 } },
          fetchPolicy: "network-only",
        }),
      ]);

      if (devicesRes.data?.CastDevices?.Edges) {
        setDevices(
          devicesRes.data.CastDevices.Edges.map((e) => deviceNodeToApp(e.Node)),
        );
      }
      if (sessionsRes.data?.CastSessions?.Edges) {
        const sessions = sessionsRes.data.CastSessions.Edges.map((e) =>
          sessionNodeToApp(e.Node),
        );
        setActiveSession(sessions[0] ?? null);
      }
      if (settingsRes.data?.CastSettings?.Edges?.length) {
        setSettings(
          settingNodeToApp(settingsRes.data.CastSettings.Edges[0].Node),
        );
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load cast data");
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
      const result = await discoverCastDevices();
      if (result.data?.DiscoverCastDevices) {
        setDevices(
          result.data.DiscoverCastDevices.map(normalizeDiscoveredDevice),
        );
      } else {
        await refresh();
      }
    } catch {
      await refresh();
    } finally {
      setIsDiscovering(false);
    }
  }, [discoverCastDevices, refresh]);

  const castMedia = useCallback(
    async (input: CastMediaInput): Promise<CastSessionResult> => {
      try {
        const result = await castMediaMutation({ variables: { input } });
        if (result.data?.CastMedia.success && result.data.CastMedia.session) {
          setActiveSession(normalizeCastSession(result.data.CastMedia.session));
        }

        const castResult = result.data?.CastMedia;
        return (
          (castResult
            ? {
                success: castResult.success,
                error: castResult.error ?? null,
                session: castResult.session
                  ? normalizeCastSession(castResult.session)
                  : null,
              }
            : null) ?? {
            success: false,
            session: null,
            error: "Unknown error",
          }
        );
      } catch (e) {
        return {
          success: false,
          session: null,
          error: e instanceof Error ? e.message : "Failed to cast",
        };
      }
    },
    [castMediaMutation],
  );

  const play = useCallback(async () => {
    if (!activeSession) return;
    try {
      const result = await castPlayMutation({
        variables: { sessionId: activeSession.id },
      });
      if (result.data?.CastPlay?.session) {
        const patch = result.data.CastPlay.session;
        setActiveSession((prev) =>
          prev
            ? {
                ...prev,
                id: patch.id,
                playerState: patch.playerState as CastSession["playerState"],
                currentTime: patch.currentTime,
              }
            : null,
        );
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to play");
    }
  }, [activeSession, castPlayMutation]);

  const pause = useCallback(async () => {
    if (!activeSession) return;
    try {
      const result = await castPauseMutation({
        variables: { sessionId: activeSession.id },
      });
      if (result.data?.CastPause?.session) {
        const patch = result.data.CastPause.session;
        setActiveSession((prev) =>
          prev
            ? {
                ...prev,
                id: patch.id,
                playerState: patch.playerState as CastSession["playerState"],
                currentTime: patch.currentTime,
              }
            : null,
        );
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to pause");
    }
  }, [activeSession, castPauseMutation]);

  const stop = useCallback(async () => {
    if (!activeSession) return;
    try {
      await castStopMutation({ variables: { sessionId: activeSession.id } });
      setActiveSession(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to stop");
    }
  }, [activeSession, castStopMutation]);

  const seek = useCallback(
    async (position: number) => {
      if (!activeSession) return;
      try {
        const result = await castSeekMutation({
          variables: {
            sessionId: activeSession.id,
            position,
          },
        });
        if (result.data?.CastSeek?.session) {
          const patch = result.data.CastSeek.session;
          setActiveSession((prev) =>
            prev
              ? {
                  ...prev,
                  id: patch.id,
                  playerState: patch.playerState as CastSession["playerState"],
                  currentTime: patch.currentTime,
                }
              : null,
          );
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to seek");
      }
    },
    [activeSession, castSeekMutation],
  );

  const setVolume = useCallback(
    async (volume: number) => {
      if (!activeSession) return;
      try {
        const result = await castSetVolumeMutation({
          variables: {
            sessionId: activeSession.id,
            volume,
          },
        });
        if (result.data?.CastSetVolume?.session) {
          const patch = result.data.CastSetVolume.session;
          setActiveSession((prev) =>
            prev
              ? {
                  ...prev,
                  id: patch.id,
                  volume: patch.volume,
                  isMuted: patch.isMuted,
                }
              : null,
          );
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to set volume");
      }
    },
    [activeSession, castSetVolumeMutation],
  );

  const setMuted = useCallback(
    async (muted: boolean) => {
      if (!activeSession) return;
      try {
        const result = await castSetMutedMutation({
          variables: {
            sessionId: activeSession.id,
            muted,
          },
        });
        if (result.data?.CastSetMuted?.session) {
          const patch = result.data.CastSetMuted.session;
          setActiveSession((prev) =>
            prev
              ? {
                  ...prev,
                  id: patch.id,
                  volume: patch.volume,
                  isMuted: patch.isMuted,
                }
              : null,
          );
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to toggle mute");
      }
    },
    [activeSession, castSetMutedMutation],
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
