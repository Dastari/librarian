import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { THEMES, ThemeProvider, useTheme } from "../theme";

let listeners: Array<(event: MediaQueryListEvent) => void> = [];
let systemPrefersLight = false;

const realMatchMedia = window.matchMedia;

function mockMatchMedia() {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: (query: string) =>
      ({
        matches: query.includes("light") ? systemPrefersLight : false,
        media: query,
        onchange: null,
        addEventListener: (_: string, listener: (event: MediaQueryListEvent) => void) => listeners.push(listener),
        removeEventListener: (_: string, listener: (event: MediaQueryListEvent) => void) => {
          listeners = listeners.filter((item) => item !== listener);
        },
        addListener: () => {},
        removeListener: () => {},
        dispatchEvent: () => false,
      }) as unknown as MediaQueryList,
  });
}

function Probe() {
  const { preference, resolved, scheme, setPreference } = useTheme();
  return (
    <div>
      <p data-testid="state">{`${preference}/${resolved}/${scheme}`}</p>
      {[...THEMES.map((theme) => theme.id), "system" as const].map((id) => (
        <button key={id} type="button" onClick={() => setPreference(id)}>
          {id}
        </button>
      ))}
    </div>
  );
}

beforeEach(() => {
  systemPrefersLight = false;
  listeners = [];
  mockMatchMedia();
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.className = "";
});

afterEach(() => {
  Object.defineProperty(window, "matchMedia", { configurable: true, writable: true, value: realMatchMedia });
  vi.restoreAllMocks();
});

describe("theme provider", () => {
  it("defaults to Cinema and writes the theme onto <html>", () => {
    render(
      <ThemeProvider>
        <Probe />
      </ThemeProvider>,
    );
    expect(screen.getByTestId("state")).toHaveTextContent("dark/dark/dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(document.documentElement.style.colorScheme).toBe("dark");
  });

  it("persists the choice and applies the light class", async () => {
    render(
      <ThemeProvider>
        <Probe />
      </ThemeProvider>,
    );
    await userEvent.click(screen.getByRole("button", { name: "light" }));
    expect(window.localStorage.getItem("librarian.theme")).toBe("light");
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(document.documentElement.classList.contains("light")).toBe(true);
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("restores a stored preference and ignores an unknown one", () => {
    window.localStorage.setItem("librarian.theme", "forest");
    const { unmount } = render(
      <ThemeProvider>
        <Probe />
      </ThemeProvider>,
    );
    expect(screen.getByTestId("state")).toHaveTextContent("forest/forest/dark");
    unmount();
    window.localStorage.setItem("librarian.theme", "neon");
    render(
      <ThemeProvider>
        <Probe />
      </ThemeProvider>,
    );
    expect(screen.getByTestId("state")).toHaveTextContent("dark/dark/dark");
  });

  it("follows the OS when the preference is system", async () => {
    systemPrefersLight = true;
    render(
      <ThemeProvider>
        <Probe />
      </ThemeProvider>,
    );
    await userEvent.click(screen.getByRole("button", { name: "system" }));
    expect(screen.getByTestId("state")).toHaveTextContent("system/light/light");
  });

  it("throws when used outside the provider", () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => render(<Probe />)).toThrow(/ThemeProvider/);
    consoleError.mockRestore();
  });
});
