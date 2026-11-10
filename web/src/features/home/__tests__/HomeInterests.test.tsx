import { screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { GenreInterestShelvesDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { HomePage, interestTitles } from "../HomePage";

const shelves = vi.hoisted(() => ({ value: [] as unknown[] }));
vi.mock("@/lib/graphql/live/hooks", () => ({
  useLiveQuery: (document: unknown) => (document === GenreInterestShelvesDocument
    ? { data: { genreInterestShelves: shelves.value }, loading: false }
    : { data: undefined, loading: false }),
}));
vi.mock("@/features/libraries/browser/useCompleteConnection", () => ({
  useCompleteConnection: () => ({ rows: [{ id: "library" }], loading: false }),
}));
vi.mock("@/features/personal/usePersonalLibrary", () => ({
  useHomeShelves: () => ({ shelves: [{ key: "interests", visible: true }, { key: "libraries", visible: true }] }),
  useLibraryPreferences: () => ({ rows: [] }),
}));
vi.mock("../useContinueWatching", () => ({ useContinueWatching: () => ({ items: [], loading: false }) }));

const movie = (id: string, title: string, createdAt: string) => ({ __typename: "Movie", id, title, year: 2020, runtime: 120, mediaFileId: null, createdAt } as never);
const show = (id: string, name: string, createdAt: string) => ({ __typename: "Show", id, name, year: 2019, network: "HBO", createdAt } as never);

describe("genre interests on Home", () => {
  it("adds a row per genre you like, movies and shows together, newest first", async () => {
    shelves.value = [
      { genre: "Sci-Fi", total: 3, movies: [movie("m1", "Arrival", "1700000000"), movie("m2", "Dune", "1800000000")], shows: [show("s1", "Severance", "1750000000")] },
      { genre: "Drama", total: 1, movies: [movie("m3", "Heat", "1600000000")], shows: [] },
    ];
    await renderWithProviders(<HomePage />);
    const scifi = screen.getByRole("region", { name: "Because you like Sci-Fi" });
    // In document order: newest first, movies and shows mixed.
    const titles = within(scifi).getAllByText(/^(Arrival|Dune|Severance)$/).map((node) => node.textContent);
    expect([...new Set(titles)]).toEqual(["Dune", "Severance", "Arrival"]);
    expect(screen.getByRole("region", { name: "Because you like Drama" })).toBeInTheDocument();
  });

  it("shows no genre rows without interests", async () => {
    shelves.value = [];
    await renderWithProviders(<HomePage />);
    expect(screen.queryByRole("region", { name: /Because you like/ })).not.toBeInTheDocument();
  });

  it("merges a shelf's movies and shows by when they were added", () => {
    const shelf = { genre: "Sci-Fi", total: 2, movies: [movie("m1", "Old", "2026-01-01T00:00:00Z")], shows: [show("s1", "New", "2026-02-01T00:00:00Z")] } as never;
    expect(interestTitles(shelf).map((title) => title.kind)).toEqual(["show", "movie"]);
  });
});
