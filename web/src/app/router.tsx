import { createRouter } from "@tanstack/react-router";

import { apolloClient } from "@/lib/apollo/client";
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
  // Only the routed view animates (media.css names the shell layers); Glass UX turns
  // transitions off for Still and reduced motion.
  defaultViewTransition: true,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
