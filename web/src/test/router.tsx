import { RouterProvider, createMemoryHistory, createRootRoute, createRoute, createRouter } from "@tanstack/react-router";
import type { ReactNode } from "react";

/**
 * A throwaway TanStack router around a piece of UI.
 *
 * Components in `src/` use `Link`, `useNavigate` and `useRouterState`, all of which need a
 * router in context. The real route tree pulls in the whole app, so tests mount the component
 * as the root route of a memory router instead: every path matches, links render real hrefs,
 * and `navigate` calls can be observed through `router.state.location`.
 */
export interface TestRouterOptions {
  /** Initial URL, e.g. `/settings/quality`. */
  path?: string;
  /** Extra route paths so `Link`s to them resolve; the splat route already matches everything. */
  routes?: string[];
}

export function createTestRouter(ui: ReactNode, options: TestRouterOptions = {}) {
  const rootRoute = createRootRoute({ component: () => <>{ui}</> });
  const children = [
    createRoute({ getParentRoute: () => rootRoute, path: "/", component: () => null }),
    createRoute({ getParentRoute: () => rootRoute, path: "$", component: () => null }),
    ...(options.routes ?? []).map((path) => createRoute({ getParentRoute: () => rootRoute, path, component: () => null })),
  ];
  return createRouter({
    routeTree: rootRoute.addChildren(children),
    history: createMemoryHistory({ initialEntries: [options.path ?? "/"] }),
  });
}

export type TestRouter = ReturnType<typeof createTestRouter>;

export function TestRouterProvider({ router }: { router: TestRouter }) {
  return <RouterProvider router={router as never} />;
}
