import { Outlet, RouterProvider, createMemoryHistory, createRootRoute, createRoute, createRouter } from "@tanstack/react-router";
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Breadcrumbs, libraryTrail } from "../Breadcrumbs";

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
    const trail = screen.getByRole("navigation", { name: "Breadcrumb" });
    expect(trail.querySelector('[aria-current="page"]')).toHaveTextContent("Settings");
    expect(within(trail).queryByRole("link")).toBeNull();
  });

  it("builds the trail from static, param and loader labels", async () => {
    await mount("/libraries/lib-1/shows/s-9");
    const trail = screen.getByRole("navigation", { name: "Breadcrumb" });
    expect(within(trail).getByRole("link", { name: "Libraries" })).toHaveAttribute("href", "/libraries");
    expect(within(trail).getByRole("link", { name: "Library lib-1" })).toHaveAttribute("href", "/libraries/lib-1");
    // The page itself is never a link.
    expect(within(trail).queryByRole("link", { name: "Andor" })).toBeNull();
    expect(trail.querySelector('[aria-current="page"]')).toHaveTextContent("Andor");
  });
});

describe("Breadcrumbs for an item outside its library's routes", () => {
  async function mountAlbum() {
    const rootRoute = createRootRoute({ component: () => <><Breadcrumbs /><Outlet /></> });
    const album = createRoute({
      getParentRoute: () => rootRoute,
      path: "albums/$albumId",
      loader: () => ({ crumb: "Wasting Light", trail: libraryTrail({ id: "lib-m", name: "Music" }, "albums") }),
      component: () => null,
    });
    const router = createRouter({ routeTree: rootRoute.addChildren([album]), history: createMemoryHistory({ initialEntries: ["/albums/a-1"] }) });
    await router.load();
    render(<RouterProvider router={router as never} />);
    return screen.getByRole("navigation", { name: "Breadcrumb" });
  }

  it("reads Libraries, the library, the tab, then the album", async () => {
    const trail = await mountAlbum();
    expect(within(trail).getByRole("link", { name: "Libraries" })).toHaveAttribute("href", "/libraries");
    expect(within(trail).getByRole("link", { name: "Music" })).toHaveAttribute("href", "/libraries/lib-m");
    expect(within(trail).getByRole("link", { name: "Albums" })).toHaveAttribute("href", "/libraries/lib-m/albums");
    expect(trail.querySelector('[aria-current="page"]')).toHaveTextContent("Wasting Light");
  });

  it("names each detail page's tab and leaves the trail off without a library", () => {
    const library = { id: "l", name: "Lib" };
    expect(libraryTrail(library, "movies").map((crumb) => crumb.label)).toEqual(["Libraries", "Lib", "Browse"]);
    expect(libraryTrail(library, "shows").at(-1)?.href).toBe("/libraries/l/shows");
    expect(libraryTrail(library, "artists").at(-1)?.label).toBe("Artists");
    expect(libraryTrail(library, "audiobooks").at(-1)?.label).toBe("Audiobooks");
    expect(libraryTrail(null, "albums")).toEqual([]);
  });
});
