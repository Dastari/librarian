import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { LibraryLayout, type LibraryDetail } from "@/features/libraries/LibraryLayout";
import { DetailShell } from "@/features/media/DetailShell";
import { SettingsLayout } from "@/features/settings/SettingsLayout";

const route = vi.hoisted(() => ({ pathname: "/settings/general", routeId: "" }));
vi.mock("@tanstack/react-router", () => ({
  Outlet: () => <button>Outlet action</button>,
  Link: ({ children }: { children: ReactNode }) => <a>{children}</a>,
  useRouterState: ({ select }: { select: (state: unknown) => unknown }) => select({
    location: { pathname: route.pathname }, matches: [{ routeId: route.routeId }],
  }),
}));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => false }));
vi.mock("@/features/libraries/useLibraryScan", () => ({ useLibraryScan: () => ({ unresolvedIssues: 0 }) }));
vi.mock("@/components/ui", async () => ({
  ...await vi.importActual("../PageHeader"),
  SideTabs: ({ children }: { children: ReactNode }) => <section>{children}</section>,
  SegmentTabs: () => <nav aria-label="Library sections" />,
  HeroBanner: () => <h1>Film title</h1>,
  MediaBackdrop: () => null,
}));

const connection = { pageInfo: { totalCount: 3 } };
const library = {
  id: "library", name: "Family films", libraryType: "movies", lastScannedAt: null,
  movies: connection, shows: connection, albums: connection, audiobooks: connection,
  collections: connection, mediaFiles: connection,
} as unknown as LibraryDetail;

beforeEach(() => { route.pathname = "/settings/general"; route.routeId = ""; });

function outletPane() {
  const action = screen.getByRole("button", { name: "Outlet action" });
  const pane = action.parentElement!;
  // These are host contracts, not claims about computed browser geometry.
  expect(pane).toHaveAttribute("data-print-flow");
  expect(pane).toHaveClass("fa-focus-room", "min-h-0");
  expect(pane.className).toContain("var(--focus-reach)");
  expect(pane.parentElement?.closest("[data-print-flow]")).toBeTruthy();
  return pane;
}

describe("nested route boundaries", () => {
  it.each(["", "/_app/libraries/$libraryId/settings", "/_app/libraries/$libraryId/scans"])("reserves focus room and print flow for library outlet %s", (routeId) => {
    route.routeId = routeId;
    render(<LibraryLayout library={library} />);
    expect(outletPane()).toHaveClass(routeId ? "overflow-y-auto" : "overflow-hidden");
    // The breadcrumb names the library on screen; the heading remains for assistive technology.
    expect(screen.getByRole("heading", { name: "Family films" })).toHaveClass("sr-only");
  });

  it.each(["/settings/general", "/settings/logs"])("keeps the screen scroll owner and print marker for %s", (pathname) => {
    route.pathname = pathname;
    render(<SettingsLayout />);
    expect(outletPane()).toHaveClass(pathname.endsWith("logs") ? "overflow-hidden" : "overflow-y-auto");
  });

  it("lets detail content flow through both host boundaries on paper", () => {
    render(<DetailShell backdrop={null} hero={{} as Parameters<typeof DetailShell>[0]["hero"]}><article>Loaded chapters</article></DetailShell>);
    const content = screen.getByRole("article");
    expect(content.parentElement).toHaveAttribute("data-print-flow");
    expect(content.parentElement?.parentElement).toHaveAttribute("data-print-flow");
    expect(content.parentElement?.parentElement?.parentElement).toHaveAttribute("data-print-flow");
  });
});
