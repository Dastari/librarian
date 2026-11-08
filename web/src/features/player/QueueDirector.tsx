import { useApolloClient } from "@apollo/client/react";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect, useRef } from "react";

import { session } from "@/lib/auth/session";
import { useSession } from "@/lib/auth/useSession";

import { continueAfter } from "./continuation";
import { currentQueueItem, isVideoItem, playerStore, usePlayerState } from "./store";
import { readVideoSession } from "./videoSession";

const WATCH_PATH = /^\/watch\/([^/]+)/;

/** The media file the address is playing in the video player, if any. */
export const watchingFile = (pathname: string): string | null => {
  const match = WATCH_PATH.exec(pathname);
  return match ? decodeURIComponent(match[1]!) : null;
};

/**
 * Renders nothing; it is the queue's links to the rest of the app, kept in one place:
 *
 * - it gives the store its continuation resolver, so a queue that runs out finds what follows;
 * - it brings back this person's saved queue after a reload;
 * - it moves between the audio dock and the video player: when a different entry is selected
 *   and it is video, the app goes to /watch for it (or lets the mini player carry on), and when
 *   it is audio while /watch is open, the app leaves /watch and the dock plays it.
 */
export function QueueDirector() {
  const client = useApolloClient();
  const router = useRouter();
  const navigate = useNavigate();
  const { user, status } = useSession();
  const userId = user?.id;
  const state = usePlayerState();

  useEffect(() => {
    playerStore.setContinuationResolver((last) => continueAfter(client, last));
    return () => playerStore.setContinuationResolver(null);
  }, [client]);

  const restored = useRef<string | null>(null);
  const epoch = session.getEpoch();
  useEffect(() => {
    if (status !== "authenticated" || !userId) return;
    const key = `${epoch}:${userId}`;
    if (restored.current === key) return;
    restored.current = key;
    playerStore.restore(userId, { keepVideo: readVideoSession(userId)?.item.mediaFileId ?? null });
  }, [status, userId, epoch]);

  // Only a new selection moves the app between players; the queue merely changing does not.
  const selected = useRef(state.audioActivation);
  useEffect(() => {
    if (selected.current === state.audioActivation) return;
    selected.current = state.audioActivation;
    const snapshot = playerStore.getSnapshot();
    const item = currentQueueItem(snapshot);
    if (!item) return;
    const watching = watchingFile(router.history.location.pathname);
    if (isVideoItem(item)) {
      // A floating video window just plays the new entry where it is.
      if (watching === item.mediaFileId || (snapshot.videoMini && !watching)) return;
      void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: item.mediaFileId }, replace: watching !== null });
    } else if (watching) {
      if (window.history.length > 1) window.history.back();
      else void navigate({ to: item.href ?? "/" });
    }
  }, [state.audioActivation, router, navigate]);

  return null;
}
