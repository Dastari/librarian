import { render, screen } from "@testing-library/react";
import { SidebarProvider } from "glass-ux/sidebar";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { InputModeProvider } from "@/lib/input-mode";

import { NavRail } from "../NavRail";

vi.mock("@tanstack/react-router", () => ({
  Link: ({ to, params: _params, ...props }: { to: string; params?: unknown }) => <a href={to} {...props} />,
  useRouterState: () => "/",
}));
vi.mock("@/features/libraries/browser/useCompleteConnection", () => ({
  useCompleteConnection: () => ({ rows: [{ id: "l1", name: "Movies", libraryType: "MOVIES", scanning: false }] }),
}));
vi.mock("@/features/personal/usePersonalLibrary", () => ({ useLibraryPreferences: () => ({ rows: [] }) }));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => true, useSession: () => ({ status: "authenticated" }) }));
vi.mock("../useShellCounts", () => ({ useShellCounts: () => ({ downloads: 3, notifications: 0 }) }));

function rail() {
  render(<InputModeProvider><SidebarProvider defaultOpen><NavRail /></SidebarProvider></InputModeProvider>);
  return screen.getByRole("navigation", { name: "Primary" });
}

describe("NavRail", () => {
  beforeEach(() => window.localStorage.clear());

  it("is a side zone the remote can reach, with every destination focusable", () => {
    const nav = rail();
    expect(nav).toHaveAttribute("data-nav-zone", "rail");
    expect(nav).toHaveAttribute("data-nav-side");
    expect(screen.getByRole("link", { name: /Downloads/ })).toHaveAttribute("data-focusable");
    expect(screen.getByRole("link", { name: "Movies" })).toHaveAttribute("data-focusable");
    expect(screen.getByRole("button", { name: "Collapse navigation" })).toHaveAttribute("data-focusable");
  });

  it("marks the current page and counts downloads", () => {
    rail();
    expect(screen.getByRole("link", { name: "Home" , current: "page" })).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
  });

  it("has a resize handle for a pointer but none for a remote", () => {
    const pointer = rail();
    expect(pointer.querySelector(".fa-sidebar-rail")).not.toBeNull();
    pointer.remove();
    window.localStorage.setItem("librarian.inputMode", "tv");
    expect(rail().querySelector(".fa-sidebar-rail")).toBeNull();
  });
});
