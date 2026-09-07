import { Outlet, createFileRoute } from "@tanstack/react-router";
import { useRef } from "react";

import { ConstellationBackdrop } from "@/components/constellation/ConstellationBackdrop";
import { BrandMark, GlassFilters, LiquidGlass } from "@/components/ui";

/** Sign-in and setup screens: a centred glass card over the film constellation. */
export const Route = createFileRoute("/_auth")({
  component: AuthLayout,
});

function AuthLayout() {
  const card = useRef<HTMLDivElement>(null);
  return (
    <div className="relative isolate flex min-h-svh items-center justify-center overflow-hidden bg-background px-4 py-[calc(var(--safe-top)+2rem)] pb-[calc(var(--safe-bottom)+2rem)]">
      <GlassFilters />
      <ConstellationBackdrop clearRef={card} />
      <div ref={card} className="w-full max-w-md">
        <LiquidGlass strong className="rounded-[1.75rem] p-8 sm:p-10" depth={10} strength={40} blur={10}>
          <div className="mb-8 flex items-center gap-3">
            <BrandMark size={44} />
            <div>
              <p className="text-title-lg font-display text-foreground">Librarian</p>
              <p className="text-label-sm text-muted">Your media, your server</p>
            </div>
          </div>
          <Outlet />
        </LiquidGlass>
      </div>
      <p className="absolute bottom-[calc(var(--safe-bottom)+0.75rem)] right-4 text-[10px] text-muted/60">Cast connections from Wikidata · images from Wikipedia</p>
    </div>
  );
}
