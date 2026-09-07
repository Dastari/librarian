import { useEffect, useState, type RefObject } from "react";

/**
 * Refractive "liquid glass" backdrop filter.
 *
 * Builds an SVG displacement map sized to the element and applies it through
 * `backdrop-filter: url(...)`, bending whatever is behind the element's edges the way thick
 * glass does. Chromium supports `backdrop-filter: url()`; other engines fall back to the flat
 * blur recipes in styles/glass.css, so callers always keep a `glass` class as well.
 * Adapted from nikdelvin/liquid-glass.
 */

export const supportsRefraction = (() => {
  if (typeof document === "undefined") return false;
  const probe = document.createElement("div");
  probe.style.cssText = "backdrop-filter: url(#probe)";
  return probe.style.backdropFilter.includes("url");
})();

export interface RefractionOptions {
  /** Edge refraction depth in px. */
  depth?: number;
  /** Displacement strength; higher bends more. */
  strength?: number;
  /** RGB split amount for a prismatic edge. 0 disables. */
  chromaticAberration?: number;
  blur?: number;
  brightness?: number;
  saturate?: number;
}

function displacementMap(width: number, height: number, radius: number, depth: number): string {
  const svg = `<svg height="${height}" width="${width}" viewBox="0 0 ${width} ${height}" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="Y" x1="0" x2="0" y1="${Math.ceil((radius / height) * 15)}%" y2="${Math.floor(100 - (radius / height) * 15)}%">
      <stop offset="0%" stop-color="#0F0"/><stop offset="100%" stop-color="#000"/>
    </linearGradient>
    <linearGradient id="X" x1="${Math.ceil((radius / width) * 15)}%" x2="${Math.floor(100 - (radius / width) * 15)}%" y1="0" y2="0">
      <stop offset="0%" stop-color="#F00"/><stop offset="100%" stop-color="#000"/>
    </linearGradient>
  </defs>
  <rect width="${width}" height="${height}" fill="#808080"/>
  <g filter="blur(2px)">
    <rect width="${width}" height="${height}" fill="#000080"/>
    <rect width="${width}" height="${height}" fill="url(#Y)" style="mix-blend-mode:screen"/>
    <rect width="${width}" height="${height}" fill="url(#X)" style="mix-blend-mode:screen"/>
    <rect x="${depth}" y="${depth}" width="${Math.max(1, width - 2 * depth)}" height="${Math.max(1, height - 2 * depth)}" fill="#808080" rx="${radius}" ry="${radius}" filter="blur(${depth}px)"/>
  </g>
</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

export function refractionFilterUrl(width: number, height: number, radius: number, depth: number, strength: number, aberration: number): string {
  const map = displacementMap(width, height, radius, depth);
  const channel = (scale: number, matrix: string, result: string) =>
    `<feDisplacementMap in="SourceGraphic" in2="map" scale="${scale}" xChannelSelector="R" yChannelSelector="G"/><feColorMatrix type="matrix" values="${matrix}" result="${result}"/>`;
  const svg = `<svg height="${height}" width="${width}" viewBox="0 0 ${width} ${height}" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <filter id="displace" color-interpolation-filters="sRGB">
      <feImage x="0" y="0" width="${width}" height="${height}" href="${map}" result="map"/>
      ${channel(strength + aberration * 2, "1 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 1 0", "r")}
      ${channel(strength + aberration, "0 0 0 0 0  0 1 0 0 0  0 0 0 0 0  0 0 0 1 0", "g")}
      ${channel(strength, "0 0 0 0 0  0 0 0 0 0  0 0 1 0 0  0 0 0 1 0", "b")}
      <feBlend in="r" in2="g" mode="screen"/>
      <feBlend in2="b" mode="screen"/>
    </filter>
  </defs>
</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}#displace`;
}

/**
 * Returns a `backdrop-filter` value for the element in `ref`, regenerated on resize, or
 * `undefined` where refraction is unsupported (the CSS glass fallback then applies).
 */
export function useRefraction(ref: RefObject<HTMLElement | null>, { depth = 8, strength = 60, chromaticAberration = 0, blur = 6, brightness = 1.08, saturate = 1.4 }: RefractionOptions = {}, enabled = true): string | undefined {
  const [filter, setFilter] = useState<string | undefined>(undefined);
  useEffect(() => {
    const element = ref.current;
    if (!supportsRefraction || !enabled || !element) {
      setFilter(undefined);
      return;
    }
    let frame = 0;
    const redraw = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const rect = element.getBoundingClientRect();
        const width = Math.max(1, Math.round(rect.width));
        const height = Math.max(1, Math.round(rect.height));
        const radius = parseFloat(getComputedStyle(element).borderTopLeftRadius) || 0;
        const url = refractionFilterUrl(width, height, Math.min(radius, height / 2), depth, strength, chromaticAberration);
        setFilter(`blur(${blur / 2}px) url('${url}') blur(${blur}px) brightness(${brightness}) saturate(${saturate})`);
      });
    };
    redraw();
    const observer = new ResizeObserver(redraw);
    observer.observe(element);
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [ref, enabled, depth, strength, chromaticAberration, blur, brightness, saturate]);
  return filter;
}
