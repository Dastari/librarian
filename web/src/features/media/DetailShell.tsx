import { Children, useState, type ReactNode } from "react";

import { HeroBanner, MediaBackdrop } from "@/components/ui";

interface DetailShellProps {
  hero: Parameters<typeof HeroBanner>[0];
  /** Wide artwork behind the page; the poster is the fallback colour source. */
  backdrop: string | null | undefined;
  children: ReactNode;
}

/**
 * Hero + sections used by every media detail page. The artwork belongs to the whole page,
 * not a boxed banner: it sits behind the transparent top bar and fades into the canvas.
 * Each child is a navigation section: remote focus reveals it whole, never half a panel.
 * With wide art the hero fills the first screen; without it (albums, books, missing art)
 * the hero sizes to its text so the tracks or episodes start on the first screen.
 */
export function DetailShell({ hero, backdrop, children }: DetailShellProps) {
  const [wide, setWide] = useState(Boolean(backdrop));
  return (
    <div data-print-flow className="relative isolate flex min-w-0 flex-col overflow-x-clip pb-8 md:pb-12">
      <MediaBackdrop src={backdrop} fallback={hero.poster} tint={hero.tint} onWideChange={setWide} />
      <HeroBanner fill={wide ? "screen" : "content"} {...hero} />
      <div data-print-flow data-detail-content className="page-gutter flex min-w-0 flex-col gap-10 md:gap-12">
        {Children.toArray(children).map((child, index) => (
          <div key={index} data-print-flow data-nav-section className="min-w-0 empty:hidden">
            {child}
          </div>
        ))}
      </div>
    </div>
  );
}
