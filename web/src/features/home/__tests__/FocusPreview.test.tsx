import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MediaBackdrop, PosterCard } from "@/components/ui";
import { renderWithProviders } from "@/test";

import { HomeSpotlight, movieSpotlight, type SpotlightArt } from "../HomeSpotlight";
import { useFocusPreview } from "../useFocusPreview";

const movie = (id: string, title: string) => ({
  id, title, year: 1992, runtime: 117, certification: "R", genres: ["Thriller"], overview: `${title} overview`, mediaFileId: `file-${id}`,
});
const hunt = movie("hunt", "The Hunt for Red October");
const patriot = movie("patriot", "Patriot Games");
const clear = movie("clear", "Clear and Present Danger");

function Page({ onPlay, onArt }: { onPlay: (id: string) => void; onArt: (art: SpotlightArt) => void }) {
  const { preview, previewOnFocus, clearPreview } = useFocusPreview();
  return (
    <>
      <HomeSpotlight items={[movieSpotlight(hunt), movieSpotlight(clear)]} preview={preview} onBrowse={clearPreview} fill="library" poster={false} onArtChange={onArt} onPlay={(item) => onPlay(item.id)} />
      <div role="list" aria-label="Recently added">
        {[hunt, patriot].map((item) => (
          <PosterCard key={item.id} width="row" title={item.title} image={null} onPress={() => {}} onFocus={previewOnFocus(movieSpotlight(item))} />
        ))}
      </div>
    </>
  );
}

// Remote navigation; the shell's input-mode provider sets this on <html>.
beforeEach(() => {
  document.documentElement.dataset.inputMode = "tv";
  vi.useFakeTimers({ shouldAdvanceTime: true });
});
afterEach(() => {
  vi.useRealTimers();
  delete document.documentElement.dataset.inputMode;
});

const card = (title: string) => screen.getByRole("button", { name: new RegExp(title) });
const settle = () => act(() => vi.advanceTimersByTime(300));

describe("focus preview", () => {
  it("shows the focused shelf card in the spotlight and its art behind the page", async () => {
    const onPlay = vi.fn();
    const onArt = vi.fn();
    await renderWithProviders(<Page onPlay={onPlay} onArt={onArt} />, { routes: ["/movies/$movieId"] });
    expect(screen.getByRole("heading", { name: hunt.title })).toBeInTheDocument();

    act(() => card(patriot.title).focus());
    await settle();
    expect(screen.getByRole("heading", { name: patriot.title })).toBeInTheDocument();
    expect(onArt).toHaveBeenLastCalledWith(expect.objectContaining({ src: expect.stringContaining("/movie/patriot/backdrop") }));

    // Moving up to the spotlight keeps the chosen title, so Play plays it.
    fireEvent.click(screen.getByRole("button", { name: "Play" }));
    expect(onPlay).toHaveBeenCalledWith("patriot");
  });

  it("waits for focus to settle and ignores a card focus has already left", async () => {
    await renderWithProviders(<Page onPlay={() => {}} onArt={() => {}} />, { routes: ["/movies/$movieId"] });
    act(() => card(patriot.title).focus());
    act(() => card(hunt.title).focus());
    await settle();
    expect(screen.getByRole("heading", { name: hunt.title })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: patriot.title })).toBeNull();

    act(() => card(patriot.title).focus());
    act(() => screen.getByRole("button", { name: "Play" }).focus());
    await settle();
    expect(screen.getByRole("heading", { name: hunt.title })).toBeInTheDocument();
  });

  it("returns to the carousel when it is paged, keeping focus on the arrow", async () => {
    await renderWithProviders(<Page onPlay={() => {}} onArt={() => {}} />, { routes: ["/movies/$movieId"] });
    act(() => card(patriot.title).focus());
    await settle();
    expect(screen.getByLabelText("Choose a title")).not.toHaveTextContent("1 / 2");

    fireEvent.click(screen.getByRole("button", { name: "Next title" }));
    expect(screen.getByRole("heading", { name: hunt.title })).toBeInTheDocument();
    expect(screen.getByLabelText("Choose a title")).toHaveTextContent("1 / 2");
    expect(screen.getByRole("button", { name: "Next title" })).toHaveFocus();

    fireEvent.click(screen.getByRole("button", { name: "Next title" }));
    expect(screen.getByRole("heading", { name: clear.title })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Next title" })).toHaveFocus();
  });

  it("does not preview on pointer clicks", async () => {
    document.documentElement.dataset.inputMode = "pointer";
    await renderWithProviders(<Page onPlay={() => {}} onArt={() => {}} />, { routes: ["/movies/$movieId"] });
    fireEvent.focus(card(patriot.title));
    await settle();
    expect(screen.getByRole("heading", { name: hunt.title })).toBeInTheDocument();
  });
});

describe("MediaBackdrop", () => {
  it("keeps the last picture underneath while the next one loads", async () => {
    const { rerender } = render(<MediaBackdrop src="/api/artwork/movie/hunt/backdrop" tint="red" />);
    const first = document.querySelector<HTMLImageElement>(".media-backdrop-image")!;
    fireEvent.load(first);
    expect(first).toHaveAttribute("data-loaded", "true");

    rerender(<MediaBackdrop src="/api/artwork/movie/patriot/backdrop" tint="red" />);
    const layers = [...document.querySelectorAll<HTMLImageElement>(".media-backdrop-image")];
    expect(layers.map((layer) => layer.getAttribute("src"))).toEqual([expect.stringContaining("/hunt/"), expect.stringContaining("/patriot/")]);
    // The previous picture is the same element, still shown, under the incoming one.
    expect(layers[0]).toBe(first);
    expect(layers[0]).toHaveAttribute("data-loaded", "true");
    expect(layers[1]).not.toHaveAttribute("data-loaded");

    rerender(<MediaBackdrop src={null} fallback="/api/artwork/movie/x/poster" tint="red" />);
    expect(document.querySelectorAll(".media-backdrop-image")).toHaveLength(0);
  });
});
