import { useCallback, useEffect, useRef, useState, type RefObject } from "react";

import { resolvePlaybackSource, type PlaybackSource } from "@/lib/api/urls";

type HlsModule = typeof import("hls.js");
type HlsInstance = import("hls.js").default;

export interface MediaSourceOptions {
  audioStreamIndex?: number;
  retryKey?: number;
  /** A new playback activation restarts even when it uses the same file. */
  activationKey?: string | number;
}

export interface MediaSourceState {
  source: PlaybackSource | null;
  loading: boolean;
  error: string | null;
  /** Full file duration; growing HLS playlists under-report it. */
  duration: number | undefined;
  hls: HlsInstance | null;
  retry: () => void;
  /** Synchronous guard for pause/time events emitted by source teardown and loading. */
  isTransitioning: () => boolean;
}

type SourceSnapshot = Omit<MediaSourceState, "retry" | "isTransitioning">;
const emptySource: SourceSnapshot = { source: null, loading: false, error: null, duration: undefined, hls: null };
const MAX_HLS_RECOVERIES = 2;

/** Attaches one source generation; late promises and media listeners belong only to it. */
export function useMediaSource(
  mediaFileId: string | null,
  element: RefObject<HTMLMediaElement | null>,
  startPosition?: number,
  options?: MediaSourceOptions,
): MediaSourceState {
  const [state, setState] = useState<SourceSnapshot>(emptySource);
  const [attempt, setAttempt] = useState(0);
  const startRef = useRef(startPosition);
  startRef.current = startPosition;
  const transition = useRef(true);
  const previous = useRef<{ fileId: string; activationKey?: string | number; position: number } | null>(null);
  const { audioStreamIndex, retryKey, activationKey } = options ?? {};
  // Used as well as effect cancellation: a render for another item must never expose
  // the previous item's source or let its event handlers write the new transport.
  const key = JSON.stringify([mediaFileId, activationKey, audioStreamIndex, retryKey, attempt]);
  const currentKey = useRef(key);
  currentKey.current = key;
  const attachedKey = useRef<string | null>(null);
  const retry = useCallback(() => {
    transition.current = true;
    setAttempt((value) => value + 1);
  }, []);
  const isTransitioning = useCallback(() => transition.current || attachedKey.current !== currentKey.current, []);

  useEffect(() => {
    const media = element.current;
    transition.current = true;
    attachedKey.current = key;
    if (!mediaFileId || !media) {
      previous.current = null;
      setState(emptySource);
      return;
    }
    const controller = new AbortController();
    let cancelled = false;
    let terminal = false;
    let hls: HlsInstance | null = null;
    let recoveries = 0;
    const saved = previous.current;
    const start = saved?.fileId === mediaFileId && saved.activationKey === activationKey ? saved.position : startRef.current;
    const position = typeof start === "number" && Number.isFinite(start) && start > 0 ? start : 0;
    let lastPosition = position;
    let metadataReady = false;
    const rememberPosition = () => {
      if (!terminal && metadataReady && Number.isFinite(media.currentTime)) lastPosition = media.currentTime;
    };
    const active = () => !cancelled && currentKey.current === key;
    setState({ ...emptySource, loading: true });

    const fail = (message: string) => {
      if (!active() || terminal) return;
      rememberPosition();
      terminal = true;
      // destroy() can call media.load(), emitting pause/timeupdate at zero. Keep
      // those events suppressed and the last real position frozen until Retry.
      transition.current = true;
      hls?.destroy();
      hls = null;
      setState((snapshot) => ({ ...snapshot, error: message, loading: false, hls: null }));
    };
    const applyStart = () => {
      if (!active() || terminal) return;
      try {
        media.currentTime = position;
      } catch {
        fail("Playback could not resume at the saved position. Retry playback.");
        return;
      }
      metadataReady = true;
      lastPosition = position;
      transition.current = false;
      setState((snapshot) => ({ ...snapshot, loading: false }));
    };
    const ready = () => {
      if (!active() || terminal) return;
      transition.current = false;
      setState((snapshot) => snapshot.loading ? { ...snapshot, loading: false } : snapshot);
    };
    const onError = () => {
      fail("This media could not be played. Retry playback.");
    };
    media.addEventListener("loadedmetadata", applyStart, { once: true });
    media.addEventListener("canplay", ready);
    media.addEventListener("playing", ready);
    media.addEventListener("error", onError);
    media.addEventListener("timeupdate", rememberPosition);

    void (async () => {
      try {
        const source = await resolvePlaybackSource(mediaFileId, controller.signal, { audioStreamIndex });
        if (!active() || terminal) return;
        if (!source.isHls) {
          setState({ source, loading: true, error: null, duration: source.duration, hls: null });
          media.src = source.url;
          return;
        }

        // Native HLS does not need the lazy hls.js bundle.
        if (media.canPlayType("application/vnd.apple.mpegurl")) {
          setState({ source, loading: true, error: null, duration: source.duration, hls: null });
          media.src = source.url;
          return;
        }
        const { default: Hls }: HlsModule = await import("hls.js");
        if (!active() || terminal) return;
        if (!Hls.isSupported()) {
          fail("This browser cannot play streamed media.");
          return;
        }
        const instance = new Hls({
          lowLatencyMode: false, backBufferLength: 90, maxBufferLength: 60,
          enableWorker: true, startPosition: position > 0 ? position : -1,
          // Bound hls.js's own request attempts as well as our fatal recoveries.
          manifestLoadingMaxRetry: 2, levelLoadingMaxRetry: 2, fragLoadingMaxRetry: 2,
        });
        hls = instance;
        instance.on(Hls.Events.ERROR, (_event, data) => {
          if (!active() || terminal || !data.fatal) return;
          if (recoveries < MAX_HLS_RECOVERIES) {
            recoveries += 1;
            if (data.type === Hls.ErrorTypes.NETWORK_ERROR) {
              instance.startLoad();
              return;
            }
            if (data.type === Hls.ErrorTypes.MEDIA_ERROR) {
              instance.recoverMediaError();
              return;
            }
          }
          fail("Playback failed. The server could not prepare this file. Retry playback.");
        });
        instance.on(Hls.Events.MEDIA_ATTACHED, () => {
          if (active() && !terminal) instance.loadSource(source.url);
        });
        setState({ source, loading: true, error: null, duration: source.duration, hls: instance });
        instance.attachMedia(media);
      } catch (error) {
        if (active() && !controller.signal.aborted) {
          fail(error instanceof Error ? error.message : "The media source could not be loaded. Retry playback.");
        }
      }
    })();

    return () => {
      cancelled = true;
      transition.current = true;
      if (!terminal) rememberPosition();
      previous.current = { fileId: mediaFileId, activationKey, position: lastPosition };
      controller.abort();
      media.removeEventListener("loadedmetadata", applyStart);
      media.removeEventListener("canplay", ready);
      media.removeEventListener("playing", ready);
      media.removeEventListener("error", onError);
      media.removeEventListener("timeupdate", rememberPosition);
      hls?.destroy();
      hls = null;
      media.removeAttribute("src");
      media.load();
    };
  }, [mediaFileId, element, activationKey, audioStreamIndex, retryKey, attempt, key]);

  const snapshot = attachedKey.current === key ? state : { ...emptySource, loading: Boolean(mediaFileId) };
  return { ...snapshot, retry, isTransitioning };
}
