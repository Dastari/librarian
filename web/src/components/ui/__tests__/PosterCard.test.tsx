import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { statusMeta } from "@/lib/status";
import { renderWithProviders } from "@/test";

import { PosterCard } from "../PosterCard";

describe("PosterCard", () => {
  it("shows the title, meta line and artwork", async () => {
    await renderWithProviders(<PosterCard title="Andor" meta="2022 · 24 episodes" image="/api/artwork/show/s1/poster" />);
    expect(screen.getByText("Andor")).toBeInTheDocument();
    expect(screen.getByText("2022 · 24 episodes")).toBeInTheDocument();
    const image = document.querySelector("img");
    expect(image).toHaveAttribute("src", "/api/artwork/show/s1/poster");
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
    const { container } = await renderWithProviders(<PosterCard title="Dune" image={null} badge="12/24" status={statusMeta("DOWNLOADING")} progress={0.42} />);
    expect(screen.getByText("12/24")).toBeInTheDocument();
    expect(container.querySelector('[title="Downloading"]')).toBeTruthy();
    expect(container.querySelector('[style*="width: 42%"]')).toBeTruthy();
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
    expect(screen.getByRole("button", { name: /Dune/ }).className).toContain("ring-brand");
  });
});
