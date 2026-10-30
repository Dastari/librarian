import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { Artwork } from "../Artwork";
import { BackdropCard } from "../BackdropCard";
import { MediaRow } from "../MediaRow";
import { PosterCard } from "../PosterCard";

afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); });

/** jsdom cannot apply Tailwind; catch the classes that originally hid usable controls. */
function expectPermanentControl(element: HTMLElement) {
  for (let current: HTMLElement | null = element; current; current = current.parentElement) {
    expect(current.hidden).toBe(false);
    expect(current.getAttribute("aria-hidden")).not.toBe("true");
    expect(current.className.split(/\s+/)).not.toEqual(expect.arrayContaining([expect.stringMatching(/^(?:hidden|invisible|opacity-0)$/)]));
  }
}

describe("PosterCard interaction regressions", () => {
  it("renders a real named link with playback and extra actions outside that link", async () => {
    const onPlay = vi.fn();
    const onAction = vi.fn();
    const { router } = await renderWithProviders(
      <PosterCard title="Andor" image={null} to="/shows/$showId" params={{ showId: "andor" }} onPlay={onPlay}
        actions={<button type="button" onClick={onAction}>Queue Andor</button>} />,
      { routes: ["/shows/$showId"] },
    );
    const link = screen.getByRole("link", { name: "Andor" });
    const play = screen.getByRole("button", { name: "Play Andor" });
    const action = screen.getByRole("button", { name: "Queue Andor" });
    expect(link.tagName).toBe("A");
    expect(link).toHaveAttribute("href", "/shows/andor");
    expect(link).not.toContainElement(play);
    expect(link).not.toContainElement(action);
    expect(play.closest("a, button button, [role=button] [role=button]")).toBeNull();
    expectPermanentControl(play);
    expectPermanentControl(action);
    play.focus();
    await userEvent.keyboard("{Enter} ");
    expect(onPlay).toHaveBeenCalledTimes(2);
    await userEvent.click(action);
    expect(onAction).toHaveBeenCalledOnce();
    expect(router!.state.location.pathname).toBe("/");
    await userEvent.click(link);
    expect(router!.state.location.pathname).toBe("/shows/andor");
  });

  it("keeps keyboard playback independent from an onPress card target", async () => {
    const onPress = vi.fn();
    const onPlay = vi.fn();
    await renderWithProviders(<PosterCard title="Dune" image={null} onPress={onPress} onPlay={onPlay} />);
    const card = screen.getByRole("button", { name: "Dune" });
    const play = screen.getByRole("button", { name: "Play Dune" });
    expect(card).not.toContainElement(play);
    play.focus();
    await userEvent.keyboard("{Enter} ");
    expect(onPlay).toHaveBeenCalledTimes(2);
    expect(onPress).not.toHaveBeenCalled();
    card.focus();
    await userEvent.keyboard("{Enter}");
    expect(onPress).toHaveBeenCalledOnce();
  });

  it("keeps a passive card out of the tab order and button/link roles", async () => {
    await renderWithProviders(<PosterCard title="Informational poster" image={null} />);
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.queryByRole("link")).toBeNull();
    expect(screen.getByText("Informational poster").closest("[tabindex], [data-focusable]")).toBeNull();
  });

  it.each([0.42, 2])("exposes watched progress %s through the actual accessible progress control", async (progress) => {
    await renderWithProviders(<PosterCard title="Dune" image={null} progress={progress} />);
    const meter = screen.getByRole("progressbar", { name: "Watched progress for Dune" });
    expect(meter).toHaveAttribute("aria-valuemin", "0");
    expect(meter).toHaveAttribute("aria-valuemax", "1");
    expect(meter).toHaveAttribute("aria-valuenow", String(Math.min(progress, 1)));
  });

  it.each([0, -1, undefined])("does not announce progress when the value is %s", async (progress) => {
    await renderWithProviders(<PosterCard title="Dune" image={null} progress={progress} />);
    expect(screen.queryByRole("progressbar")).toBeNull();
  });
});

describe("BackdropCard", () => {
  it("provides keyboard activation, persistent playback affordance and accessible progress", async () => {
    const onPress = vi.fn();
    await renderWithProviders(<BackdropCard title="Continue Andor" subtitle="S01E03" meta="23 minutes left" image={null} progress={0.7} onPress={onPress} corner="HD" />);
    const button = screen.getByRole("button", { name: /Continue Andor/ });
    expectPermanentControl(button);
    expect(screen.getByText("HD")).toBeInTheDocument();
    expect(screen.getByText("23 minutes left")).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Watched progress for Continue Andor" })).toHaveAttribute("aria-valuenow", "0.7");
    button.focus();
    await userEvent.keyboard("{Enter} ");
    expect(onPress).toHaveBeenCalledTimes(2);
  });

  it("uses one real link without a nested button when given a destination", async () => {
    const onPress = vi.fn();
    const { router } = await renderWithProviders(<BackdropCard title="Andor" image={null} to="/shows/$showId" params={{ showId: "andor" }} onPress={onPress} />, { routes: ["/shows/$showId"] });
    const link = screen.getByRole("link", { name: "Andor" });
    expect(link).toHaveAttribute("href", "/shows/andor");
    expect(screen.queryByRole("button")).toBeNull();
    await userEvent.click(link);
    expect(router!.state.location.pathname).toBe("/shows/andor");
    expect(onPress).not.toHaveBeenCalled();
  });
});

describe("Artwork", () => {
  it("replaces a failed image with its fallback, then recovers when its source changes", () => {
    const { rerender } = render(<Artwork src="/broken.jpg" alt="Andor artwork" fallback={<span>Artwork unavailable</span>} />);
    fireEvent.error(screen.getByRole("img", { name: "Andor artwork" }));
    expect(screen.queryByRole("img")).toBeNull();
    expect(screen.getByText("Artwork unavailable")).toBeInTheDocument();
    rerender(<Artwork src="/recovered.jpg" alt="Andor artwork" fallback={<span>Artwork unavailable</span>} />);
    const image = screen.getByRole("img", { name: "Andor artwork" });
    expect(image).toHaveAttribute("src", "/recovered.jpg");
    expect(screen.queryByText("Artwork unavailable")).toBeNull();
    fireEvent.load(image);
    expect(image).toHaveClass("opacity-100");
  });

  it("renders missing artwork without a broken image and preserves priority hints", () => {
    const { rerender } = render(<Artwork src={null} alt="Cover" fallback={<span>Cover unavailable</span>} />);
    expect(screen.queryByRole("img")).toBeNull();
    expect(screen.getByText("Cover unavailable")).toBeInTheDocument();
    rerender(<Artwork src="/cover.jpg" alt="Cover" priority sizes="200px" />);
    const image = screen.getByRole("img", { name: "Cover" });
    expect(image).toHaveAttribute("loading", "eager");
    expect(image).toHaveAttribute("fetchpriority", "high");
    expect(image).toHaveAttribute("sizes", "200px");
    expect(image).toHaveAttribute("draggable", "false");
  });
});

describe("MediaRow", () => {
  it("keeps usable arrows visible, scrolls in both directions and updates at scroll boundaries", async () => {
    let scrollLeft = 0;
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(500);
    vi.spyOn(HTMLElement.prototype, "scrollWidth", "get").mockReturnValue(1200);
    vi.spyOn(HTMLElement.prototype, "scrollLeft", "get").mockImplementation(() => scrollLeft);
    const scrollBy = vi.fn();
    await renderWithProviders(<MediaRow ariaLabel="Recent movies"><span role="listitem">Dune</span></MediaRow>);
    const list = screen.getByRole("list", { name: "Recent movies" });
    Object.defineProperty(list, "scrollBy", { configurable: true, value: scrollBy });
    const previous = screen.getByRole("button", { name: "Previous items", hidden: true });
    const next = screen.getByRole("button", { name: "Next items" });
    expect(previous).toBeDisabled();
    expect(next).toBeEnabled();
    expectPermanentControl(next);
    next.focus();
    await userEvent.keyboard("{Enter}");
    expect(scrollBy).toHaveBeenLastCalledWith({ left: 425, behavior: "smooth" });
    scrollLeft = 350;
    fireEvent.scroll(list);
    expect(previous).toBeEnabled();
    expectPermanentControl(previous);
    await userEvent.click(previous);
    expect(scrollBy).toHaveBeenLastCalledWith({ left: -425, behavior: "smooth" });
    scrollLeft = 700;
    fireEvent.scroll(list);
    expect(next).toBeDisabled();
    expect(previous).toBeEnabled();
  });

  it("recomputes overflow when its measured size changes and disconnects on unmount", async () => {
    let contentWidth = 500;
    let resized: () => void = () => {};
    const disconnect = vi.fn();
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(500);
    vi.spyOn(HTMLElement.prototype, "scrollWidth", "get").mockImplementation(() => contentWidth);
    vi.stubGlobal("ResizeObserver", class {
      constructor(callback: () => void) { resized = callback; }
      observe() {}
      disconnect = disconnect;
    });
    const { unmount } = await renderWithProviders(<MediaRow ariaLabel="Movies"><span role="listitem">Dune</span></MediaRow>);
    const next = screen.getByRole("button", { name: "Next items", hidden: true });
    expect(next).toBeDisabled();
    contentWidth = 900;
    act(() => resized());
    expect(next).toBeEnabled();
    expectPermanentControl(next);
    unmount();
    expect(disconnect).toHaveBeenCalled();
  });
});
