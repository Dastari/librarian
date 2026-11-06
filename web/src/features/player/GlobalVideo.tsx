import { useNavigate, useRouterState } from "@tanstack/react-router";
import { useCallback, useEffect, useRef } from "react";

import { useSession } from "@/lib/auth/useSession";

import { playerStore, usePlayerState } from "./store";
import { clearVideoSession, readVideoSession } from "./videoSession";
import { WatchPage } from "./WatchPage";

const WATCH_PATH = /^\/watch\/([^/]+)/;

/**
 * The one video player, above the routes so nothing a route does can unmount it. On /watch it fills
 * the screen. Made small, it floats in a corner while you carry on using the app, and a refresh
 * brings it back. Leaving /watch any other way stops it.
 */
export function GlobalVideo() {
  const navigate = useNavigate();
  const { user, status } = useSession();
  const userId = user?.id;
  const state = usePlayerState();
  const pathname = useRouterState({ select: (router) => router.location.pathname });
  const routeFileId = WATCH_PATH.exec(pathname)?.[1] ? decodeURIComponent(WATCH_PATH.exec(pathname)![1]!) : null;
  const mini = state.videoMini && !routeFileId;

  // Pick a mini player back up after a refresh.
  const restored = useRef(false);
  useEffect(() => {
    if (restored.current || status !== "authenticated" || !userId) return;
    restored.current = true;
    if (routeFileId || playerStore.getSnapshot().video) return;
    const session = readVideoSession(userId);
    if (!session) return;
    playerStore.setVideo({ ...session.item, startPosition: session.position, startPaused: !session.playing });
    playerStore.setVideoMini(true);
  }, [routeFileId, status, userId]);

  // On /watch it is the full player again. Off it, without having been made small, it has stopped.
  useEffect(() => {
    if (routeFileId) playerStore.setVideoMini(false);
    else if (!playerStore.getSnapshot().videoMini) playerStore.setVideo(null);
  }, [routeFileId]);
  // A full player leaves nothing to come back to.
  useEffect(() => {
    if (userId && !state.videoMini && restored.current) clearVideoSession(userId);
  }, [state.videoMini, userId]);

  const goBack = useCallback(() => {
    if (window.history.length > 1) window.history.back();
    else void navigate({ to: playerStore.getSnapshot().video?.href ?? "/" });
  }, [navigate]);
  const minimise = useCallback(() => {
    playerStore.setVideoMini(true);
    goBack();
  }, [goBack]);
  const expand = useCallback(() => {
    const video = playerStore.getSnapshot().video;
    if (video) void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: video.mediaFileId } });
  }, [navigate]);
  const close = useCallback(() => {
    if (userId) clearVideoSession(userId);
    playerStore.setVideoMini(false);
    playerStore.setVideo(null);
  }, [userId]);

  const mediaFileId = routeFileId ?? (mini ? state.video?.mediaFileId : null);
  if (status !== "authenticated" || !mediaFileId) return null;
  return (
    <div className={mini ? "video-host-mini" : "video-host-full"}>
      <WatchPage mediaFileId={mediaFileId} mini={mini} userId={userId} onBack={goBack} onMinimise={minimise} onExpand={expand} onClose={close} />
    </div>
  );
}
