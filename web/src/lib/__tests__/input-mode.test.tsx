import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Select } from "glass-ux/select";
import { Dialog } from "glass-ux/dialog";
import { Segmented } from "glass-ux/segmented";
import { afterEach, describe, expect, it, vi } from "vitest";

import { GlassSegmented } from "@/components/ui/GlassControls";

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

function GlassSpatialFixture({ overlay }: { overlay: boolean }) {
  useSpatialNavigation();
  return (
    <main>
      <button type="button" data-testid="page">Page action</button>
      {overlay ? (
        <Dialog.Root defaultOpen>
          <Dialog.Popup>
            <Dialog.Header title="Choose an action" close={false} />
            <Dialog.Body>
              <Select aria-label="Library" defaultValue="movies" options={[{ value: "movies", label: "Movies" }, { value: "shows", label: "Shows" }]} />
              <Segmented aria-label="Overlay view" defaultValue="list" collapse="none" options={[{ value: "list", label: "List" }, { value: "grid", label: "Grid" }]} />
              <button type="button" data-testid="overlay-left">First action</button>
              <button type="button" data-testid="overlay-right">Second action</button>
            </Dialog.Body>
          </Dialog.Popup>
        </Dialog.Root>
      ) : (
        <GlassSegmented ariaLabel="View" value="list" onChange={() => {}} items={[{ key: "list", label: "List" }, { key: "grid", label: "Grid" }]} />
      )}
    </main>
  );
}

function position(testId: string, rect: { x: number; y: number }) {
  const element = screen.getByTestId(testId);
  element.getBoundingClientRect = () => ({ x: rect.x, y: rect.y, left: rect.x, top: rect.y, right: rect.x + 40, bottom: rect.y + 20, width: 40, height: 20, toJSON: () => ({}) }) as DOMRect;
  return element;
}

describe("spatial navigation", () => {
  it("moves between ordinary Glass dialog buttons without leaving the modal in TV mode", async () => {
    window.localStorage.setItem("librarian.inputMode", "tv");
    render(<InputModeProvider><GlassSpatialFixture overlay /></InputModeProvider>);
    const dialog = await screen.findByRole("dialog", { name: "Choose an action" });
    expect(dialog.closest("[data-base-ui-portal]")).not.toBeNull();
    expect(document.documentElement).toHaveAttribute("data-input-mode", "tv");
    const left = position("overlay-left", { x: 0, y: 0 });
    const right = position("overlay-right", { x: 100, y: 0 });
    position("page", { x: 200, y: 0 });
    left.focus();
    const arrow = new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true });
    fireEvent(left, arrow);
    expect(document.activeElement).toBe(right);
    expect(arrow.defaultPrevented).toBe(true);
    fireEvent.keyDown(right, { key: "ArrowRight" });
    expect(document.activeElement).toBe(right);
    fireEvent.keyDown(right, { key: "ArrowLeft" });
    expect(document.activeElement).toBe(left);
  });

  it("lets a Glass Select open its listbox from a modal with ArrowDown", async () => {
    render(<InputModeProvider><GlassSpatialFixture overlay /></InputModeProvider>);
    await screen.findByRole("dialog", { name: "Choose an action" });
    const trigger = screen.getByRole("combobox", { name: /Library/ });
    expect(trigger).toHaveAttribute("aria-haspopup", "listbox");
    trigger.getBoundingClientRect = () => ({ x: 0, y: 0, left: 0, top: 0, right: 100, bottom: 40, width: 100, height: 40, toJSON: () => ({}) }) as DOMRect;
    position("overlay-left", { x: 0, y: 100 });
    position("overlay-right", { x: 100, y: 100 });
    trigger.focus();
    await userEvent.keyboard("{ArrowDown}");
    expect(await screen.findByRole("listbox")).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await vi.waitFor(() => expect(trigger).toHaveFocus());
  });

  it("leaves a bare Glass composite inside a modal in charge of its arrows", async () => {
    render(<InputModeProvider><GlassSpatialFixture overlay /></InputModeProvider>);
    await screen.findByRole("dialog", { name: "Choose an action" });
    const list = screen.getByRole("button", { name: "List" }), grid = screen.getByRole("button", { name: "Grid" });
    list.getBoundingClientRect = () => ({ x: 0, y: 0, left: 0, top: 0, right: 40, bottom: 20, width: 40, height: 20, toJSON: () => ({}) }) as DOMRect;
    grid.getBoundingClientRect = () => ({ x: 200, y: 0, left: 200, top: 0, right: 240, bottom: 20, width: 40, height: 20, toJSON: () => ({}) }) as DOMRect;
    // This ordinary modal button is closer; composite focus must still choose Grid.
    position("overlay-left", { x: 100, y: 0 });
    position("overlay-right", { x: 300, y: 0 });
    list.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(grid).toHaveFocus();
  });

  it("lets Glass Segmented own roving focus in TV mode", async () => {
    window.localStorage.setItem("librarian.inputMode", "tv");
    render(<InputModeProvider><GlassSpatialFixture overlay={false} /></InputModeProvider>);
    const list = screen.getByRole("button", { name: "List" });
    const grid = screen.getByRole("button", { name: "Grid" });
    list.getBoundingClientRect = () => ({ x: 0, y: 0, left: 0, top: 0, right: 40, bottom: 20, width: 40, height: 20, toJSON: () => ({}) }) as DOMRect;
    grid.getBoundingClientRect = () => ({ x: 200, y: 0, left: 200, top: 0, right: 240, bottom: 20, width: 40, height: 20, toJSON: () => ({}) }) as DOMRect;
    // A page action is nearer in the same direction; only the composite should choose Grid.
    position("page", { x: 100, y: 0 });
    list.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(document.activeElement).toBe(grid);
    expect(list).toHaveAttribute("aria-pressed", "true");
  });

  it("moves focus to the nearest element in the pressed direction", () => {
    window.localStorage.setItem("librarian.inputMode", "tv");
    render(<InputModeProvider><SpatialFixture /></InputModeProvider>);
    expect(document.documentElement).toHaveAttribute("data-input-mode", "tv");
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

function place(testId: string, rect: { x: number; y: number; w?: number; h?: number }) {
  const element = screen.getByTestId(testId);
  const width = rect.w ?? 40, height = rect.h ?? 20;
  element.getBoundingClientRect = () => ({ x: rect.x, y: rect.y, left: rect.x, top: rect.y, right: rect.x + width, bottom: rect.y + height, width, height, toJSON: () => ({}) }) as DOMRect;
  return element;
}

function TvFixture({ onBack }: { onBack?: () => void }) {
  useSpatialNavigation(true, true, onBack);
  return (
    <>
      <aside data-nav-zone="rail" data-nav-side="">
        <a href="/home" data-testid="rail-home">Home</a>
        <a href="/libraries" data-testid="rail-libraries">Libraries</a>
      </aside>
      <main>
        <section data-nav-section>
          <button type="button" data-testid="play">Play</button>
          <input data-testid="field" defaultValue="Dune" />
          <button type="button" aria-haspopup="menu" data-testid="more">More</button>
        </section>
        <section data-nav-section>
          <span data-tv-skip><a href="/all" data-testid="see-all">See all</a></span>
          <div data-nav-group>
            <a href="/m/1" data-testid="poster-1">One</a>
            <a href="/m/2" data-testid="poster-2">Two</a>
            <a href="/m/3" data-testid="poster-3">Three</a>
          </div>
        </section>
        <section data-nav-section>
          <div data-nav-group>
            <a href="/s/1" data-testid="show-1">Show</a>
          </div>
        </section>
        <a href="/far" data-testid="loose">Loose link</a>
        <div role="tablist" aria-label="Seasons">
          <button type="button" role="tab" data-testid="tab">Season 1</button>
        </div>
      </main>
    </>
  );
}

describe("remote focus", () => {
  function layout() {
    place("rail-home", { x: 0, y: 100 });
    place("rail-libraries", { x: 0, y: 400 });
    place("play", { x: 200, y: 100 });
    place("field", { x: 300, y: 100, w: 120 });
    place("more", { x: 460, y: 100 });
    place("see-all", { x: 900, y: 180 });
    place("poster-1", { x: 200, y: 220, w: 100, h: 150 });
    place("poster-2", { x: 320, y: 220, w: 100, h: 150 });
    place("poster-3", { x: 440, y: 220, w: 100, h: 150 });
    place("show-1", { x: 200, y: 420, w: 100, h: 150 });
    place("loose", { x: 700, y: 380 });
    place("tab", { x: 200, y: 700 });
  }
  const press = (key: string) => fireEvent.keyDown(document.activeElement ?? window, { key, bubbles: true });

  it("moves to the nearest row first and skips header shortcuts", () => {
    window.localStorage.setItem("librarian.inputMode", "tv");
    render(<InputModeProvider><TvFixture /></InputModeProvider>);
    layout();
    screen.getByTestId("play").focus();
    press("ArrowDown");
    // The poster row, not the closer "See all" link in its heading.
    expect(document.activeElement).toBe(screen.getByTestId("poster-1"));
  });

  it("returns to the remembered card when coming back to a row", () => {
    render(<InputModeProvider><TvFixture /></InputModeProvider>);
    layout();
    screen.getByTestId("poster-3").focus();
    press("ArrowDown");
    // The next section wins over a loose link that is nearer to the right.
    expect(document.activeElement).toBe(screen.getByTestId("show-1"));
    press("ArrowUp");
    expect(document.activeElement).toBe(screen.getByTestId("poster-3"));
  });

  it("keeps the rail out of vertical moves and returns to content from it", () => {
    render(<InputModeProvider><TvFixture /></InputModeProvider>);
    layout();
    screen.getByTestId("poster-1").focus();
    press("ArrowLeft");
    expect(document.activeElement?.closest("[data-nav-zone]")).toHaveAttribute("data-nav-zone", "rail");
    press("ArrowDown");
    expect(document.activeElement).toBe(screen.getByTestId("rail-libraries"));
    press("ArrowRight");
    expect(document.activeElement).toBe(screen.getByTestId("poster-1"));
  });

  it("does nothing at the end of a row instead of jumping elsewhere", () => {
    render(<InputModeProvider><TvFixture /></InputModeProvider>);
    layout();
    screen.getByTestId("poster-3").focus();
    press("ArrowRight");
    expect(document.activeElement).toBe(screen.getByTestId("poster-3"));
  });

  it("lets the remote leave a single-line field in TV mode", () => {
    window.localStorage.setItem("librarian.inputMode", "tv");
    render(<InputModeProvider><TvFixture /></InputModeProvider>);
    layout();
    const field = screen.getByTestId("field") as HTMLInputElement;
    field.focus();
    field.setSelectionRange(2, 2);
    press("ArrowRight");
    expect(document.activeElement).toBe(field);
    field.setSelectionRange(4, 4);
    press("ArrowRight");
    expect(document.activeElement).toBe(screen.getByTestId("more"));
    field.focus();
    press("ArrowDown");
    expect(document.activeElement).not.toBe(field);
  });

  it("lets Up and Down leave a horizontal tab strip", () => {
    render(<InputModeProvider><TvFixture /></InputModeProvider>);
    layout();
    screen.getByTestId("tab").focus();
    press("ArrowLeft");
    expect(document.activeElement).toBe(screen.getByTestId("tab"));
    press("ArrowUp");
    expect(document.activeElement).not.toBe(screen.getByTestId("tab"));
  });

  it("moves past a menu button on a TV instead of opening it", () => {
    window.localStorage.setItem("librarian.inputMode", "tv");
    render(<InputModeProvider><TvFixture /></InputModeProvider>);
    layout();
    const more = screen.getByTestId("more");
    const opened = vi.fn();
    more.addEventListener("keydown", opened);
    more.focus();
    press("ArrowDown");
    expect(opened).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(screen.getByTestId("poster-3"));
  });

  it("treats Backspace as the remote's Back in TV mode, except while editing or in an overlay", () => {
    window.localStorage.setItem("librarian.inputMode", "tv");
    const onBack = vi.fn();
    render(<InputModeProvider><TvFixture onBack={onBack} /></InputModeProvider>);
    layout();
    screen.getByTestId("play").focus();
    press("Backspace");
    expect(onBack).toHaveBeenCalledOnce();
    screen.getByTestId("field").focus();
    press("Backspace");
    expect(onBack).toHaveBeenCalledOnce();
    fireEvent.keyDown(screen.getByTestId("field"), { key: "GoBack", keyCode: 461 });
    expect(onBack).toHaveBeenCalledTimes(2);
  });
});
