import { useEffect, useRef, useState, type RefObject } from "react";

import { resolvePlaybackSource, type PlaybackSource } from "@/lib/api/urls";

type HlsModule = typeof import("hls.js");
type HlsInstance = import("hls.js").default;

export interface MediaSourceState {
  source: PlaybackSource | null;
  loading: boolean;
  error: string | null;
  /** Full file duration; growing HLS playlists under-report it. */
  duration: number | undefined;
  hls: HlsInstance | null;
}

/**
 * Attaches a media file to a <video>/<audio> element. Asks the backend whether the file needs
 * HLS; uses hls.js where MSE is available and the element's native HLS support otherwise.
 * hls.js is loaded lazily so direct-play sessions never download it.
 */
export function useMediaSource(mediaFileId: string | null, element: RefObject<HTMLMediaElement | null>, startPosition?: number): MediaSourceState {
  const [state, setState] = useState<MediaSourceState>({ source: null, loading: Boolean(mediaFileId), error: null, duration: undefined, hls: null });
  const hlsRef = useRef<HlsInstance | null>(null);
  const startRef = useRef(startPosition);
  startRef.current = startPosition;

  useEffect(() => {
    const media = element.current;
    if (!mediaFileId || !media) return;
    const controller = new AbortController();
    let cancelled = false;

    setState({ source: null, loading: true, error: null, duration: undefined, hls: null });

    const applyStart = () => {
      const start = startRef.current;
      if (start && start > 0 && Number.isFinite(start)) media.currentTime = start;
    };

    void (async () => {
      const source = await resolvePlaybackSource(mediaFileId, controller.signal);
      if (cancelled) return;

      if (!source.isHls) {
        media.src = source.url;
        media.addEventListener("loadedmetadata", applyStart, { once: true });
        setState({ source, loading: false, error: null, duration: source.duration, hls: null });
        return;
      }

      const nativeHls = media.canPlayType("application/vnd.apple.mpegurl") !== "";
      const { default: Hls }: HlsModule = await import("hls.js");
      if (cancelled) return;

      if (Hls.isSupported()) {
        const hls = new Hls({
          lowLatencyMode: false,
          backBufferLength: 90,
          maxBufferLength: 60,
          enableWorker: true,
          startPosition: startRef.current && startRef.current > 0 ? startRef.current : -1,
        });
        hlsRef.current = hls;
        hls.on(Hls.Events.ERROR, (_event, data) => {
          if (!data.fatal) return;
          if (data.type === Hls.ErrorTypes.NETWORK_ERROR) {
            hls.startLoad();
            return;
          }
          if (data.type === Hls.ErrorTypes.MEDIA_ERROR) {
            hls.recoverMediaError();
            return;
          }
          setState((previous) => ({ ...previous, error: "Playback failed. The server could not prepare this file.", loading: false }));
        });
        hls.attachMedia(media);
        hls.on(Hls.Events.MEDIA_ATTACHED, () => hls.loadSource(source.url));
        setState({ source, loading: false, error: null, duration: source.duration, hls });
      } else if (nativeHls) {
        media.src = source.url;
        media.addEventListener("loadedmetadata", applyStart, { once: true });
        setState({ source, loading: false, error: null, duration: source.duration, hls: null });
      } else {
        setState({ source, loading: false, error: "This browser cannot play streamed video.", duration: source.duration, hls: null });
      }
    })();

    return () => {
      cancelled = true;
      controller.abort();
      hlsRef.current?.destroy();
      hlsRef.current = null;
      media.removeAttribute("src");
      media.load();
    };
  }, [mediaFileId, element]);

  return state;
}
