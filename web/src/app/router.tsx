import { createRouter } from "@tanstack/react-router";

import { apolloClient } from "@/lib/apollo/client";
import { isTvApp } from "@/lib/tv-app";
import { routeTransitionTypes } from "@/lib/view-transition";
import { routeTree } from "@/routeTree.gen";

export const router = createRouter({
  routeTree,
  context: { apollo: apolloClient },
  // In the TV app every remote move would be "intent" and preload a page and its data; a budget TV
  // has neither the memory nor the network to spare (Q139).
  defaultPreload: isTvApp() ? false : "intent",
  defaultPreloadStaleTime: 0,
  scrollRestoration: true,
  scrollRestorationBehavior: "instant",
  getScrollRestorationKey: (location) => location.pathname,
  defaultPendingMs: 200,
  defaultPendingMinMs: 300,
  // Only the routed view animates (media.css names the shell layers), and not between tabs of
  // one section; Glass UX turns transitions off for Still and reduced motion.
  // The TV app skips them: a transition snapshots the old and new views as full-screen textures.
  defaultViewTransition: isTvApp() ? false : { types: routeTransitionTypes },
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
