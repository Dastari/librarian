import { Outlet, RouterProvider, createMemoryHistory, createRootRoute, createRoute, createRouter } from "@tanstack/react-router";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Breadcrumbs } from "../Breadcrumbs";

/**
 * A three-level tree that exercises both crumb sources: static labels, a label built from the
 * route params, and a label a loader resolved (the real show route names the show that way).
 */
function buildRouter(path: string) {
  const rootRoute = createRootRoute({
    component: () => (
      <>
        <Breadcrumbs />
        <Outlet />
      </>
    ),
  });
  const shell = createRoute({ getParentRoute: () => rootRoute, id: "shell", component: Outlet });
  const home = createRoute({ getParentRoute: () => shell, path: "/", staticData: { crumb: "Home" }, component: () => null });
  const libraries = createRoute({ getParentRoute: () => shell, path: "libraries", staticData: { crumb: "Libraries" }, component: Outlet });
  const library = createRoute({
    getParentRoute: () => libraries,
    path: "$libraryId",
    staticData: { crumb: (params: Record<string, string>) => `Library ${params.libraryId}` },
    component: Outlet,
  });
  const show = createRoute({
    getParentRoute: () => library,
    path: "shows/$showId",
    loader: () => ({ crumb: "Andor" }),
    component: () => null,
  });
  const settings = createRoute({ getParentRoute: () => shell, path: "settings", staticData: { crumb: "Settings" }, component: () => null });
  const router = createRouter({
    routeTree: rootRoute.addChildren([shell.addChildren([home, libraries.addChildren([library.addChildren([show])]), settings])]),
    history: createMemoryHistory({ initialEntries: [path] }),
  });
  return router;
}

async function mount(path: string) {
  const router = buildRouter(path);
  await router.load();
  return render(<RouterProvider router={router as never} />);
}

describe("Breadcrumbs", () => {
  it("renders nothing when no route declares a crumb", async () => {
    const rootRoute = createRootRoute({ component: () => <Breadcrumbs /> });
    const index = createRoute({ getParentRoute: () => rootRoute, path: "/", component: () => null });
    const router = createRouter({ routeTree: rootRoute.addChildren([index]), history: createMemoryHistory({ initialEntries: ["/"] }) });
    await router.load();
    render(<RouterProvider router={router as never} />);
    expect(screen.queryByRole("navigation", { name: "Breadcrumb" })).toBeNull();
  });

  it("shows a single unlinked crumb for a top-level page", async () => {
    await mount("/settings");
    expect(screen.getByRole("navigation", { name: "Breadcrumb" })).toBeInTheDocument();
    const current = screen.getByText("Settings");
    expect(current).toHaveAttribute("aria-current", "page");
    expect(screen.queryByRole("link")).toBeNull();
  });

  it("builds the trail from static, param and loader labels", async () => {
    await mount("/libraries/lib-1/shows/s-9");
    expect(screen.getAllByRole("listitem").map((item) => item.textContent)).toEqual(["Libraries", "Library lib-1", "Andor"]);
    expect(screen.getByRole("link", { name: "Libraries" })).toHaveAttribute("href", "/libraries");
    expect(screen.getByRole("link", { name: "Library lib-1" })).toHaveAttribute("href", "/libraries/lib-1");
    // The page itself is never a link.
    expect(screen.queryByRole("link", { name: "Andor" })).toBeNull();
    expect(screen.getByText("Andor")).toHaveAttribute("aria-current", "page");
  });

  it("hides all but the last two crumbs on a phone", async () => {
    await mount("/libraries/lib-1/shows/s-9");
    const [first, second, third] = screen.getAllByRole("listitem");
    expect(first!.className).toContain("hidden sm:block");
    expect(second!.className).not.toContain("hidden");
    expect(third!.className).not.toContain("hidden");
  });
});
