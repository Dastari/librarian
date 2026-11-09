import { useEffect, useState } from "react";

import { fillTransform, fillTransformCss, type PictureCropInfo } from "./videoPreviews";

/**
 * Zooms the video element past baked-in black bars (design.md Q123) whenever its box is wider
 * (letterbox) or taller (pillarbox) than the picture, and follows the box as the window, view
 * mode or rendition changes. Only CSS is touched: the element keeps `object-fit: contain` and
 * gains a transform, and Glass's frame clips the overflow. Pass `null` to turn it off.
 * Returns the zoom in use (1 when none).
 */
export function usePictureFill(video: HTMLMediaElement | null, crop: PictureCropInfo | null): number {
  const [zoom, setZoom] = useState(1);
  useEffect(() => {
    if (!(video instanceof HTMLVideoElement) || !crop) {
      setZoom(1);
      return;
    }
    const apply = () => {
      const transform = fillTransform(video.videoWidth, video.videoHeight, crop, video.clientWidth, video.clientHeight);
      video.style.transform = fillTransformCss(transform);
      setZoom(transform.scale);
    };
    video.style.transformOrigin = "center";
    video.style.transition = "transform var(--duration-base, 200ms) ease";
    apply();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(apply);
    observer?.observe(video);
    // A new source, or an HLS rendition of another size, changes the decoded size.
    video.addEventListener("loadedmetadata", apply);
    video.addEventListener("resize", apply);
    window.addEventListener("resize", apply);
    return () => {
      observer?.disconnect();
      video.removeEventListener("loadedmetadata", apply);
      video.removeEventListener("resize", apply);
      window.removeEventListener("resize", apply);
      video.style.transform = "";
      video.style.transition = "";
      video.style.transformOrigin = "";
    };
  }, [video, crop]);
  return zoom;
}
