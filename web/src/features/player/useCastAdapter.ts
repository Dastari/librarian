import { useMutation } from "@apollo/client/react";
import type { CastAdapter, CastSession } from "glass-ux/media-player";
import { useEffect, useMemo, useRef, useState } from "react";

import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import {
  CastPauseDocument, CastPlayDocument, CastSeekDocument, CastSetMutedDocument, CastSetVolumeDocument, CastStopDocument,
  EntityCastDeviceListDocument, EntityCastSessionListDocument, type CastSessionFieldsFragment,
} from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { assertSuccess } from "@/lib/graphql/errors";

const AVAILABLE: CastSession = { status: "available" };

function toSession(row: CastSessionFieldsFragment, deviceName: string | undefined): CastSession {
  const duration = typeof row.duration === "number" && Number.isFinite(row.duration) && row.duration > 0 ? row.duration : undefined;
  return {
    status: "connected",
    deviceName,
    currentTime: row.currentPosition,
    duration,
    paused: row.playerState.toUpperCase() !== "PLAYING",
    volume: Math.max(0, Math.min(1, row.volume)),
    muted: row.isMuted,
  };
}

export interface BackendCast {
  adapter: CastAdapter;
  /** The receiver picker is open: the player's cast button asked for it. */
  pickerOpen: boolean;
  setPickerOpen: (open: boolean) => void;
  /** Re-read sessions now, after the picker started one. */
  refresh: () => void;
}

/**
 * Glass UX's casting seam over the server's cast sessions. The server is the sender: it holds
 * the receiver connection, so the adapter only reads the signed-in user's live session for
 * this file and sends commands. Starting a session happens in the picker, which passes the
 * position itself, so `load` has nothing left to do.
 */
export function useCastAdapter(mediaFileId: string): BackendCast {
  const { user } = useSession();
  const userId = user?.id;
  const [pickerOpen, setPickerOpen] = useState(false);
  const sessions = useSettingsConnection(EntityCastSessionListDocument, {
    // Administrators can read all sessions: ownership must be explicit here too.
    where: { userId: { eq: userId ?? "" }, mediaFileId: { eq: mediaFileId }, endedAt: { isNull: true } },
    orderBy: [{ startedAt: "DESC" }],
  }, (data) => data.castSessions, { skip: !userId });
  const devices = useSettingsConnection(EntityCastDeviceListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.castDevices, { skip: !userId });
  const row = sessions.rows.find((candidate) => candidate.userId === userId && candidate.mediaFileId === mediaFileId && candidate.endedAt === null);
  const deviceName = devices.rows.find((device) => device.id === row?.deviceId)?.name;

  const [play] = useMutation(CastPlayDocument);
  const [pause] = useMutation(CastPauseDocument);
  const [seek] = useMutation(CastSeekDocument);
  const [setVolume] = useMutation(CastSetVolumeDocument);
  const [setMuted] = useMutation(CastSetMutedDocument);
  const [stop] = useMutation(CastStopDocument);

  const sessionId = useRef<string | undefined>(undefined);
  const snapshot = useRef<CastSession>(AVAILABLE);
  const listeners = useRef(new Set<() => void>());
  const commands = useRef({ play, pause, seek, setVolume, setMuted, stop });
  commands.current = { play, pause, seek, setVolume, setMuted, stop };

  // Keep the same object until something changes, as the adapter contract asks.
  const signature = row ? JSON.stringify([row.id, row.currentPosition, row.duration, row.playerState, row.volume, row.isMuted, deviceName]) : "";
  useEffect(() => {
    sessionId.current = row?.id;
    snapshot.current = row ? toSession(row, deviceName) : AVAILABLE;
    for (const listener of listeners.current) listener();
    // `signature` holds every field read from `row`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature]);

  const adapter = useMemo<CastAdapter>(() => {
    const command = async (run: (id: string) => Promise<unknown>) => {
      const id = sessionId.current;
      if (id) await run(id);
    };
    return {
      getSession: () => snapshot.current,
      subscribe: (listener) => {
        listeners.current.add(listener);
        return () => listeners.current.delete(listener);
      },
      connect: () => setPickerOpen(true),
      disconnect: () => command(async (id) => { assertSuccess((await commands.current.stop({ variables: { sessionId: id } })).data?.castStop, "Could not stop casting"); }),
      load: () => undefined,
      play: () => command(async (id) => { assertSuccess((await commands.current.play({ variables: { sessionId: id } })).data?.castPlay, "Could not play on receiver"); }),
      pause: () => command(async (id) => { assertSuccess((await commands.current.pause({ variables: { sessionId: id } })).data?.castPause, "Could not pause receiver"); }),
      seek: (position) => command(async (id) => { assertSuccess((await commands.current.seek({ variables: { sessionId: id, position } })).data?.castSeek, "Could not seek receiver"); }),
      stop: () => command(async (id) => { assertSuccess((await commands.current.stop({ variables: { sessionId: id } })).data?.castStop, "Could not stop casting"); }),
      setVolume: (volume) => command(async (id) => { assertSuccess((await commands.current.setVolume({ variables: { sessionId: id, volume } })).data?.castSetVolume, "Could not change receiver volume"); }),
      setMuted: (muted) => command(async (id) => { assertSuccess((await commands.current.setMuted({ variables: { sessionId: id, muted } })).data?.castSetMuted, "Could not mute receiver"); }),
    };
  }, []);

  return { adapter, pickerOpen, setPickerOpen, refresh: () => void sessions.refetch() };
}
