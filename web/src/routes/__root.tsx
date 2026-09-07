import { Outlet, createRootRouteWithContext } from "@tanstack/react-router";
import type { ApolloClient } from "@apollo/client";
import { NuqsAdapter } from "nuqs/adapters/tanstack-router";

import { NotFound } from "@/components/shell/NotFound";
import { RouteError } from "@/components/shell/RouteError";

export interface RouterContext {
  apollo: ApolloClient;
}

declare module "@tanstack/react-router" {
  interface StaticDataRouteOption {
    /** Breadcrumb label, or a function of the route params. */
    crumb?: string | ((params: Record<string, string>) => string);
    /** The page opens with a full-bleed hero; the top bar floats over it. */
    hero?: boolean;
    /** The page manages its own scrolling (fixed header and navigation, scrolling content). */
    fixedHeight?: boolean;
  }
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: () => (
    <NuqsAdapter>
      <Outlet />
    </NuqsAdapter>
  ),
  notFoundComponent: NotFound,
  errorComponent: RouteError,
});
