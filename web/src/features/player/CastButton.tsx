import { useMutation } from "@apollo/client/react";
import { Button, type ButtonVariant } from "glass-ux/button";
import { Dialog } from "glass-ux/dialog";
import { IconDisplays } from "glass-ux/icons";
import { ListRow } from "glass-ux/list-row";
import { Slider } from "glass-ux/slider";
import { useEffect, useMemo, useRef, useState } from "react";

import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import {
  CastMediaDocument, CastPauseDocument, CastPlayDocument, CastSeekDocument,
  CastSetMutedDocument, CastSetVolumeDocument, CastStopDocument,
  EntityCastDeviceListDocument, EntityCastSessionGetDocument, EntityCastSessionListDocument,
  type CastDeviceFieldsFragment, type CastSessionFieldsFragment,
} from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { formatClock } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveEpoch, useLiveQuery } from "@/lib/graphql/live/hooks";
import { liveTransport } from "@/lib/graphql/live/transport";

import type { PlayItem } from "./store";

export interface CastButtonProps {
  item?: PlayItem;
  /** Current local playback position, in seconds. */
  position?: number;
  /** Identity of this playback activation, including same-file restarts. */
  activationKey?: string | number;
  /** Pause local playback here; called only after a successful remote start. */
  onStarted?: () => void;
  className?: string;
  variant?: ButtonVariant;
}

function deviceReason(device: CastDeviceFieldsFragment): string | undefined {
  const type = device.deviceType.trim().toUpperCase();
  if (type === "DLNA_RENDERER") return "DLNA discovery only; playback is unavailable.";
  if (device.enabled !== true) return "This receiver is disabled.";
  if (!["CHROMECAST", "CHROMECAST_AUDIO"].includes(type) || device.playbackSupported !== true) {
    return "Playback requires a supported Chromecast or Google TV receiver.";
  }
}

/** One entry point for receiver selection and the current user's persisted sessions. */
export function CastButton(props: CastButtonProps) {
  const { user } = useSession();
  const epoch = useLiveEpoch();
  // Drop selections, retry closures and in-flight UI state on authority replacement.
  return <CastDialog key={`${epoch}:${user?.id ?? "anonymous"}`} {...props} userId={user?.id} />;
}

function CastDialog({ item, position, activationKey, onStarted, className, variant = "ghost", userId }: CastButtonProps & { userId?: string }) {
  const [open, setOpen] = useState(false);
  const [deviceId, setDeviceId] = useState<string>();
  const [sessionId, setSessionId] = useState<string>();
  const [pending, setPending] = useState<string>();
  const [failure, setFailure] = useState<string>();
  const [announcement, setAnnouncement] = useState("");
  const [connected, setConnected] = useState<boolean>();
  const busy = useRef(false);
  const mounted = useRef(true);
  const origin = useMemo(() => ({}), [activationKey, item?.mediaFileId, item?.entity.kind, item?.entity.id]);
  const currentOrigin = useRef(origin);
  currentOrigin.current = origin;
  const isOrigin = () => mounted.current && currentOrigin.current === origin;
  const retry = useRef<(() => Promise<void>) | undefined>(undefined);
  const trigger = useRef<HTMLElement | null>(null);
  const refreshButton = useRef<HTMLElement | null>(null);
  const openRef = useRef(open);
  openRef.current = open;
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  useEffect(() => liveTransport.subscribe(setConnected), []);

  const devices = useSettingsConnection(EntityCastDeviceListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.castDevices, { skip: !open || !userId });
  const sessions = useSettingsConnection(EntityCastSessionListDocument, {
    // Administrators can read all sessions: ownership must be explicit here too.
    where: { userId: { eq: userId ?? "" }, endedAt: { isNull: true } },
    orderBy: [{ startedAt: "DESC" }],
  }, (data) => data.castSessions, { skip: !open || !userId });
  const owned = sessions.rows.filter((row) => row.userId === userId && row.endedAt === null);
  const selected = owned.find((row) => row.id === sessionId) ?? owned[0];
  useEffect(() => {
    // A newly started ID may arrive before the reconciled list. Keep that ID
    // while the command is pending instead of replacing it with an older row.
    if (selected && selected.id !== sessionId && !pending) setSessionId(selected.id);
  }, [selected?.id, sessionId, pending]);
  useEffect(() => {
    setFailure(undefined);
    retry.current = undefined;
  }, [selected?.id, origin, deviceId]);
  const detail = useLiveQuery(EntityCastSessionGetDocument, {
    variables: { id: selected?.id ?? "" }, skip: !open || !selected,
    fetchPolicy: "network-only",
  });
  const detailRow = detail.data?.castSession;
  const active = detailRow?.id === selected?.id && detailRow?.userId === userId && detailRow?.endedAt === null
    ? detailRow : selected;
  const device = devices.rows.find((row) => row.id === deviceId);
  const [cast] = useMutation(CastMediaDocument);
  const [play] = useMutation(CastPlayDocument);
  const [pause] = useMutation(CastPauseDocument);
  const [seek] = useMutation(CastSeekDocument);
  const [volume] = useMutation(CastSetVolumeDocument);
  const [mute] = useMutation(CastSetMutedDocument);
  const [stop] = useMutation(CastStopDocument);

  const refresh = async () => {
    await Promise.all([devices.refetch(), sessions.refetch(), ...(selected ? [detail.refetch().catch(() => undefined)] : [])]);
  };
  const run = async (label: string, action: () => Promise<void>, isCurrent: () => boolean = () => true) => {
    if (busy.current || !userId || !isCurrent()) return;
    busy.current = true;
    setPending(label);
    setFailure(undefined);
    retry.current = undefined;
    try {
      await action();
      if (!mounted.current || !isCurrent()) return;
      setAnnouncement(`${label} succeeded.`);
      await refresh();
    } catch (cause) {
      if (mounted.current && isCurrent()) {
        setFailure(errorMessage(cause));
        retry.current = () => run(label, action, isCurrent);
      }
    } finally {
      busy.current = false;
      if (mounted.current) setPending(undefined);
    }
  };
  const start = async () => {
    if (!isOrigin() || !item || !device || deviceReason(device)) return;
    const startPosition = position ?? item.startPosition;
    const payload = assertSuccess((await cast({ variables: { input: {
      deviceId: device.id, mediaFileId: item.mediaFileId,
      ...(item.entity.kind === "episode" ? { episodeId: item.entity.id } : {}),
      ...(typeof startPosition === "number" && Number.isFinite(startPosition) ? { startPosition: Math.max(0, startPosition) } : {}),
    } } })).data?.castMedia, "Could not start casting");
    if (!payload.session?.id) throw new Error("The receiver did not return a cast session. Retry to reconnect.");
    if (!isOrigin()) return;
    setSessionId(payload.session.id);
    onStarted?.();
  };
  const chooseSession = (id: string) => {
    if (busy.current) return;
    setSessionId(id);
    setFailure(undefined);
    retry.current = undefined;
  };
  const startReason = !userId ? "Sign in to cast." : !item ? "Choose media in a player to start casting."
    : !device ? "Choose a receiver." : deviceReason(device);
  const queryError = devices.error || sessions.error || detail.error;

  return (
    <Dialog.Root open={open} onOpenChange={setOpen}>
      <Dialog.Trigger render={<Button ref={trigger} variant={variant} shape="circle" size="sm" className={className} aria-label={item ? "Cast" : "Casting"} data-focusable><IconDisplays /></Button>} />
      <Dialog.Popup size="md" finalFocus={trigger}>
        <Dialog.Header title="Casting" icon={<IconDisplays />} subtitle={item?.title} />
        <Dialog.Body className="flex flex-col gap-4">
          <div role="status" aria-live="polite" className="min-h-6 text-sm">
            {pending ? `${pending}…` : connected === false ? "Live updates disconnected. Reconnecting…" : connected === true ? "Live updates connected." : "Receiver status updates appear here."}
          </div>
          <div aria-live="polite">
            <div className="min-h-12">
              {queryError ? <p role="alert">Could not load casting: {errorMessage(queryError)}</p> : null}
              {failure ? <p role="alert">{failure}</p> : null}
            </div>
            <span className="sr-only">{announcement}</span>
            <Button ref={refreshButton} variant="ghost" size="sm" tooltip="Refresh casting" onClick={() => {
              if (failure && retry.current) void retry.current();
              else void refresh();
            }} disabledReason={pending ? "Wait for the current cast command." : undefined}>{queryError || failure || active?.lastError ? "Retry" : "Refresh"}</Button>
          </div>
          {!userId ? <p>Sign in to manage your casting sessions.</p> : null}
          <section aria-label="Active casting sessions" className="flex flex-col gap-2">
            {sessions.loading ? <p>Loading your casting sessions…</p> : null}
            {!sessions.loading && !sessions.error && !owned.length ? <p>No active casting sessions. {item ? "Choose a receiver below." : "Choose media in a player, then use Cast to send it to a receiver."}</p> : null}
            {owned.map((row) => <ListRow key={row.id} title={devices.rows.find((receiver) => receiver.id === row.deviceId)?.name ?? "Cast receiver"} meta={row.playerState} selected={active?.id === row.id} aria-label={`Control ${devices.rows.find((receiver) => receiver.id === row.deviceId)?.name ?? row.id}`} onClick={() => chooseSession(row.id)} disabled={Boolean(pending)} focusableWhenDisabled />)}
            {active ? <SessionControls key={active.id} session={active} pending={pending} run={run}
              play={async () => { assertSuccess((await play({ variables: { sessionId: active.id } })).data?.castPlay, "Could not play on receiver"); }}
              pause={async () => { assertSuccess((await pause({ variables: { sessionId: active.id } })).data?.castPause, "Could not pause receiver"); }}
              seek={async (position) => { assertSuccess((await seek({ variables: { sessionId: active.id, position } })).data?.castSeek, "Could not seek receiver"); }}
              volume={async (value) => { assertSuccess((await volume({ variables: { sessionId: active.id, volume: value } })).data?.castSetVolume, "Could not change receiver volume"); }}
              mute={async () => { assertSuccess((await mute({ variables: { sessionId: active.id, muted: !active.isMuted } })).data?.castSetMuted, "Could not mute receiver"); }}
              stop={async () => {
                assertSuccess((await stop({ variables: { sessionId: active.id } })).data?.castStop, "Could not stop casting");
                // Controls can disappear after the stopped session leaves the live list.
                if (mounted.current && openRef.current) refreshButton.current?.focus();
              }} /> : null}
          </section>
          <section aria-label="Cast receivers" className="flex flex-col gap-2">
            {devices.loading ? <p>Loading receivers…</p> : null}
            {!devices.loading && !devices.error && !devices.rows.length ? <p>No receivers found. A Chromecast or Google TV must be reachable on your server’s network.</p> : null}
            {devices.rows.map((row) => {
              const reason = deviceReason(row);
              return <ListRow key={row.id} title={row.name} meta={reason ?? row.lastProbeError ?? row.model ?? row.address}
                selected={row.id === deviceId} aria-label={row.name} aria-description={reason} disabled={Boolean(reason || pending)} focusableWhenDisabled
                onClick={() => { if (!reason && !busy.current) { setDeviceId(row.id); setFailure(undefined); retry.current = undefined; } }} />;
            })}
            {!item ? <p>Start casting from the media’s player. Active receiver controls remain available here after navigation.</p> : null}
          </section>
        </Dialog.Body>
        <Dialog.Footer>
          <Dialog.Close render={<Button variant="ghost">Close</Button>} />
          <Button variant="primary" tooltip="Start casting" aria-label="Start casting" disabledReason={pending ? "Wait for the current cast command." : startReason} status={pending === "Start casting" ? "pending" : "idle"} onClick={() => void run("Start casting", start, isOrigin)}>Start casting</Button>
        </Dialog.Footer>
      </Dialog.Popup>
    </Dialog.Root>
  );
}

interface SessionControlsProps {
  session: CastSessionFieldsFragment;
  pending?: string;
  run: (label: string, action: () => Promise<void>) => Promise<void>;
  play: () => Promise<void>;
  pause: () => Promise<void>;
  seek: (position: number) => Promise<void>;
  volume: (volume: number) => Promise<void>;
  mute: () => Promise<void>;
  stop: () => Promise<void>;
}

function SessionControls({ session, pending, run, play, pause, seek, volume, mute, stop }: SessionControlsProps) {
  const [seekDraft, setSeekDraft] = useState<number>();
  const [volumeDraft, setVolumeDraft] = useState<number>();
  const playing = session.playerState.toUpperCase() === "PLAYING";
  const duration = typeof session.duration === "number" && Number.isFinite(session.duration) && session.duration > 0 ? session.duration : 0;
  const reason = pending ? "Wait for the current cast command." : undefined;
  return <div role="group" aria-label="Receiver playback" className="flex flex-col gap-3">
    <p>Receiver: {session.playerState}</p>
    <p role={session.lastError ? "alert" : undefined} className="min-h-6 text-sm">{session.lastError ?? session.playbackReason}</p>
    <div className="flex flex-wrap gap-2">
      <Button tooltip="Receiver playback" aria-label={playing ? "Pause" : "Play"} disabledReason={reason} onClick={() => void run(playing ? "Pause" : "Play", playing ? pause : play)}>{playing ? "Pause" : "Play"}</Button>
      <Button tooltip="Receiver mute" aria-label={session.isMuted ? "Unmute" : "Mute"} disabledReason={reason} onClick={() => void run(session.isMuted ? "Unmute" : "Mute", mute)}>{session.isMuted ? "Unmute" : "Mute"}</Button>
      <Button variant="danger-soft" tooltip="Stop casting" disabledReason={reason} onClick={() => void run("Stop casting", stop)}>Stop casting</Button>
    </div>
    <p className="text-sm tabular-nums">{formatClock(seekDraft ?? session.currentPosition)} / {formatClock(duration)}</p>
    <Slider aria-label="Receiver position" min={0} max={duration || 1} step={1} format={formatClock}
      value={Math.min(duration || 1, Math.max(0, seekDraft ?? session.currentPosition))} disabled={!duration}
      onValueChange={(value) => { if (!pending && typeof value === "number") setSeekDraft(value); }}
      onValueCommitted={(value) => { if (!pending && typeof value === "number" && Number.isFinite(value)) void run("Seek", async () => { try { await seek(value); } finally { setSeekDraft(undefined); } }); }} />
    {!duration ? <p className="text-sm">Seek is unavailable until the receiver reports a duration.</p> : null}
    <Slider aria-label="Receiver volume" min={0} max={1} step={0.01} format={(value) => `${Math.round(value * 100)}%`}
      value={volumeDraft ?? Math.max(0, Math.min(1, session.volume))}
      onValueChange={(value) => { if (!pending && typeof value === "number") setVolumeDraft(value); }}
      onValueCommitted={(value) => { if (!pending && typeof value === "number" && Number.isFinite(value)) void run("Set volume", async () => { try { await volume(value); } finally { setVolumeDraft(undefined); } }); }} />
  </div>;
}
