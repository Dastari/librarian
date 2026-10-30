import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AppearanceProvider, useAppearance } from "glass-ux/appearance";
import { useEffect } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { APPEARANCE_KEY, LEGACY_THEME_KEY } from "@/lib/glass/appearance";

import { THEMES, ThemeProvider, useTheme } from "../theme";

const DARK_QUERY = "(prefers-color-scheme: dark)";
const realMatchMedia = window.matchMedia;
const queries = new Map<string, Set<() => void>>();
let systemPrefersDark = true;

function mockMatchMedia() {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: (query: string) => ({
      get matches() { return query === DARK_QUERY && systemPrefersDark; },
      media: query,
      onchange: null,
      addEventListener: (_: string, listener: () => void) => {
        if (!queries.has(query)) queries.set(query, new Set());
        queries.get(query)!.add(listener);
      },
      removeEventListener: (_: string, listener: () => void) => queries.get(query)?.delete(listener),
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }) as unknown as MediaQueryList,
  });
}

function ResetAppearance() {
  const { reset } = useAppearance();
  useEffect(() => reset(), [reset]);
  return null;
}

function Probe() {
  const { preference, resolved, scheme, setPreference } = useTheme();
  const { appearance, update } = useAppearance();
  return (
    <div>
      <p data-testid="state">{`${preference}/${resolved}/${scheme}`}</p>
      <output data-testid="appearance">{JSON.stringify(appearance)}</output>
      {[...THEMES.map((theme) => theme.id), "system" as const].map((id) => (
        <button key={id} type="button" onClick={() => setPreference(id)}>{id}</button>
      ))}
      <button type="button" onClick={() => update({ glass: "solid", density: "compact", uiScale: 1.125 })}>compact solid</button>
    </div>
  );
}

const mount = () => render(<ThemeProvider><Probe /></ThemeProvider>);

beforeEach(() => {
  systemPrefersDark = true;
  queries.clear();
  mockMatchMedia();
  // Reset through the public API: unmounting does not reset Glass's shared store.
  const reset = render(<AppearanceProvider><ResetAppearance /></AppearanceProvider>);
  reset.unmount();
  window.localStorage.removeItem(LEGACY_THEME_KEY);
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.className = "";
});

afterEach(() => {
  cleanup();
  Object.defineProperty(window, "matchMedia", { configurable: true, writable: true, value: realMatchMedia });
  vi.restoreAllMocks();
});

describe("theme provider", () => {
  it("defaults to Cinema and applies Glass and derived legacy attributes", () => {
    mount();
    expect(screen.getByTestId("state")).toHaveTextContent("dark/dark/dark");
    expect(document.documentElement).toHaveAttribute("data-mode", "dark");
    expect(document.documentElement).toHaveAttribute("data-palette", "cinema");
    expect(document.documentElement).toHaveAttribute("data-glass", "full");
    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
    expect(document.documentElement).toHaveClass("dark");
    expect(document.documentElement.style.colorScheme).toBe("dark");
  });

  it("persists the light choice in Glass appearance and the rollback key", async () => {
    mount();
    await userEvent.click(screen.getByRole("button", { name: "light" }));
    expect(JSON.parse(window.localStorage.getItem(APPEARANCE_KEY)!)).toMatchObject({ mode: "light", palette: "cinema" });
    expect(window.localStorage.getItem(LEGACY_THEME_KEY)).toBe("light");
    expect(document.documentElement).toHaveAttribute("data-mode", "light");
    expect(document.documentElement).toHaveAttribute("data-palette", "cinema");
    expect(document.documentElement).toHaveAttribute("data-theme", "light");
    expect(document.documentElement).toHaveClass("light");
    expect(document.documentElement).not.toHaveClass("dark");
  });

  it.each(["midnight", "ember", "forest"] as const)("migrates the legacy %s palette only once", (palette) => {
    window.localStorage.setItem(LEGACY_THEME_KEY, palette);
    const { unmount } = mount();
    expect(screen.getByTestId("state")).toHaveTextContent(`${palette}/${palette}/dark`);
    expect(document.documentElement).toHaveAttribute("data-palette", palette);
    expect(document.documentElement).toHaveAttribute("data-theme", palette);
    expect(JSON.parse(window.localStorage.getItem(APPEARANCE_KEY)!)).toMatchObject({ mode: "dark", palette });
    unmount();
    window.localStorage.setItem(LEGACY_THEME_KEY, "light");
    mount();
    expect(screen.getByTestId("state")).toHaveTextContent(`${palette}/${palette}/dark`);
  });

  it.each(["light", "system"] as const)("migrates the legacy %s mode", (mode) => {
    systemPrefersDark = false;
    window.localStorage.setItem(LEGACY_THEME_KEY, mode);
    mount();
    expect(screen.getByTestId("state")).toHaveTextContent(`${mode}/light/light`);
    expect(JSON.parse(window.localStorage.getItem(APPEARANCE_KEY)!)).toMatchObject({ mode, palette: "cinema" });
  });

  it("keeps existing Glass preferences ahead of the legacy key", () => {
    const stored = { mode: "light", palette: "forest", glass: "solid", density: "compact", uiScale: 1.125 };
    window.localStorage.setItem(APPEARANCE_KEY, JSON.stringify(stored));
    window.localStorage.setItem(LEGACY_THEME_KEY, "midnight");
    mount();
    expect(screen.getByTestId("state")).toHaveTextContent("forest/light/light");
    expect(document.documentElement).toHaveAttribute("data-mode", "light");
    expect(document.documentElement).toHaveAttribute("data-palette", "forest");
    expect(document.documentElement).toHaveAttribute("data-glass", "solid");
    expect(document.documentElement).toHaveAttribute("data-density", "compact");
    expect(document.documentElement.style.getPropertyValue("--ui-scale")).toBe("1.125");
    expect(JSON.parse(window.localStorage.getItem(APPEARANCE_KEY)!)).toEqual(stored);
  });

  it("ignores an unknown legacy theme", () => {
    window.localStorage.setItem(LEGACY_THEME_KEY, "neon");
    mount();
    expect(screen.getByTestId("state")).toHaveTextContent("dark/dark/dark");
    expect(document.documentElement).toHaveAttribute("data-palette", "cinema");
  });

  it("falls back for invalid appearance without restoring the legacy key", () => {
    window.localStorage.setItem(APPEARANCE_KEY, "invalid json");
    window.localStorage.setItem(LEGACY_THEME_KEY, "forest");
    mount();
    expect(screen.getByTestId("state")).toHaveTextContent("dark/dark/dark");
    expect(window.localStorage.getItem(APPEARANCE_KEY)).toBe("invalid json");
  });

  it("follows changes to the OS dark query while using system mode", async () => {
    systemPrefersDark = false;
    mount();
    await userEvent.click(screen.getByRole("button", { name: "system" }));
    expect(screen.getByTestId("state")).toHaveTextContent("system/light/light");
    expect(document.documentElement).toHaveAttribute("data-mode", "light");
    act(() => {
      systemPrefersDark = true;
      for (const listener of queries.get(DARK_QUERY) ?? []) listener();
    });
    expect(screen.getByTestId("state")).toHaveTextContent("system/dark/dark");
    expect(document.documentElement).toHaveAttribute("data-mode", "dark");
    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
  });

  it("restores saved material, density, scale and palette on remount", async () => {
    const { unmount } = mount();
    await userEvent.click(screen.getByRole("button", { name: "ember" }));
    await userEvent.click(screen.getByRole("button", { name: "compact solid" }));
    const stored = window.localStorage.getItem(APPEARANCE_KEY);
    unmount();
    mount();
    expect(JSON.parse(screen.getByTestId("appearance").textContent!)).toMatchObject({ mode: "dark", palette: "ember", glass: "solid", density: "compact", uiScale: 1.125 });
    expect(document.documentElement).toHaveAttribute("data-mode", "dark");
    expect(document.documentElement).toHaveAttribute("data-palette", "ember");
    expect(document.documentElement).toHaveAttribute("data-glass", "solid");
    expect(document.documentElement).toHaveAttribute("data-theme", "ember");
    expect(window.localStorage.getItem(APPEARANCE_KEY)).toBe(stored);
  });

  it("throws when used outside the provider", () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => render(<Probe />)).toThrow(/ThemeProvider/);
    consoleError.mockRestore();
  });
});
