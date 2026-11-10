import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AppearanceProvider, useAppearance } from "glass-ux/appearance";
import { useEffect } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ThemePicker } from "@/components/ui/ThemePicker";
import { APPEARANCE_KEY, BLUR_KEY, LEGACY_THEME_KEY, migrateAppearance, setBlur } from "@/lib/glass/appearance";

import { THEMES, ThemeProvider, useTheme } from "../theme";

const DARK_QUERY = "(prefers-color-scheme: dark)";
const realMatchMedia = window.matchMedia;
let systemPrefersDark = true;

function mockMatchMedia() {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: (query: string) => ({
      get matches() { return query === DARK_QUERY && systemPrefersDark; },
      media: query,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
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
  const { preference, scheme, setPreference, blur } = useTheme();
  const { appearance, update } = useAppearance();
  return (
    <div>
      <p data-testid="state">{`${preference}/${scheme}/${blur ? "blur" : "no-blur"}`}</p>
      <output data-testid="appearance">{JSON.stringify(appearance)}</output>
      {THEMES.map((theme) => (
        <button key={theme.id} type="button" onClick={() => setPreference(theme.id)}>{theme.id}</button>
      ))}
      <button type="button" onClick={() => update({ palette: "ocean", glass: "solid", density: "compact", uiScale: 1.125, radiusScale: 0.5 })}>try others</button>
    </div>
  );
}

const mount = () => render(<ThemeProvider><Probe /></ThemeProvider>);
const root = document.documentElement;

const LOCKED = { palette: "cinema", glass: "full", density: "comfortable", motion: "system", radiusScale: 1, borderWidth: 1, uiScale: 1 };

beforeEach(() => {
  systemPrefersDark = true;
  mockMatchMedia();
  // Reset through the public API: unmounting does not reset Glass's shared store.
  const reset = render(<AppearanceProvider><ResetAppearance /></AppearanceProvider>);
  reset.unmount();
  window.localStorage.removeItem(LEGACY_THEME_KEY);
  setBlur(true);
  root.removeAttribute("data-theme");
  root.className = "";
});

afterEach(() => {
  cleanup();
  Object.defineProperty(window, "matchMedia", { configurable: true, writable: true, value: realMatchMedia });
  vi.restoreAllMocks();
});

describe("theme provider", () => {
  it("defaults to Cinema, dark and full glass, even when the system prefers light", () => {
    systemPrefersDark = false;
    mount();
    expect(screen.getByTestId("state")).toHaveTextContent("dark/dark/blur");
    expect(root).toHaveAttribute("data-mode", "dark");
    expect(root).toHaveAttribute("data-palette", "cinema");
    expect(root).toHaveAttribute("data-glass", "full");
    expect(root).toHaveAttribute("data-density", "comfortable");
    expect(root).not.toHaveAttribute("data-motion");
    expect(root).not.toHaveAttribute("data-glass-rendering");
    expect(root).toHaveAttribute("data-theme", "dark");
    expect(root).toHaveClass("dark");
    expect(root.style.colorScheme).toBe("dark");
  });

  it("switches to Light and back, keeping the palette", async () => {
    mount();
    await userEvent.click(screen.getByRole("button", { name: "light" }));
    expect(JSON.parse(window.localStorage.getItem(APPEARANCE_KEY)!)).toMatchObject({ mode: "light", palette: "cinema" });
    expect(window.localStorage.getItem(LEGACY_THEME_KEY)).toBe("light");
    expect(root).toHaveAttribute("data-mode", "light");
    expect(root).toHaveAttribute("data-palette", "cinema");
    expect(root).toHaveClass("light");
    expect(root).not.toHaveClass("dark");
    await userEvent.click(screen.getByRole("button", { name: "dark" }));
    expect(root).toHaveAttribute("data-mode", "dark");
  });

  it("only offers Dark and Light", () => {
    expect(THEMES.map((theme) => theme.id)).toEqual(["dark", "light"]);
  });

  it("ignores stored palettes, materials, density, motion and geometry", () => {
    const stored = { mode: "light", palette: "forest", glass: "solid", density: "compact", motion: "reduced", radiusScale: 0.4, borderWidth: 2, uiScale: 1.125, accentHue: 120 };
    window.localStorage.setItem(APPEARANCE_KEY, JSON.stringify(stored));
    mount();
    expect(screen.getByTestId("state")).toHaveTextContent("light/light/blur");
    expect(JSON.parse(screen.getByTestId("appearance").textContent!)).toMatchObject({ mode: "light", ...LOCKED, accentHue: null });
    expect(root).toHaveAttribute("data-palette", "cinema");
    expect(root).toHaveAttribute("data-glass", "full");
    expect(root).toHaveAttribute("data-density", "comfortable");
    expect(root).not.toHaveAttribute("data-motion");
    expect(root.style.getPropertyValue("--ui-scale")).toBe("");
    expect(root.style.getPropertyValue("--radius-scale")).toBe("");
    expect(root.style.getPropertyValue("--border-width")).toBe("");
    // The stored preference is reduced to what can still be chosen.
    expect(JSON.parse(window.localStorage.getItem(APPEARANCE_KEY)!)).toEqual({ mode: "light" });
  });

  it("cannot change the locked knobs at runtime", async () => {
    mount();
    await userEvent.click(screen.getByRole("button", { name: "try others" }));
    expect(JSON.parse(screen.getByTestId("appearance").textContent!)).toMatchObject(LOCKED);
    expect(root).toHaveAttribute("data-palette", "cinema");
    expect(root).toHaveAttribute("data-glass", "full");
  });

  it("turns a stored System mode into Dark", () => {
    systemPrefersDark = false;
    window.localStorage.setItem(APPEARANCE_KEY, JSON.stringify({ mode: "system", palette: "ember" }));
    mount();
    expect(screen.getByTestId("state")).toHaveTextContent("dark/dark/blur");
    expect(JSON.parse(window.localStorage.getItem(APPEARANCE_KEY)!)).toEqual({ mode: "dark" });
  });

  it.each([
    ["light", "light"],
    ["system", "dark"],
    ["midnight", "dark"],
    ["neon", "dark"],
  ] as const)("migrates the legacy %s theme to %s once", (legacy, mode) => {
    systemPrefersDark = false;
    window.localStorage.setItem(LEGACY_THEME_KEY, legacy);
    mount();
    expect(screen.getByTestId("state")).toHaveTextContent(`${mode}/${mode}/blur`);
    expect(root).toHaveAttribute("data-palette", "cinema");
    expect(JSON.parse(window.localStorage.getItem(APPEARANCE_KEY)!)).toEqual({ mode });
  });

  it("starts again from Dark when the stored appearance is unreadable", () => {
    window.localStorage.setItem(APPEARANCE_KEY, "invalid json");
    window.localStorage.setItem(LEGACY_THEME_KEY, "light");
    mount();
    expect(screen.getByTestId("state")).toHaveTextContent("dark/dark/blur");
    expect(JSON.parse(window.localStorage.getItem(APPEARANCE_KEY)!)).toEqual({ mode: "dark" });
  });

  it("leaves a clean preference untouched", () => {
    const store = { getItem: vi.fn(() => '{"mode":"light"}'), setItem: vi.fn(), removeItem: vi.fn() };
    migrateAppearance(store);
    expect(store.setItem).not.toHaveBeenCalled();
  });

  it("throws when used outside the provider", () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => render(<Probe />)).toThrow(/ThemeProvider/);
    consoleError.mockRestore();
  });
});

describe("blur", () => {
  it("is on by default and turns off for the whole device", async () => {
    render(<ThemeProvider><ThemePicker /><Probe /></ThemeProvider>);
    const toggle = screen.getByRole("switch", { name: "Blur" });
    expect(toggle).toBeChecked();
    await userEvent.click(toggle);
    expect(toggle).not.toBeChecked();
    expect(window.localStorage.getItem(BLUR_KEY)).toBe("off");
    expect(root).toHaveAttribute("data-glass-rendering", "none");
    expect(screen.getByTestId("state")).toHaveTextContent("dark/dark/no-blur");
    // The appearance preference itself carries nothing about blur.
    expect(window.localStorage.getItem(APPEARANCE_KEY) ?? "").not.toMatch(/blur|Rendering/i);
    await userEvent.click(toggle);
    expect(window.localStorage.getItem(BLUR_KEY)).toBeNull();
    expect(root).not.toHaveAttribute("data-glass-rendering");
  });

  it("follows a change made in another tab", () => {
    mount();
    act(() => {
      window.localStorage.setItem(BLUR_KEY, "off");
      window.dispatchEvent(new StorageEvent("storage", { key: BLUR_KEY, newValue: "off" }));
    });
    expect(screen.getByTestId("state")).toHaveTextContent("dark/dark/no-blur");
    expect(root).toHaveAttribute("data-glass-rendering", "none");
  });

  it("keeps blur off when appearance is applied again", async () => {
    mount();
    act(() => setBlur(false));
    await userEvent.click(screen.getByRole("button", { name: "light" }));
    expect(root).toHaveAttribute("data-glass-rendering", "none");
  });

  it("follows the device switch after a colour change stored blur in the appearance (Glass 0.19.4+)", async () => {
    mount();
    // Glass's update() stores the whole appearance, its blur included.
    await userEvent.click(screen.getByRole("button", { name: "light" }));
    expect(window.localStorage.getItem(APPEARANCE_KEY)).toMatch(/"glassRendering":"live"/);
    act(() => setBlur(false));
    await userEvent.click(screen.getByRole("button", { name: "dark" }));
    expect(root).toHaveAttribute("data-glass-rendering", "none");
    expect(screen.getByTestId("appearance")).toHaveTextContent('"glassRendering":"none"');
    act(() => setBlur(true));
    await userEvent.click(screen.getByRole("button", { name: "light" }));
    expect(root).not.toHaveAttribute("data-glass-rendering");
  });

  it("offers only colour mode, blur and animated backgrounds (display size is the TV app's)", () => {
    render(<ThemeProvider><ThemePicker /></ThemeProvider>);
    expect(screen.getByRole("switch", { name: "Animated backgrounds" })).toBeChecked();
    expect(screen.queryByText("Display size")).toBeNull();
    expect(screen.getByRole("button", { name: "Dark" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Light" })).toHaveAttribute("aria-pressed", "false");
    expect(screen.queryByRole("button", { name: "System" })).toBeNull();
    for (const name of [/palette/i, /density/i, /motion/i, /corner radius/i, /border width/i, /interface size/i]) {
      expect(screen.queryByText(name)).toBeNull();
    }
  });
});
