import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { InputModeProvider } from "@/lib/input-mode";

import { NavRail } from "../NavRail";
import { TopBar } from "../TopBar";

const router = vi.hoisted(() => ({ pathname: "/libraries/l1/movies" }));
vi.mock("@tanstack/react-router", () => ({
  Link: ({ to, params: _params, ...props }: { to: string; params?: unknown }) => <a href={to} {...props} />,
  useNavigate: () => vi.fn(),
  useRouterState: ({ select }: { select?: (state: unknown) => unknown } = {}) => (select ? select({ location: { pathname: router.pathname } }) : router.pathname),
}));
vi.mock("glass-ux/command-palette", () => ({ useCommandPalette: () => ({ open: vi.fn() }) }));
vi.mock("@/features/player/CastButton", () => ({ CastButton: () => <button type="button" aria-label="Cast">Cast</button> }));
vi.mock("../Breadcrumbs", () => ({ Breadcrumbs: () => <nav aria-label="Breadcrumb"><a href="/libraries">Libraries</a></nav> }));
vi.mock("@/features/libraries/browser/useCompleteConnection", () => ({ useCompleteConnection: () => ({ rows: [] }) }));
vi.mock("@/features/personal/usePersonalLibrary", () => ({ useLibraryPreferences: () => ({ rows: [] }) }));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => true, useSession: () => ({ status: "authenticated", user: { id: "u1", username: "toby", displayName: "Toby" } }) }));
vi.mock("@/lib/auth/session", () => ({ session: { logout: vi.fn(async () => undefined), onChange: () => () => undefined } }));
vi.mock("@/lib/theme", () => ({ THEMES: [{ id: "dark", label: "Dark" }], useTheme: () => ({ preference: "dark", setPreference: vi.fn() }) }));
vi.mock("../useShellCounts", () => ({ useShellCounts: () => ({ downloads: 0, notifications: 2 }) }));

function chrome() {
  render(<InputModeProvider><TopBar /><NavRail /></InputModeProvider>);
  return { top: document.querySelector<HTMLElement>('[data-nav-zone="top"]')!, rail: screen.getByRole("navigation", { name: "Primary" }) };
}

beforeEach(() => {
  window.localStorage.clear();
  router.pathname = "/libraries/l1/movies";
});

describe("app chrome in the normal layout", () => {
  it("keeps search, commands, cast and the account in the top bar, and nothing extra on the rail", () => {
    const { top, rail } = chrome();
    expect(within(top).getByRole("button", { name: /Search library/ })).toBeInTheDocument();
    expect(within(top).getByRole("button", { name: "Commands" })).toBeInTheDocument();
    expect(within(top).getByRole("button", { name: "Cast" })).toBeInTheDocument();
    expect(within(top).getByRole("button", { name: /Toby/ })).toBeInTheDocument();
    expect(within(top).getByRole("button", { name: "Back" })).toBeInTheDocument();
    expect(within(rail).queryByRole("button", { name: /Toby/ })).toBeNull();
    expect(within(rail).getByRole("link", { name: "Search" })).toBeInTheDocument();
  });
});

describe("app chrome in TV mode", () => {
  beforeEach(() => window.localStorage.setItem("librarian.inputMode", "tv"));

  it("drops search, commands, cast, back and the account from the top bar, keeping the breadcrumbs", () => {
    const { top } = chrome();
    expect(within(top).queryByRole("button")).toBeNull();
    expect(screen.queryByRole("button", { name: /Search library/ })).toBeNull();
    expect(screen.queryByRole("button", { name: "Commands" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Cast" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Back" })).toBeNull();
    expect(within(top).getByRole("link", { name: "Libraries" })).toBeInTheDocument();
  });

  it("still reaches Search from the rail, and puts the account at its foot as a focusable item", () => {
    const { rail } = chrome();
    expect(within(rail).getByRole("link", { name: "Search" })).toHaveAttribute("href", "/search");
    const account = within(rail).getByRole("button", { name: /Toby/ });
    expect(account).toHaveAttribute("data-focusable");
    const focusable = [...rail.querySelectorAll<HTMLElement>("[data-focusable]")];
    expect(focusable.at(-1)).toBe(account);
    // After Settings, so it is the last stop of the D-pad walk down the rail.
    expect(focusable.at(-2)).toBe(within(rail).getByRole("link", { name: "Settings" }));
  });

  it("opens the same account menu from the rail and returns focus to the rail item when it closes", async () => {
    const { rail } = chrome();
    const account = within(rail).getByRole("button", { name: /Toby/ });
    account.focus();
    await userEvent.keyboard("{Enter}");
    const menu = await screen.findByRole("menu", { name: "Account menu" });
    expect(within(menu).getByRole("menuitem", { name: /Settings/ })).toBeInTheDocument();
    expect(within(menu).getByRole("menuitem", { name: "Sign out" })).toBeInTheDocument();
    expect(within(menu).getByRole("menuitemcheckbox", { name: /TV mode/ })).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await act(async () => undefined);
    expect(document.activeElement).toBe(account);
  });
});
