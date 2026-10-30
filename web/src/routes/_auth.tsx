import { Outlet, createFileRoute } from "@tanstack/react-router";
import { useRef } from "react";
import { Surface } from "glass-ux/surface";

import { ConstellationBackdrop } from "@/components/constellation/ConstellationBackdrop";
import { BrandMark } from "@/components/ui";

/** Sign-in and setup screens: a centred glass card over the film constellation. */
export const Route = createFileRoute("/_auth")({
  component: AuthLayout,
});

function AuthLayout() {
  const card = useRef<HTMLDivElement>(null);
  return (
    <div className="relative isolate flex min-h-svh items-center justify-center overflow-hidden bg-background px-4 py-[calc(var(--safe-top)+2rem)] pb-[calc(var(--safe-bottom)+2rem)]">
      <ConstellationBackdrop clearRef={card} />
      <div ref={card} className="w-full max-w-md">
        <Surface material="panel" radius="dialog" padding="dialog">
          <div className="mb-8 flex items-center gap-3">
            <BrandMark size={44} />
            <div>
              <p className="text-title-lg font-display text-foreground">Librarian</p>
              <p className="text-label-sm text-muted">Your media, your server</p>
            </div>
          </div>
          <Outlet />
        </Surface>
      </div>
      <p className="absolute bottom-[calc(var(--safe-bottom)+0.75rem)] right-4 text-[10px] text-muted/60">Cast connections from Wikidata · images from Wikipedia</p>
    </div>
  );
}
