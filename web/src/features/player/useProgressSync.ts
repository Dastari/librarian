import { useApolloClient, useQuery } from "@apollo/client/react";
import { useCallback, useEffect, useRef } from "react";

import {
  EntityPlaybackProgressCreateDocument,
  EntityPlaybackProgressUpdateDocument,
  EntityPlaybackSessionCreateDocument,
  EntityPlaybackSessionUpdateDocument,
  PlaybackProgressForFileDocument,
  PlaybackSyncIntervalDocument,
} from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";

import type { PlayItem } from "./store";

const WATCHED_THRESHOLD = 0.92;

/**
 * Persists playback position on an interval (server setting `playback_sync_interval`), on
 * pause and on unmount, and keeps a PlaybackSession row alive so other clients can see what is
 * playing. Returns the resume position for the current file.
 */
export function useProgressSync(item: PlayItem | null) {
  const { user } = useSession();
  const client = useApolloClient();
  const mediaFileId = item?.mediaFileId ?? null;

  const existing = useQuery(PlaybackProgressForFileDocument, {
    variables: { userId: user?.id ?? "", mediaFileId: mediaFileId ?? "" },
    skip: !user || !mediaFileId,
    fetchPolicy: "network-only",
  });
  const interval = useQuery(PlaybackSyncIntervalDocument);
  const intervalMs = Math.max(5, Number(interval.data?.appSettings.edges[0]?.node.value ?? 15) || 15) * 1000;

  const progressIdRef = useRef<string | null>(null);
  const sessionIdRef = useRef<string | null>(null);
  const latest = useRef<{ position: number; duration: number; playing: boolean }>({ position: 0, duration: 0, playing: false });
  const lastSaved = useRef(0);

  useEffect(() => {
    progressIdRef.current = existing.data?.playbackProgresses.edges[0]?.node.id ?? null;
  }, [existing.data]);

  const resumePosition = (() => {
    const row = existing.data?.playbackProgresses.edges[0]?.node;
    if (!row || row.isWatched) return item?.startPosition;
    if (item?.startPosition !== undefined) return item.startPosition;
    return row.currentPosition > 5 ? row.currentPosition : undefined;
  })();

  const flush = useCallback(
    async (force = false) => {
      if (!user || !mediaFileId) return;
      const { position, duration } = latest.current;
      if (!force && Date.now() - lastSaved.current < intervalMs) return;
      if (!Number.isFinite(position) || position <= 0) return;
      lastSaved.current = Date.now();
      const percent = duration > 0 ? Math.min(100, (position / duration) * 100) : 0;
      const isWatched = duration > 0 && position / duration >= WATCHED_THRESHOLD;
      const now = new Date().toISOString();
      try {
        if (progressIdRef.current) {
          await client.mutate({
            mutation: EntityPlaybackProgressUpdateDocument,
            variables: { id: progressIdRef.current, input: { currentPosition: position, duration: duration || null, progressPercent: percent, isWatched, watchedAt: isWatched ? now : null } },
          });
        } else {
          const { data } = await client.mutate({
            mutation: EntityPlaybackProgressCreateDocument,
            variables: { input: { userId: user.id, mediaFileId, currentPosition: position, duration: duration || null, progressPercent: percent, isWatched, watchedAt: isWatched ? now : null } },
          });
          progressIdRef.current = data?.createPlaybackProgress.playbackProgress?.id ?? null;
        }
        if (sessionIdRef.current) {
          await client.mutate({
            mutation: EntityPlaybackSessionUpdateDocument,
            variables: { id: sessionIdRef.current, input: { currentPosition: position, duration: duration || null, isPlaying: latest.current.playing, lastUpdatedAt: now, completedAt: isWatched ? now : null } },
          });
        }
      } catch {
        // Progress is best-effort; the next tick retries.
      }
    },
    [client, intervalMs, mediaFileId, user],
  );

  // Open a session row when playback starts, close it on unmount.
  useEffect(() => {
    if (!user || !item) return;
    const now = new Date().toISOString();
    const entityField: Record<string, string> =
      item.entity.kind === "movie" ? { movieId: item.entity.id } : item.entity.kind === "episode" ? { episodeId: item.entity.id } : item.entity.kind === "track" ? { trackId: item.entity.id } : { audiobookId: item.entity.id };
    let cancelled = false;
    void client
      .mutate({
        mutation: EntityPlaybackSessionCreateDocument,
        variables: {
          input: { userId: user.id, mediaFileId: item.mediaFileId, contentType: item.entity.kind, ...entityField, currentPosition: item.startPosition ?? 0, volume: 1, isMuted: false, isPlaying: true, startedAt: now, lastUpdatedAt: now },
        },
      })
      .then(({ data }) => {
        if (!cancelled) sessionIdRef.current = data?.createPlaybackSession.playbackSession?.id ?? null;
      })
      .catch(() => undefined);

    const timer = setInterval(() => void flush(), 5_000);
    const onHide = () => {
      if (document.visibilityState === "hidden") void flush(true);
    };
    document.addEventListener("visibilitychange", onHide);
    window.addEventListener("pagehide", onHide);
    return () => {
      cancelled = true;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onHide);
      window.removeEventListener("pagehide", onHide);
      void flush(true).then(() => {
        if (sessionIdRef.current) {
          void client.mutate({ mutation: EntityPlaybackSessionUpdateDocument, variables: { id: sessionIdRef.current, input: { isPlaying: false, lastUpdatedAt: new Date().toISOString() } } }).catch(() => undefined);
          sessionIdRef.current = null;
        }
      });
    };
  }, [client, flush, item, user]);

  const report = useCallback((position: number, duration: number, playing: boolean) => {
    latest.current = { position, duration, playing };
  }, []);

  return { resumePosition, report, flush, loading: existing.loading && !existing.data };
}
