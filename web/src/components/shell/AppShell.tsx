import { Outlet, useRouter, useRouterState } from "@tanstack/react-router";
import { useCallback, useEffect, useState, useSyncExternalStore } from "react";

import { useInputMode, useRouteFocus, useSpatialNavigation } from "@/lib/input-mode";
import { cn } from "@/lib/utils";


import { ShellCommands } from "./ShellCommands";
import { BottomTabs } from "./BottomTabs";
import { ConnectionBanner } from "./ConnectionBanner";
import { NavRail } from "./NavRail";
import { PlayerDock } from "@/features/player/PlayerDock";
import { currentAudioItem, playerStore } from "@/features/player/store";
import { TopBar } from "./TopBar";
import { UpdatePrompt } from "./UpdatePrompt";

const hasAudio = () => Boolean(currentAudioItem(playerStore.getSnapshot()));

/**
 * Authenticated application frame: rail + top bar + scrolling content + player dock.
 * Pages that open with a hero (home, detail pages) set `hero` in their route static data so
 * the top bar floats transparently over the artwork until the user scrolls. `--view-h` on
 * the scroller is its visible height, which heroes use to fill exactly the first screen.
 */
export function AppShell() {
  const { mode } = useInputMode();
  const router = useRouter();
  // The remote's Back never leaves the app: it walks in-app history, then returns Home.
  const back = useCallback(() => {
    if (router.history.canGoBack()) router.history.back();
    else if (router.state.location.pathname !== "/") void router.navigate({ to: "/" });
  }, [router]);
  useSpatialNavigation(true, mode === "tv", back);
  const resolvedPath = useRouterState({ select: (state) => state.resolvedLocation?.pathname ?? state.location.pathname });
  useRouteFocus(resolvedPath, mode === "tv");
  const audioActive = useSyncExternalStore(playerStore.subscribe, hasAudio, hasAudio);
  const hero = useRouterState({ select: (state) => state.matches.some((match) => match.staticData?.hero) });
  const fixedHeight = useRouterState({ select: (state) => state.matches.some((match) => match.staticData?.fixedHeight) });
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const main = document.getElementById("main-scroll");
    if (!main) return;
    const onScroll = () => setScrolled(main.scrollTop > 24);
    const measure = () => main.style.setProperty("--view-h", `${main.clientHeight}px`);
    onScroll();
    measure();
    main.addEventListener("scroll", onScroll, { passive: true });
    const observer = new ResizeObserver(measure);
    observer.observe(main);
    return () => {
      main.removeEventListener("scroll", onScroll);
      observer.disconnect();
    };
  }, []);

  return (
    <div className="app-shell canvas-ambient flex h-svh w-full overflow-hidden text-foreground">
      <ShellCommands />
      <NavRail />
      <div className="app-shell-body relative flex min-w-0 flex-1 flex-col">
        <ConnectionBanner />
        <div id="main-scroll" className={cn("app-shell-scroll scrollbar-thin relative flex min-h-0 flex-1 flex-col overflow-x-hidden", fixedHeight ? "overflow-y-hidden" : "overflow-y-auto", hero && "[scrollbar-gutter:stable]")}>
          <div className={cn("app-shell-top sticky top-0 z-20 shrink-0", hero && "-mb-[calc(var(--topbar-height)+var(--safe-top))]")}>
            <TopBar overlay={hero && !scrolled} />
          </div>
          <main id="main" data-print="keep" className={cn("app-shell-main flex min-h-0 flex-1 flex-col md:pb-0", audioActive ? "pb-[calc(var(--tabbar-height)+var(--safe-bottom)+4rem)]" : "pb-[calc(var(--tabbar-height)+var(--safe-bottom))]", fixedHeight && "overflow-hidden")} tabIndex={-1}>
            <Outlet />
          </main>
        </div>
        <div className="contents" data-print="hide"><PlayerDock /></div>
      </div>
      <BottomTabs />
      <div className="contents" data-print="hide"><UpdatePrompt /></div>
    </div>
  );
}
