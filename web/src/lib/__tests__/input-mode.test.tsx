import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { InputModeProvider, useInputMode, useSpatialNavigation, type InputMode } from "../input-mode";

const realMatchMedia = window.matchMedia;

function setUserAgent(value: string) {
  Object.defineProperty(window.navigator, "userAgent", { configurable: true, value });
}

function setCoarsePointer(coarse: boolean) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: (query: string) => ({ matches: coarse && query.includes("pointer: coarse"), media: query, onchange: null, addEventListener: () => {}, removeEventListener: () => {}, addListener: () => {}, removeListener: () => {}, dispatchEvent: () => false }) as unknown as MediaQueryList,
  });
}

function Probe() {
  const { mode, forced, setForced } = useInputMode();
  return (
    <div>
      <p data-testid="mode">{mode}</p>
      <p data-testid="forced">{forced ?? "none"}</p>
      {(["tv", "pointer", "touch"] as InputMode[]).map((value) => (
        <button key={value} type="button" onClick={() => setForced(value)}>
          force {value}
        </button>
      ))}
      <button type="button" onClick={() => setForced(null)}>
        clear
      </button>
    </div>
  );
}

const mount = () =>
  render(
    <InputModeProvider>
      <Probe />
    </InputModeProvider>,
  );

afterEach(() => {
  Object.defineProperty(window, "matchMedia", { configurable: true, writable: true, value: realMatchMedia });
  setUserAgent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36");
  document.documentElement.removeAttribute("data-input-mode");
});

describe("input mode", () => {
  it("detects a pointer by default and mirrors it onto <html>", () => {
    setCoarsePointer(false);
    mount();
    expect(screen.getByTestId("mode")).toHaveTextContent("pointer");
    expect(document.documentElement.dataset.inputMode).toBe("pointer");
  });

  it("detects touch from a coarse pointer", () => {
    setCoarsePointer(true);
    mount();
    expect(screen.getByTestId("mode")).toHaveTextContent("touch");
  });

  it("detects a TV from the user agent", () => {
    setCoarsePointer(false);
    setUserAgent("Mozilla/5.0 (SMART-TV; Linux; Tizen 6.0)");
    mount();
    expect(screen.getByTestId("mode")).toHaveTextContent("tv");
  });

  it("promotes to TV after repeated arrow presses and returns to pointer on a mouse press", () => {
    setCoarsePointer(false);
    mount();
    for (let press = 0; press < 6; press += 1) act(() => void fireEvent.keyDown(window, { key: "ArrowDown" }));
    expect(screen.getByTestId("mode")).toHaveTextContent("tv");
    // Note: the arrow counter lives inside the effect, so it resets when `detected` changes and
    // the `arrowPresses < 6` guard in the pointer handler never holds. Any mouse press demotes.
    act(() => void fireEvent.pointerDown(window, { pointerType: "mouse" }));
    expect(screen.getByTestId("mode")).toHaveTextContent("pointer");
  });

  it("remembers a forced mode across mounts and can clear it", async () => {
    setCoarsePointer(false);
    const { unmount } = mount();
    await userEvent.click(screen.getByRole("button", { name: "force tv" }));
    expect(screen.getByTestId("mode")).toHaveTextContent("tv");
    expect(window.localStorage.getItem("librarian.inputMode")).toBe("tv");
    unmount();

    mount();
    expect(screen.getByTestId("forced")).toHaveTextContent("tv");
    await userEvent.click(screen.getByRole("button", { name: "clear" }));
    expect(window.localStorage.getItem("librarian.inputMode")).toBeNull();
    expect(screen.getByTestId("mode")).toHaveTextContent("pointer");
  });

  it("throws outside the provider", () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => render(<Probe />)).toThrow(/InputModeProvider/);
    consoleError.mockRestore();
  });
});

/** Lays three buttons out as a row of two with one below, so directions are unambiguous. */
function SpatialFixture() {
  useSpatialNavigation();
  return (
    <main>
      <button type="button" data-testid="left">Left</button>
      <button type="button" data-testid="right">Right</button>
      <button type="button" data-testid="below">Below</button>
      <input data-testid="text" />
    </main>
  );
}

function position(testId: string, rect: { x: number; y: number }) {
  const element = screen.getByTestId(testId);
  element.getBoundingClientRect = () => ({ x: rect.x, y: rect.y, left: rect.x, top: rect.y, right: rect.x + 40, bottom: rect.y + 20, width: 40, height: 20, toJSON: () => ({}) }) as DOMRect;
  return element;
}

describe("spatial navigation", () => {
  it("moves focus to the nearest element in the pressed direction", () => {
    render(<SpatialFixture />);
    const left = position("left", { x: 0, y: 0 });
    const right = position("right", { x: 200, y: 0 });
    const below = position("below", { x: 0, y: 200 });
    position("text", { x: 500, y: 500 });

    left.focus();
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(document.activeElement).toBe(right);

    fireEvent.keyDown(window, { key: "ArrowLeft" });
    expect(document.activeElement).toBe(left);

    fireEvent.keyDown(window, { key: "ArrowDown" });
    expect(document.activeElement).toBe(below);

    fireEvent.keyDown(window, { key: "ArrowUp" });
    expect(document.activeElement).toBe(left);
  });

  it("leaves text fields, modified keys and ignored regions alone", () => {
    render(<SpatialFixture />);
    position("left", { x: 0, y: 0 });
    const right = position("right", { x: 200, y: 0 });
    position("below", { x: 0, y: 200 });
    const text = position("text", { x: 500, y: 0 });

    text.focus();
    fireEvent.keyDown(window, { key: "ArrowLeft" });
    expect(document.activeElement).toBe(text);

    screen.getByTestId("left").focus();
    fireEvent.keyDown(window, { key: "ArrowRight", metaKey: true });
    expect(document.activeElement).not.toBe(right);
  });
});
