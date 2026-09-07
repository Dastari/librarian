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
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
