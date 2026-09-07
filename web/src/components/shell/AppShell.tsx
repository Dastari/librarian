import { Outlet, useRouterState } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { useSpatialNavigation } from "@/lib/input-mode";
import { cn } from "@/lib/utils";

import { GlassFilters } from "@/components/ui";

import { BottomTabs } from "./BottomTabs";
import { ConnectionBanner } from "./ConnectionBanner";
import { NavRail } from "./NavRail";
import { PlayerDock } from "@/features/player/PlayerDock";
import { TopBar } from "./TopBar";
import { UpdatePrompt } from "./UpdatePrompt";

/**
 * Authenticated application frame: rail + top bar + scrolling content + player dock.
 * Pages that open with a hero (home, detail pages) set `hero` in their route static data so
 * the top bar floats transparently over the artwork until the user scrolls.
 */
export function AppShell() {
  useSpatialNavigation();
  const hero = useRouterState({ select: (state) => state.matches.some((match) => match.staticData?.hero) });
  const fixedHeight = useRouterState({ select: (state) => state.matches.some((match) => match.staticData?.fixedHeight) });
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const main = document.getElementById("main-scroll");
    if (!main) return;
    const onScroll = () => setScrolled(main.scrollTop > 24);
    onScroll();
    main.addEventListener("scroll", onScroll, { passive: true });
    return () => main.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <div className="ambient-canvas flex h-svh w-full overflow-hidden text-foreground">
      <GlassFilters />
      <NavRail />
      <div className="relative flex min-w-0 flex-1 flex-col">
        <ConnectionBanner />
        <div id="main-scroll" className={cn("scrollbar-thin relative flex min-h-0 flex-1 flex-col overflow-x-hidden", fixedHeight ? "overflow-y-hidden" : "overflow-y-auto", hero && "[scrollbar-gutter:stable]")}>
          <div className={cn("sticky top-0 z-20 shrink-0", hero && "-mb-[calc(var(--topbar-height)+var(--safe-top))]")}>
            <TopBar overlay={hero && !scrolled} />
          </div>
          <main id="main" className={cn("flex min-h-0 flex-1 flex-col pb-[calc(var(--tabbar-height)+var(--safe-bottom))] md:pb-0", fixedHeight && "h-full")} tabIndex={-1}>
            <Outlet />
          </main>
        </div>
        <PlayerDock />
      </div>
      <BottomTabs />
      <UpdatePrompt />
    </div>
  );
}
