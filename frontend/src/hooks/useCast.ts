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

type DeviceNode = CastDevicesQuery["castDevices"]["edges"][0]["node"];
type SessionNode = CastSessionsQuery["castSessions"]["edges"][0]["node"];
type SettingNode = CastSettingsQuery["castSettings"]["edges"][0]["node"];
export type CastDevice =
  DiscoverCastDevicesOpMutation["discoverCastDevices"][number];
export type CastSession = NonNullable<
  CastMediaOpMutation["castMedia"]["session"]
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
    id: node.id,
    name: node.name,
    address: node.address,
    port: node.port,
    model: node.model ?? null,
    deviceType: node.deviceType as CastDevice["deviceType"],
    isFavorite: node.isFavorite,
    isManual: node.isManual,
    isConnected: false,
    enabled: node.enabled ?? true,
    playbackSupported: node.playbackSupported ?? false,
    discoveryOrigin: node.discoveryOrigin ?? null,
    lastSeenAt: node.lastSeenAt ?? null,
  };
}

function sessionNodeToApp(node: SessionNode): CastSession {
  return {
    id: node.id,
    deviceId: node.deviceId ?? null,
    deviceName: null,
    mediaFileId: node.mediaFileId ?? null,
    episodeId: node.episodeId ?? null,
    playerState: node.playerState as CastSession["playerState"],
    currentTime: node.currentPosition,
    duration: node.duration ?? null,
    volume: node.volume,
    isMuted: node.isMuted,
    startedAt: node.startedAt,
    lastError: node.lastError ?? null,
    playbackDecision: node.playbackDecision ?? null,
    playbackReason: node.playbackReason ?? null,
  };
}

function settingNodeToApp(node: SettingNode): CastSettings {
  return {
    autoDiscoveryEnabled: node.autoDiscoveryEnabled,
    discoveryIntervalSeconds: node.discoveryIntervalSeconds,
    defaultVolume: node.defaultVolume,
    transcodeIncompatible: node.transcodeIncompatible,
    preferredQuality: node.preferredQuality ?? null,
  };
}

function normalizeDiscoveredDevice(
  device: DiscoverCastDevicesOpMutation["discoverCastDevices"][number],
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
    enabled: device.enabled,
    playbackSupported: device.playbackSupported,
    discoveryOrigin: device.discoveryOrigin ?? null,
    lastSeenAt: device.lastSeenAt ?? null,
  };
}

function normalizeCastSession(
  session: NonNullable<CastMediaOpMutation["castMedia"]["session"]>,
): CastSession {
  return {
    id: session.id,
    deviceId: session.deviceId ?? null,
    deviceName: session.deviceName ?? null,
    mediaFileId: session.mediaFileId ?? null,
    episodeId: session.episodeId ?? null,
    playerState: session.playerState as CastSession["playerState"],
    currentTime: session.currentTime,
    duration: session.duration ?? null,
    volume: session.volume,
    isMuted: session.isMuted,
    startedAt: session.startedAt,
    lastError: session.lastError ?? null,
    playbackDecision: session.playbackDecision ?? null,
    playbackReason: session.playbackReason ?? null,
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
          variables: {
            orderBy: [{ startedAt: "DESC" }],
            page: { limit: 20, offset: 0 },
          },
          fetchPolicy: "network-only",
        }),
        apolloClient.query({
          query: CastSettingsDocument,
          variables: { page: { limit: 1, offset: 0 } },
          fetchPolicy: "network-only",
        }),
      ]);

      if (devicesRes.data?.castDevices?.edges) {
        setDevices(
          devicesRes.data.castDevices.edges.map((e) => deviceNodeToApp(e.node)),
        );
      }
      if (sessionsRes.data?.castSessions?.edges) {
        const sessions = sessionsRes.data.castSessions.edges.map((e) =>
          sessionNodeToApp(e.node),
        );
        setActiveSession(
          sessions.find(
            (session) =>
              !["ENDED", "IDLE"].includes(
                session.playerState,
              ),
          ) ?? null,
        );
      }
      if (settingsRes.data?.castSettings?.edges?.length) {
        setSettings(
          settingNodeToApp(settingsRes.data.castSettings.edges[0].node),
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

  useEffect(() => {
    if (!activeSession) return;
    const timer = window.setInterval(() => void refresh(), 5000);
    return () => window.clearInterval(timer);
  }, [activeSession?.id, refresh]);

  const discoverDevices = useCallback(async () => {
    setIsDiscovering(true);
    try {
      const result = await discoverCastDevices();
      if (result.data?.discoverCastDevices) {
        setDevices(
          result.data.discoverCastDevices.map(normalizeDiscoveredDevice),
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
        if (result.data?.castMedia.success && result.data.castMedia.session) {
          setActiveSession(normalizeCastSession(result.data.castMedia.session));
        }

        const castResult = result.data?.castMedia;
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
      if (result.data?.castPlay?.session) {
        const patch = result.data.castPlay.session;
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
      if (result.data?.castPause?.session) {
        const patch = result.data.castPause.session;
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
    if (["FAILED", "DISCONNECTED", "ENDED", "IDLE"].includes(activeSession.playerState)) {
      setActiveSession(null);
      return;
    }
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
        if (result.data?.castSeek?.session) {
          const patch = result.data.castSeek.session;
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
        if (result.data?.castSetVolume?.session) {
          const patch = result.data.castSetVolume.session;
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
        if (result.data?.castSetMuted?.session) {
          const patch = result.data.castSetMuted.session;
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
