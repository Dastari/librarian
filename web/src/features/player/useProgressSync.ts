import type { ApolloClient } from "@apollo/client";
import { useApolloClient } from "@apollo/client/react";
import { useLiveEpoch, useLiveInvalidation, useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  EntityPlaybackProgressCreateDocument,
  EntityPlaybackProgressUpdateDocument,
  EntityPlaybackSessionCreateDocument,
  EntityPlaybackSessionUpdateDocument,
  PlaybackProgressForFileDocument,
  PlaybackSyncIntervalDocument,
  type PlaybackProgressFieldsFragment,
} from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { session } from "@/lib/auth/session";
import { assertSuccess } from "@/lib/graphql/errors";

import type { PlayItem } from "./store";

const WATCHED_THRESHOLD = 0.92;
type Checkpoint = { position: number; duration: number; playing: boolean };
type Activation = {
  order: number;
  latest: Checkpoint;
  reported: boolean;
  closed: boolean;
  sessionId: string | null;
  sessionRequested: boolean;
  lastSaved: number;
};
type FileLane = {
  progressId: string | null;
  tail: Promise<void>;
  mounts: number;
  pending: number;
  nextActivation: number;
  latestReported: number;
};
type Authority = { key: string; files: Map<string, FileLane> };

// Share only active/in-flight work, including across keyed player remounts. A weak
// client key and idle eviction avoid retaining a history of private file IDs.
let authorities = new WeakMap<ApolloClient, Authority>();
session.onChange(() => { authorities = new WeakMap(); });

function acquireLane(client: ApolloClient, key: string, fileId: string) {
  let owner = authorities.get(client);
  if (owner?.key !== key) {
    owner = { key, files: new Map() };
    authorities.set(client, owner);
  }
  const ownedAuthority = owner;
  let lane = ownedAuthority.files.get(fileId);
  if (!lane) {
    lane = { progressId: null, tail: Promise.resolve(), mounts: 0, pending: 0, nextActivation: 0, latestReported: 0 };
    ownedAuthority.files.set(fileId, lane);
  }
  lane.mounts += 1;
  const ownedLane = lane;
  const evict = () => {
    if (!ownedLane.mounts && !ownedLane.pending && ownedAuthority.files.get(fileId) === ownedLane) ownedAuthority.files.delete(fileId);
  };
  return {
    lane,
    current: () => authorities.get(client) === ownedAuthority,
    enqueue: (work: () => Promise<void>) => {
      ownedLane.pending += 1;
      const pending = ownedLane.tail.then(work).catch(() => undefined).finally(() => {
        ownedLane.pending -= 1;
        evict();
      });
      ownedLane.tail = pending;
      return pending;
    },
    release: () => { ownedLane.mounts -= 1; evict(); },
  };
}
type LaneLease = ReturnType<typeof acquireLane>;

/** Checkpoint reads and writes share a file lane; sessions stay local to each activation. */
export function useProgressSync(item: PlayItem | null, activationKey?: string | number) {
  const { user } = useSession();
  const client = useApolloClient();
  const epoch = useLiveEpoch();
  const userId = user?.id ?? null;
  const mediaFileId = item?.mediaFileId ?? null;
  const sessionEpoch = session.getEpoch();
  const principal = useMemo(() => ({ client, userId, epoch, sessionEpoch }), [client, userId, epoch, sessionEpoch]);
  const authority = useRef(principal);
  authority.current = principal;
  const kind = item?.entity.kind;
  const entityId = item?.entity.id;
  const audiobookId = item?.audiobookId;
  const scope = useMemo(() => ({ lease: null as LaneLease | null, readReady: false, activation: null as Activation | null }), [principal, mediaFileId, activationKey, kind, entityId, audiobookId]);
  const [read, setRead] = useState<{ scope: typeof scope; row?: PlaybackProgressFieldsFragment; error?: boolean }>();

  const interval = useQuery(PlaybackSyncIntervalDocument);
  const configuredInterval = Number(interval.data?.appSettings.edges[0]?.node.value ?? 15);
  const intervalMs = useRef(15_000);
  intervalMs.current = Math.max(5, Number.isFinite(configuredInterval) && configuredInterval > 0 ? configuredInterval : 15) * 1000;

  const canWrite = useCallback(() => authority.current === principal && session.getEpoch() === principal.sessionEpoch && scope.lease?.current() === true, [principal, scope]);
  const persist = useCallback((activation: Activation, snapshot: Checkpoint, force: boolean, closing = false): Promise<void> => {
    const lease = scope.lease;
    if (!userId || !mediaFileId || !lease || !activation.reported) return Promise.resolve();
    const fileLane = lease.lane;
    return lease.enqueue(async () => {
      if (!canWrite() || !scope.readReady) return;
      if (!force && Date.now() - activation.lastSaved < intervalMs.current) return;
      const { position, duration, playing } = snapshot;
      const percent = duration > 0 ? Math.min(100, (position / duration) * 100) : 0;
      const isWatched = duration > 0 && position / duration >= WATCHED_THRESHOLD;
      const now = new Date().toISOString();
      let saved = true;
      try {
        // A late old cleanup can close its session, but cannot rewind a newer
        // activation's checkpoint, even when both activations share one file.
        if (position > 0 && activation.order >= fileLane.latestReported) {
          const input = { currentPosition: position, duration: duration || null, progressPercent: percent, isWatched, watchedAt: isWatched ? now : null };
          if (fileLane.progressId) {
            const result = await client.mutate({ mutation: EntityPlaybackProgressUpdateDocument, variables: { id: fileLane.progressId, input } });
            assertSuccess(result.data?.updatePlaybackProgress, "Could not save playback progress");
          } else {
            const result = await client.mutate({ mutation: EntityPlaybackProgressCreateDocument, variables: { input: { userId, mediaFileId, ...input } } });
            const payload = assertSuccess(result.data?.createPlaybackProgress, "Could not save playback progress");
            if (!payload.playbackProgress?.id) throw new Error("Playback progress ID missing");
            if (!canWrite()) return;
            fileLane.progressId = payload.playbackProgress.id;
          }
        }
      } catch {
        saved = false;
      }
      // Progress failure must not prevent closing the old session. Authority
      // replacement invalidates both queued work and post-await continuations.
      if (!canWrite()) return;
      try {
        if (!activation.sessionId && !closing) {
          const entityField = kind === "movie" ? { movieId: entityId }
            : kind === "episode" ? { episodeId: entityId }
            : kind === "track" ? { trackId: entityId }
            : { audiobookId };
          const result = await client.mutate({
            mutation: EntityPlaybackSessionCreateDocument,
            variables: { input: { userId, mediaFileId, contentType: kind!, ...entityField, currentPosition: position, duration: duration || null, volume: 1, isMuted: false, isPlaying: !activation.closed && playing, startedAt: now, lastUpdatedAt: now } },
          });
          const payload = assertSuccess(result.data?.createPlaybackSession, "Could not start playback session");
          if (!payload.playbackSession?.id) throw new Error("Playback session ID missing");
          if (!canWrite()) return;
          activation.sessionId = payload.playbackSession.id;
        } else if (activation.sessionId) {
          const result = await client.mutate({
            mutation: EntityPlaybackSessionUpdateDocument,
            variables: { id: activation.sessionId, input: { currentPosition: position, duration: duration || null, isPlaying: closing ? false : playing, lastUpdatedAt: now, completedAt: isWatched ? now : null } },
          });
          assertSuccess(result.data?.updatePlaybackSession, "Could not update playback session");
        }
      } catch {
        saved = false;
        activation.sessionRequested = false;
      }
      if (saved) activation.lastSaved = Date.now();
    });
  }, [client, userId, mediaFileId, scope, canWrite, kind, entityId, audiobookId]);

  const refresh = useCallback(() => {
    const lease = scope.lease;
    const activation = scope.activation;
    if (!lease || !activation || !userId || !mediaFileId) return Promise.resolve();
    return lease.enqueue(async () => {
      if (!canWrite() || activation.closed) return;
      try {
        // This request starts AFTER old final checkpoints finish. A network-only
        // hook read started on mount could otherwise return pre-cleanup data, or
        // deduplicate against a stale request from the previous same-file mount.
        const result = await client.query({ query: PlaybackProgressForFileDocument,
          variables: { userId, mediaFileId }, fetchPolicy: "no-cache", context: { queryDeduplication: false } });
        if (!canWrite() || activation.closed) return;
        const rows = result.data?.playbackProgresses.edges;
        const row = rows?.find(({ node }) => node.userId === userId && node.mediaFileId === mediaFileId)?.node;
        if (!rows || (!row && rows.length > 0)) throw new Error("Playback checkpoint scope mismatch");
        lease.lane.progressId = row?.id ?? lease.lane.progressId;
        scope.readReady = true;
        setRead({ scope, row });
        if (activation.reported && activation.latest.playing && !activation.sessionRequested) {
          activation.sessionRequested = true;
          void persist(activation, { ...activation.latest }, true);
        }
      } catch {
        if (canWrite() && !activation.closed) setRead({ scope, error: true });
      }
    });
  }, [scope, userId, mediaFileId, client, canWrite, persist]);
  useLiveInvalidation(PlaybackProgressForFileDocument, { userId: userId ?? "", mediaFileId: mediaFileId ?? "" }, refresh, { enabled: Boolean(userId && mediaFileId), shared: false });

  const flush = useCallback((force = false) => {
    const activation = scope.activation;
    if (!activation || activation.closed) return Promise.resolve();
    return persist(activation, { ...activation.latest }, force);
  }, [scope, persist]);

  useEffect(() => {
    if (!userId || !mediaFileId) return;
    const lease = acquireLane(client, JSON.stringify([epoch, sessionEpoch, userId]), mediaFileId);
    scope.lease = lease;
    scope.readReady = false;
    const activation: Activation = {
      order: ++lease.lane.nextActivation,
      latest: { position: 0, duration: 0, playing: false }, reported: false, closed: false,
      sessionId: null, sessionRequested: false, lastSaved: -Infinity,
    };
    scope.activation = activation;
    void refresh();
    const timer = setInterval(() => void flush(), 5_000);
    const onHide = () => {
      if (document.visibilityState === "hidden") void flush(true);
    };
    const onPageHide = () => void flush(true);
    document.addEventListener("visibilitychange", onHide);
    window.addEventListener("pagehide", onPageHide);
    return () => {
      activation.closed = true;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onHide);
      window.removeEventListener("pagehide", onPageHide);
      void persist(activation, { ...activation.latest, playing: false }, true, true);
      lease.release();
    };
  }, [client, epoch, sessionEpoch, userId, mediaFileId, scope, refresh, persist, flush]);

  const report = useCallback((position: number, duration: number, playing: boolean) => {
    const activation = scope.activation;
    if (!activation || activation.closed || !canWrite()) return;
    if (!Number.isFinite(position) || position < 0) return;
    activation.latest = { position, duration: Number.isFinite(duration) && duration > 0 ? duration : 0, playing };
    activation.reported = true;
    scope.lease!.lane.latestReported = Math.max(scope.lease!.lane.latestReported, activation.order);
    if (playing && !activation.sessionRequested && scope.readReady) {
      activation.sessionRequested = true;
      void persist(activation, { ...activation.latest }, true);
    }
  }, [scope, canWrite, persist]);

  const row = read?.scope === scope ? read.row : undefined;
  const resumePosition = item?.startPosition !== undefined ? item.startPosition
    : row && !row.isWatched && row.currentPosition > 5 ? row.currentPosition : undefined;
  return { resumePosition, report, flush, loading: Boolean(userId && mediaFileId && !scope.readReady && !(read?.scope === scope && read.error)) };
}
