import { useEffect, useRef } from "react";
import type { RefObject } from "react";
import Hls from "hls.js";

/**
 * Attach `src` to a `<video>`/`<audio>` element, transparently handling HLS
 * (`.m3u8`) playlists via hls.js (or native Safari HLS where available) and
 * plain direct-play URLs otherwise.
 *
 * Extracted from `VideoPlayer.tsx` so the persistent video/audio players
 * (`PersistentPlayer.tsx`, `PersistentAudioPlayer.tsx`) can reuse the exact
 * same HLS-attach logic instead of assigning `src` directly, which would
 * silently fail to play `.m3u8` URLs in any non-Safari browser (see
 * `docs/tier1-features-plan.md` §4 / `docs/design.md`'s HLS decision Q entry).
 *
 * Deliberately imperative (no JSX `src` prop on the media element) so hls.js
 * fully owns the element's `src` for HLS playback; mixing a React-controlled
 * `src` prop with hls.js's own `MediaSource` blob URL assignment risks a
 * spurious native decode error firing before hls.js attaches.
 */
export function useHlsMediaSource(
  mediaRef: RefObject<HTMLMediaElement | null>,
  src: string | undefined,
  onError?: (error: Error) => void,
) {
  const errorRef = useRef(onError);
  useEffect(() => {
    errorRef.current = onError;
  }, [onError]);

  useEffect(() => {
    const media = mediaRef.current;
    if (!media || !src) return;

    if (src.includes(".m3u8")) {
      if (Hls.isSupported()) {
        const hls = new Hls({
          enableWorker: true,
          lowLatencyMode: false,
          startPosition: 0,
        });

        hls.loadSource(src);
        hls.attachMedia(media);

        hls.on(Hls.Events.ERROR, (_event, data) => {
          if (data.fatal) {
            errorRef.current?.(new Error(`HLS fatal error: ${data.type}`));
          }
        });

        return () => {
          hls.destroy();
        };
      } else if (media.canPlayType("application/vnd.apple.mpegurl")) {
        // Native HLS support (Safari)
        media.src = src;
        return () => {
          media.pause();
          media.src = "";
          media.load();
        };
      } else {
        errorRef.current?.(new Error("HLS not supported"));
        return;
      }
    } else {
      // Direct play
      media.src = src;
      return () => {
        media.pause();
        media.src = "";
        media.load();
      };
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [src, mediaRef]);
}
