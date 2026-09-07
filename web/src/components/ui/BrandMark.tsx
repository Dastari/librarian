import type { CSSProperties } from "react";

import { cn } from "@/lib/utils";

/**
 * The Librarian mark as a React SVG. The fill is a gradient driven by the `--logo-from` and
 * `--logo-to` theme tokens, so the logo recolours with the active theme. `public/icons` holds
 * the static rasters for the manifest and favicons.
 */
export function BrandMark({ className, size = 32, style, title }: { className?: string; size?: number | string; style?: CSSProperties; title?: string }) {
  return (
    <svg viewBox="0 0 735 638" width={size} height={size} className={cn("shrink-0", className)} style={style} role={title ? "img" : undefined} aria-hidden={title ? undefined : true}>
      {title ? <title>{title}</title> : null}
      <defs>
        <linearGradient id="librarian-mark-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style={{ stopColor: "var(--logo-from)" }} />
          <stop offset="100%" style={{ stopColor: "var(--logo-to)" }} />
        </linearGradient>
      </defs>
      <g transform="translate(0,638) scale(0.1,-0.1)" fill="url(#librarian-mark-gradient)" stroke="none">
        <path d="M2280 6195 c-6 -16 -94 -278 -196 -584 -102 -306 -189 -565 -194 -576 -5 -11 -16 -45 -25 -75 -9 -30 -20 -64 -25 -75 -5 -11 -27 -78 -50 -150 -23 -71 -45 -139 -50 -150 -5 -11 -68 -198 -140 -415 -73 -217 -147 -438 -165 -490 -85 -248 -91 -269 -121 -440 -24 -132 -14 -395 19 -521 33 -124 64 -200 124 -300 92 -155 231 -293 391 -388 81 -49 250 -123 262 -116 8 5 130 367 130 386 0 5 -5 9 -12 9 -24 0 -155 65 -214 106 -227 158 -340 440 -294 734 22 140 -5 56 489 1525 122 363 256 764 299 890 l79 230 84 3 c337 14 653 -155 775 -413 72 -151 94 -355 59 -540 -12 -63 -389 -1217 -425 -1300 -5 -11 -19 -54 -32 -95 -13 -41 -137 -412 -275 -825 -253 -754 -282 -850 -302 -982 -15 -101 -14 -293 3 -390 61 -344 275 -634 581 -787 110 -55 216 -91 349 -117 98 -19 139 -20 1283 -18 852 1 1185 5 1192 13 15 20 150 439 165 516 68 337 34 622 -104 878 -155 287 -433 492 -775 572 -157 36 -271 41 -805 38 l-515 -3 -67 -199 c-37 -109 -65 -203 -62 -208 4 -6 239 -8 597 -7 326 1 615 -1 642 -5 461 -70 722 -367 702 -801 -5 -121 -38 -278 -74 -352 l-15 -33 -967 -2 c-531 0 -975 0 -986 1 -209 20 -329 58 -463 150 -173 117 -282 340 -282 575 0 42 5 98 10 124 6 26 15 70 21 97 12 53 104 332 419 1265 218 646 551 1646 569 1705 36 125 45 204 45 385 0 198 -8 251 -62 410 -106 311 -373 573 -702 688 -186 65 -257 75 -582 79 l-298 5 -10 -27z" />
      </g>
    </svg>
  );
}
