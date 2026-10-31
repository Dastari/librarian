import { createRouter } from "@tanstack/react-router";

import { apolloClient } from "@/lib/apollo/client";
import { routeTransitionTypes } from "@/lib/view-transition";
import { routeTree } from "@/routeTree.gen";

export const router = createRouter({
  routeTree,
  context: { apollo: apolloClient },
  defaultPreload: "intent",
  defaultPreloadStaleTime: 0,
  scrollRestoration: true,
  scrollRestorationBehavior: "instant",
  getScrollRestorationKey: (location) => location.pathname,
  defaultPendingMs: 200,
  defaultPendingMinMs: 300,
  // Only the routed view animates (media.css names the shell layers), and not between tabs of
  // one section; Glass UX turns transitions off for Still and reduced motion.
  defaultViewTransition: { types: routeTransitionTypes },
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
