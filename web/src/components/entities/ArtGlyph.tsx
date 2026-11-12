import { useState, type ReactNode } from "react";

/**
 * A title's own artwork in an identity's glyph slot (list rows, table cells), with the kind's
 * glyph standing in until there's art or when it fails to load. Rows read faster by their
 * covers; the state dot stays on the slot's corner. Sized by `.entity-art` in overrides.css.
 */
export function ArtGlyph({ src, fallback, shape = "square" }: { src: string | null | undefined; fallback: ReactNode; shape?: "square" | "poster" }) {
  const [failed, setFailed] = useState<string | null>(null);
  if (!src || failed === src) return <>{fallback}</>;
  return (
    <span className="entity-art" data-shape={shape}>
      <img src={src} alt="" loading="lazy" decoding="async" draggable={false} onError={() => setFailed(src)} />
    </span>
  );
}

/**
 * A title's artwork in an identity's image slot (headers, dialogs). The artwork URL always
 * exists, so a title without art would show the browser's broken-image icon: the kind's glyph
 * stands in instead.
 */
export function ArtImage({ src, fallback }: { src: string | null | undefined; fallback: ReactNode }) {
  const [failed, setFailed] = useState<string | null>(null);
  if (!src || failed === src) return <span className="grid size-full place-items-center bg-surface-secondary text-muted [&>svg]:size-1/2" aria-hidden>{fallback}</span>;
  return <img src={src} alt="" decoding="async" draggable={false} onError={() => setFailed(src)} />;
}
