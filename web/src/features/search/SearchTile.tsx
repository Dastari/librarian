import { Link } from "@tanstack/react-router";
import { Surface } from "glass-ux/surface";
import { useId, type FocusEventHandler, type MouseEvent } from "react";

import { Artwork } from "@/components/ui/Artwork";
import { cn } from "@/lib/utils";
import { claimMorph } from "@/lib/view-transition";

import type { SearchHit } from "./searchResults";

/**
 * A search result the way TV apps show them: wide artwork with the title written over its
 * foot, so films, shows, albums and books share one grid. Square covers sit on a blurred
 * copy of themselves. One link covers the tile and takes the focus ring and the TV lift.
 */
export function SearchTile({ hit, onOpen, onFocus }: { hit: SearchHit; onOpen?: () => void; onFocus?: FocusEventHandler<HTMLElement> }) {
  const labelId = useId();
  const open = (event: MouseEvent<HTMLElement>) => {
    claimMorph(event.currentTarget.nextElementSibling);
    onOpen?.();
  };
  return (
    <article className="poster-card search-tile group relative min-w-0" data-kind={hit.kind}>
      <Link to={hit.to} params={hit.params} aria-labelledby={labelId} className="poster-navigation absolute inset-0 z-10 rounded-tile outline-none" data-focusable draggable={false} onClick={open} onFocus={onFocus} />
      <Surface material="card" radius="tile" padding="none" className="poster-art pointer-events-none relative overflow-clip">
        {hit.art === "wide" ? (
          <Artwork src={hit.image} alt="" aspect="backdrop" tint={hit.tint} displayWidth={420} fallback={hit.fallbackImage ? <Artwork src={hit.fallbackImage} alt="" aspect="backdrop" tint={hit.tint} displayWidth={420} className="absolute inset-0" /> : undefined} />
        ) : (
          <div className="relative">
            <Artwork src={hit.image} alt="" aspect="backdrop" tint={hit.tint} displayWidth={180} imageClassName="scale-125 blur-xl saturate-150 opacity-70" />
            <Artwork src={hit.image} alt="" aspect="square" tint={hit.tint} displayWidth={240} className={cn("absolute start-[5%] top-[10%] w-[45%] overflow-clip rounded-tile shadow-[var(--shadow-3)]", hit.kind === "artist" && "rounded-full")} />
          </div>
        )}
        <div className={cn("over-imagery absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/85 via-black/45 to-transparent px-3 pb-2.5 pt-10 text-white", hit.art === "square" && "ps-[53%]")}>
          <p id={labelId} className="line-clamp-2 text-title-sm">{hit.title}</p>
          <p className="truncate text-label-sm opacity-75">{hit.via ?? hit.meta}</p>
        </div>
      </Surface>
    </article>
  );
}
