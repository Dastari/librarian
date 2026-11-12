import { useMutation } from "@apollo/client/react";
import { Button, type ButtonVariant } from "glass-ux/button";
import { Dialog } from "glass-ux/dialog";
import { IconDisplays, IconPause, IconPlay, IconRefresh, IconStop, IconVolume, IconVolumeOff } from "glass-ux/icons";
import { ListRow } from "glass-ux/list-row";
import { Notice } from "glass-ux/notice";
import { Slider } from "glass-ux/slider";
import { useEffect, useMemo, useRef, useState } from "react";

import { makeAndModel, playbackBlocker, protocolLabel } from "@/features/casting/castDevice";
import { castDeviceGlyph } from "@/features/casting/CastDeviceIdentity";

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
  /** Called only after a successful remote start. */
  onStarted?: () => void;
  className?: string;
  variant?: ButtonVariant;
  /** Controlled picker, for a player whose own cast button opens it. */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  /** No button here: something else opens the picker. */
  hideTrigger?: boolean;
}

function deviceReason(device: CastDeviceFieldsFragment): string | undefined {
  return playbackBlocker(device);
}

/** Favourites first, then by name. */
const byFavourite = (a: CastDeviceFieldsFragment, b: CastDeviceFieldsFragment) => Number(b.isFavorite) - Number(a.isFavorite) || a.name.localeCompare(b.name);

function ReceiverRow({ device, selected, reason, disabled, onChoose }: { device: CastDeviceFieldsFragment; selected: boolean; reason?: string; disabled: boolean; onChoose: () => void }) {
  const Glyph = castDeviceGlyph(device);
  const said = makeAndModel(device);
  return (
    <ListRow
      icon={<Glyph />}
      title={device.isFavorite ? <>{device.name} <span aria-hidden className="text-accent">★</span></> : device.name}
      meta={reason ?? (device.lastProbeError ? `Not responding: ${device.lastProbeError}` : [said, protocolLabel(device)].filter(Boolean).join(" · "))}
      selected={selected}
      aria-label={device.name}
      aria-description={reason}
      disabled={disabled}
      focusableWhenDisabled
      onClick={onChoose}
    />
  );
}

/** One entry point for receiver selection and the current user's persisted sessions. */
export function CastButton(props: CastButtonProps) {
  const { user } = useSession();
  const epoch = useLiveEpoch();
  // Drop selections, retry closures and in-flight UI state on authority replacement.
  return <CastDialog key={`${epoch}:${user?.id ?? "anonymous"}`} {...props} userId={user?.id} />;
}

function CastDialog({ item, position, activationKey, onStarted, className, variant = "ghost", userId, open: controlledOpen, onOpenChange, hideTrigger }: CastButtonProps & { userId?: string }) {
  const [ownOpen, setOwnOpen] = useState(false);
  const open = controlledOpen ?? ownOpen;
  const setOpen = (next: boolean) => {
    setOwnOpen(next);
    onOpenChange?.(next);
  };
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

  const ready = devices.rows.filter((row) => !deviceReason(row)).sort(byFavourite);
  const unavailable = devices.rows.filter((row) => deviceReason(row)).sort(byFavourite);
  const receiverName = (id: string | null | undefined) => devices.rows.find((receiver) => receiver.id === id)?.name ?? "Cast receiver";
  const problem = queryError ? `Could not load casting: ${errorMessage(queryError)}` : failure;

  return (
    <Dialog.Root open={open} onOpenChange={setOpen}>
      {hideTrigger ? null : <Dialog.Trigger render={<Button ref={trigger} variant={variant} shape="circle" size="sm" className={className} aria-label={item ? "Cast" : "Casting"} data-focusable><IconDisplays /></Button>} />}
      <Dialog.Popup size="md" finalFocus={trigger}>
        <Dialog.Header title={item ? "Cast" : "Casting"} icon={<IconDisplays />} subtitle={item?.title} />
        <Dialog.Body className="flex flex-col gap-5">
          <div role="status" aria-live="polite" className={pending || connected === false ? "text-body-sm text-muted" : "sr-only"}>
            {pending ? `${pending}…` : connected === false ? "Live updates disconnected. Reconnecting…" : ""}
          </div>
          <div aria-live="polite" className="contents">
            {problem ? (
              <Notice tone="danger" role="alert" action={<Button ref={refreshButton} size="sm" variant="ghost" onClick={() => {
                if (failure && retry.current) void retry.current();
                else void refresh();
              }} disabledReason={pending ? "Wait for the current cast command." : undefined}>Retry</Button>}>{problem}</Notice>
            ) : null}
            <span className="sr-only">{announcement}</span>
          </div>
          {!userId ? <p className="text-body-sm text-muted">Sign in to cast.</p> : null}

          {owned.length ? (
            <section aria-label="Active casting sessions" className="flex flex-col gap-2">
              <h3 className="text-label-sm font-medium text-muted">Now casting</h3>
              {owned.length > 1 ? owned.map((row) => <ListRow key={row.id} title={receiverName(row.deviceId)} meta={row.playerState} selected={active?.id === row.id} aria-label={`Control ${receiverName(row.deviceId)}`} onClick={() => chooseSession(row.id)} disabled={Boolean(pending)} focusableWhenDisabled />) : null}
              {active ? <SessionControls key={active.id} session={active} receiver={receiverName(active.deviceId)} pending={pending} run={run}
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
          ) : sessions.loading ? <p className="text-body-sm text-muted">Loading your casting sessions…</p> : null}

          <section aria-label="Cast receivers" className="flex flex-col gap-2">
            <div className="flex items-center justify-between gap-2">
              <h3 className="text-label-sm font-medium text-muted">{item ? "Cast to" : "Receivers"}</h3>
              {problem ? null : <Button ref={refreshButton} variant="ghost" shape="circle" size="sm" aria-label="Refresh" tooltip="Refresh" onClick={() => void refresh()} disabledReason={pending ? "Wait for the current cast command." : undefined} icon={<IconRefresh />} />}
            </div>
            {devices.loading ? <p className="text-body-sm text-muted">Loading receivers…</p> : null}
            {ready.map((row) => <ReceiverRow key={row.id} device={row} selected={row.id === deviceId} disabled={Boolean(pending)}
              onChoose={() => { if (!busy.current) { setDeviceId(row.id); setFailure(undefined); retry.current = undefined; } }} />)}
            {!devices.loading && !devices.error && !ready.length ? (
              <p className="text-body-sm text-muted">
                {devices.rows.length ? "None of the receivers found can play from Librarian yet." : "No receivers found."} Librarian casts to Google Cast receivers (a Chromecast, a Google TV, or a TV with Chromecast built in) on the server's own network.
              </p>
            ) : null}
            {unavailable.length ? (
              <details className="group flex flex-col gap-2" open={!ready.length}>
                <summary className="cursor-pointer py-1 text-body-sm text-muted">Can't play from Librarian ({unavailable.length})</summary>
                <div className="flex flex-col gap-1 pt-1">
                  {unavailable.map((row) => <ReceiverRow key={row.id} device={row} selected={false} reason={deviceReason(row)} disabled onChoose={() => undefined} />)}
                </div>
              </details>
            ) : null}
            {!item && !owned.length ? <p className="text-body-sm text-muted">Start casting from a player's Cast button. What is casting can be controlled here from anywhere.</p> : null}
          </section>
        </Dialog.Body>
        <Dialog.Footer>
          <Dialog.Close render={<Button variant="ghost">Close</Button>} />
          {item ? <Button variant="primary" tooltip={device ? `Cast to ${device.name}` : "Start casting"} aria-label="Start casting" disabledReason={pending ? "Wait for the current cast command." : startReason} status={pending === "Start casting" ? "pending" : "idle"} onClick={() => void run("Start casting", start, isOrigin)}>{device ? `Cast to ${device.name}` : "Start casting"}</Button> : null}
        </Dialog.Footer>
      </Dialog.Popup>
    </Dialog.Root>
  );
}

interface SessionControlsProps {
  session: CastSessionFieldsFragment;
  receiver: string;
  pending?: string;
  run: (label: string, action: () => Promise<void>) => Promise<void>;
  play: () => Promise<void>;
  pause: () => Promise<void>;
  seek: (position: number) => Promise<void>;
  volume: (volume: number) => Promise<void>;
  mute: () => Promise<void>;
  stop: () => Promise<void>;
}

const STATE_LABEL: Record<string, string> = { PLAYING: "Playing", PAUSED: "Paused", BUFFERING: "Buffering", IDLE: "Idle", LOADING: "Loading" };

function SessionControls({ session, receiver, pending, run, play, pause, seek, volume, mute, stop }: SessionControlsProps) {
  const [seekDraft, setSeekDraft] = useState<number>();
  const [volumeDraft, setVolumeDraft] = useState<number>();
  const state = session.playerState.toUpperCase();
  const playing = state === "PLAYING";
  const duration = typeof session.duration === "number" && Number.isFinite(session.duration) && session.duration > 0 ? session.duration : 0;
  const reason = pending ? "Wait for the current cast command." : undefined;
  return <div role="group" aria-label="Receiver playback" className="flex flex-col gap-3 rounded-card bg-surface-secondary p-4">
    <div className="flex items-baseline justify-between gap-3">
      <p className="truncate text-title-sm text-foreground">{receiver}</p>
      <p className="shrink-0 text-label-sm text-muted">{STATE_LABEL[state] ?? session.playerState}</p>
    </div>
    {session.lastError ? <p role="alert" className="text-body-sm text-danger">{session.lastError}</p> : session.playbackReason ? <p className="text-label-sm text-muted">{session.playbackReason}</p> : null}
    <div className="flex flex-col gap-1">
      <Slider aria-label="Receiver position" min={0} max={duration || 1} step={1} format={formatClock}
        value={Math.min(duration || 1, Math.max(0, seekDraft ?? session.currentPosition))} disabled={!duration}
        onValueChange={(value) => { if (!pending && typeof value === "number") setSeekDraft(value); }}
        onValueCommitted={(value) => { if (!pending && typeof value === "number" && Number.isFinite(value)) void run("Seek", async () => { try { await seek(value); } finally { setSeekDraft(undefined); } }); }} />
      <p className="text-numeric text-label-sm text-muted">{formatClock(seekDraft ?? session.currentPosition)} / {formatClock(duration)}</p>
      {!duration ? <p className="text-label-sm text-muted">Seeking works once the receiver reports a length.</p> : null}
    </div>
    <div className="flex items-center gap-2">
      <Button shape="circle" variant="primary" tooltip={playing ? "Pause" : "Play"} aria-label={playing ? "Pause" : "Play"} disabledReason={reason} onClick={() => void run(playing ? "Pause" : "Play", playing ? pause : play)} icon={playing ? <IconPause /> : <IconPlay />} />
      <Button shape="circle" variant="ghost" tooltip={session.isMuted ? "Unmute" : "Mute"} aria-label={session.isMuted ? "Unmute" : "Mute"} disabledReason={reason} onClick={() => void run(session.isMuted ? "Unmute" : "Mute", mute)} icon={session.isMuted ? <IconVolumeOff /> : <IconVolume />} />
      <Slider className="min-w-0 flex-1" aria-label="Receiver volume" min={0} max={1} step={0.01} format={(value) => `${Math.round(value * 100)}%`}
        value={volumeDraft ?? Math.max(0, Math.min(1, session.volume))}
        onValueChange={(value) => { if (!pending && typeof value === "number") setVolumeDraft(value); }}
        onValueCommitted={(value) => { if (!pending && typeof value === "number" && Number.isFinite(value)) void run("Set volume", async () => { try { await volume(value); } finally { setVolumeDraft(undefined); } }); }} />
      <Button variant="danger-soft" size="sm" tooltip="Stop casting" disabledReason={reason} onClick={() => void run("Stop casting", stop)}><IconStop /> Stop casting</Button>
    </div>
  </div>;
}
