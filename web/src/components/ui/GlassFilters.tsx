import { useEffect } from "react";

/**
 * SVG filters for liquid glass, mounted once. `#liquid-refraction` warps the backdrop of large
 * surfaces with fractal noise; `#liquid-refraction-fine` is a gentler version for controls so
 * text behind small elements stays legible. `data-refraction` on <html> switches the CSS to the
 * `url()` stack only where the browser supports it.
 */
export function GlassFilters() {
  useEffect(() => {
    const probe = document.createElement("div");
    probe.style.cssText = "backdrop-filter: url(#probe)";
    document.documentElement.dataset.refraction = probe.style.backdropFilter.includes("url") ? "true" : "false";
  }, []);

  return (
    <svg aria-hidden className="pointer-events-none absolute size-0 overflow-hidden">
      <filter id="liquid-refraction" x="-5%" y="-5%" width="110%" height="110%" colorInterpolationFilters="sRGB">
        <feTurbulence type="fractalNoise" baseFrequency="0.012 0.018" numOctaves={2} seed={7} result="noise" />
        <feGaussianBlur in="noise" stdDeviation="1.5" result="soft" />
        <feDisplacementMap in="SourceGraphic" in2="soft" scale={14} xChannelSelector="R" yChannelSelector="G" />
      </filter>
      <filter id="liquid-refraction-fine" x="-5%" y="-5%" width="110%" height="110%" colorInterpolationFilters="sRGB">
        <feTurbulence type="fractalNoise" baseFrequency="0.03 0.04" numOctaves={1} seed={3} result="noise" />
        <feGaussianBlur in="noise" stdDeviation="1" result="soft" />
        <feDisplacementMap in="SourceGraphic" in2="soft" scale={6} xChannelSelector="R" yChannelSelector="G" />
      </filter>
    </svg>
  );
}
