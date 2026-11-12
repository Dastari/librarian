import { fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { statusMeta } from "@/lib/status";
import { renderWithProviders } from "@/test";

import { PosterCard, PosterInTable } from "../PosterCard";

describe("PosterCard", () => {
  it("shows the title, meta line and artwork", async () => {
    await renderWithProviders(<PosterCard title="Andor" meta="2022 · 24 episodes" image="/api/artwork/show/s1/poster" />);
    expect(screen.getByText("Andor")).toBeInTheDocument();
    expect(screen.getByText("2022 · 24 episodes")).toBeInTheDocument();
    const image = document.querySelector("img");
    // Cached artwork is requested at a stored size close to the tile's width.
    expect(image?.getAttribute("src")).toMatch(/^\/api\/artwork\/show\/s1\/poster\?w=\d+$/);
    // Artwork is decorative; the title beneath the card names it.
    expect(image).toHaveAttribute("alt", "");
  });

  it("is a link when it has a route", async () => {
    await renderWithProviders(<PosterCard title="Andor" image={null} to="/shows/$showId" params={{ showId: "s1" }} />, { routes: ["/shows/$showId"] });
    expect(screen.getByRole("link")).toHaveAttribute("href", "/shows/s1");
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("is a press target with keyboard support when it has no route", async () => {
    const onPress = vi.fn();
    await renderWithProviders(<PosterCard title="Dune" image={null} onPress={onPress} />);
    const card = screen.getByRole("button", { name: /Dune/ });
    await userEvent.click(card);
    expect(onPress).toHaveBeenCalledTimes(1);

    card.focus();
    await userEvent.keyboard("{Enter}");
    await userEvent.keyboard(" ");
    expect(onPress).toHaveBeenCalledTimes(3);
  });

  it("renders the badge, status dot and progress bar", async () => {
    await renderWithProviders(<PosterCard title="Dune" image={null} badge="12/24" status={statusMeta("DOWNLOADING")} progress={0.42} />);
    expect(screen.getByText("12/24")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Downloading" })).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Watched progress for Dune" })).toHaveAttribute("aria-valuetext", "42%");
  });

  it("hides the progress bar when nothing has been watched", async () => {
    const { container } = await renderWithProviders(<PosterCard title="Dune" image={null} progress={0} />);
    expect(container.querySelector('[style*="width"]')).toBeNull();
  });

  it("offers play and extra actions as separate targets inside a link card", async () => {
    const onPlay = vi.fn();
    const onAdd = vi.fn();
    await renderWithProviders(
      <PosterCard
        title="Dune"
        image={null}
        to="/movies/$movieId"
        params={{ movieId: "m1" }}
        onPlay={onPlay}
        actions={
          <button type="button" aria-label="Add Dune to library" onClick={onAdd}>
            +
          </button>
        }
      />,
      { routes: ["/movies/$movieId"] },
    );
    await userEvent.click(screen.getByRole("button", { name: "Play Dune" }));
    expect(onPlay).toHaveBeenCalledOnce();
    await userEvent.click(screen.getByRole("button", { name: "Add Dune to library" }));
    expect(onAdd).toHaveBeenCalledOnce();
    // The card is still a single link; the overlays did not navigate.
    expect(screen.getByRole("link")).toHaveAttribute("href", "/movies/m1");
  });

  it("marks itself selected for the table's card view", async () => {
    await renderWithProviders(<PosterCard title="Dune" image={null} onPress={() => {}} selected />);
    const card = screen.getByRole("button", { name: /Dune/ }).closest("article");
    expect(card).toHaveAttribute("data-selected", "true");
    expect(card?.querySelector(".poster-art")).toBeTruthy();
  });

  it("isn't a focus stop of its own inside a table, whose card owns the arrow keys", async () => {
    await renderWithProviders(<PosterInTable.Provider value={true}><PosterCard title="Dune" image={null} onPress={() => {}} onPlay={() => {}} /></PosterInTable.Provider>);
    const open = screen.getByRole("button", { name: /^Dune/ });
    expect(open).toHaveAttribute("tabindex", "-1");
    expect(open).not.toHaveAttribute("data-focusable");
    expect(screen.getByRole("button", { name: "Play Dune" })).toHaveAttribute("tabindex", "-1");
  });

  it("keeps its play button out of the tab order and the arrow keys, so the card is one stop", async () => {
    const onPlay = vi.fn();
    await renderWithProviders(<PosterCard title="Dune" image={null} onPlay={onPlay} onPress={vi.fn()} />);
    const play = screen.getByRole("button", { name: "Play Dune" });
    expect(play).toHaveAttribute("tabindex", "-1");
    expect(play).toHaveAttribute("data-nav-skip");
    await userEvent.click(play);
    expect(onPlay).toHaveBeenCalledTimes(1);
  });

  describe("cover mosaic for a list or playlist", () => {
    const covers = ["/api/artwork/movie/m1/poster", "/api/artwork/album/a1/cover", "/api/artwork/show/s1/poster", "/api/artwork/audiobook/b1/cover"];
    const tile = () => document.querySelector<HTMLElement>("[data-mosaic]");

    it("draws up to four covers in one tile and never more", async () => {
      await renderWithProviders(<PosterCard title="Evening" image={covers[0]} aspect="square" mosaic={[...covers, "/api/artwork/movie/m9/poster"]} />);
      expect(tile()?.dataset.mosaic).toBe("4");
      expect(tile()?.querySelectorAll("img")).toHaveLength(4);
    });

    it("draws two and three covers with their own layout", async () => {
      const two = await renderWithProviders(<PosterCard title="Two" image={covers[0]} aspect="square" mosaic={covers.slice(0, 2)} />);
      expect(tile()?.dataset.mosaic).toBe("2");
      two.unmount();
      await renderWithProviders(<PosterCard title="Three" image={covers[0]} aspect="square" mosaic={covers.slice(0, 3)} />);
      expect(tile()?.dataset.mosaic).toBe("3");
    });

    it("falls back to the single picture with fewer than two covers", async () => {
      await renderWithProviders(<PosterCard title="One" image={covers[0]} aspect="square" mosaic={covers.slice(0, 1)} />);
      expect(tile()).toBeNull();
      expect(document.querySelectorAll("img")).toHaveLength(1);
    });

    it("falls back when covers fail to load, keeping the ones that do", async () => {
      await renderWithProviders(<PosterCard title="Broken" image={covers[0]} aspect="square" mosaic={covers.slice(0, 3)} />);
      const images = [...tile()!.querySelectorAll("img")];
      fireEvent.error(images[2]!);
      expect(tile()?.dataset.mosaic).toBe("2");
      fireEvent.error(tile()!.querySelectorAll("img")[0]!);
      expect(tile()).toBeNull();
      expect(document.querySelectorAll("img")).toHaveLength(1);
    });
  });
});
