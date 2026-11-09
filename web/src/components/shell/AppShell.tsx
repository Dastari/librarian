import { Outlet, useRouter, useRouterState, Link } from "@tanstack/react-router";
import { AppShell as GlassAppShell, ShellBrand } from "glass-ux/shell";
import { useCallback, useEffect, useRef, useSyncExternalStore } from "react";

import { useDeviceCheckIn } from "@/features/devices/useDeviceCheckIn";
import { FileDialogHost } from "@/features/files/FileDetailDialog";
import { PlayerDock } from "@/features/player/PlayerDock";
import { currentAudioItem, playerStore } from "@/features/player/store";
import "@/lib/glass/brand";
import { useInputMode, useRouteFocus, useSpatialNavigation } from "@/lib/input-mode";
import { cn } from "@/lib/utils";

import { BottomTabs } from "./BottomTabs";
import { ConnectionBanner } from "./ConnectionBanner";
import { NavRail } from "./NavRail";
import { PageScroller } from "./PageScroller";
import { SHELL_ART_ID } from "./shellArt";
import { ShellCommands } from "./ShellCommands";
import { TopBar } from "./TopBar";

const hasAudio = () => Boolean(currentAudioItem(playerStore.getSnapshot()));

/**
 * The authenticated frame: Glass UX's AppShell owns the viewport, with the rail and top bar on
 * the canvas so they never scroll. Pages put their artwork in the layer behind it all, which
 * is how a title's picture reaches under the top bar and the rail. Each page has one scroller;
 * a route with `fixedHeight` scrolls inside its own panes instead.
 */
export function AppShell() {
  const { mode } = useInputMode();
  const router = useRouter();
  useDeviceCheckIn();
  // The remote's Back never leaves the app: it walks in-app history, then returns Home.
  const back = useCallback(() => {
    if (router.history.canGoBack()) router.history.back();
    else if (router.state.location.pathname !== "/") void router.navigate({ to: "/" });
  }, [router]);
  useSpatialNavigation(true, mode === "tv", back);
  const resolvedPath = useRouterState({ select: (state) => state.resolvedLocation?.pathname ?? state.location.pathname });
  useRouteFocus(resolvedPath, mode === "tv");
  const audioActive = useSyncExternalStore(playerStore.subscribe, hasAudio, hasAudio);
  const fixedHeight = useRouterState({ select: (state) => state.matches.some((match) => match.staticData?.fixedHeight) });
  const body = useRef<HTMLDivElement>(null);

  // Backdrops recede as their page scrolls, so text further down never fights the picture.
  useEffect(() => {
    const root = body.current;
    const art = document.getElementById(SHELL_ART_ID);
    if (!root || !art) return;
    art.style.setProperty("--art-scroll", "0");
    let frame = 0;
    const onScroll = (event: Event) => {
      const target = event.target;
      if (!(target instanceof HTMLElement) || !target.hasAttribute("data-page-scroller")) return;
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => art.style.setProperty("--art-scroll", String(Math.min(1, target.scrollTop / Math.max(1, target.clientHeight)))));
    };
    root.addEventListener("scroll", onScroll, { capture: true, passive: true });
    return () => {
      cancelAnimationFrame(frame);
      root.removeEventListener("scroll", onScroll, { capture: true });
    };
  }, [resolvedPath]);

  return (
    <div className="app-shell canvas-ambient relative isolate h-dvh w-full overflow-clip text-foreground">
      <div id={SHELL_ART_ID} className="shell-art" aria-hidden data-print="hide" />
      <ShellCommands />
      <GlassAppShell
        className="app-frame"
        contentAs="div"
        brand={<ShellBrand label="Librarian home" render={<Link to="/" />} data-focusable data-nav-zone="top" data-print="hide" className="nav-focus" />}
        rail={<NavRail />}
        topBar={<TopBar />}
        tabBar={<BottomTabs />}
      >
        <div ref={body} className="app-shell-body relative flex min-h-0 min-w-0 flex-col">
          <ConnectionBanner />
          <PageScroller id="main-scroll" scrolling={!fixedHeight} className={cn("app-shell-scroll relative flex flex-1 flex-col", audioActive && "max-md:pb-16")}>
            <main id="main" data-print="keep" className={cn("app-shell-main flex min-w-0 flex-1 flex-col", fixedHeight && "min-h-0")} tabIndex={-1}>
              <Outlet />
            </main>
          </PageScroller>
          <div className="contents" data-print="hide"><PlayerDock /></div>
        </div>
      </GlassAppShell>
      <FileDialogHost />
    </div>
  );
}
