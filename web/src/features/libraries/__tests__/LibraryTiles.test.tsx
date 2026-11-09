import { act, fireEvent, render, screen } from "@testing-library/react";
import { defineActions, resolveActions } from "glass-ux/actions";
import { describe, expect, it, vi } from "vitest";

import { useSpatialNavigation } from "@/lib/input-mode";

import { LibraryTile } from "../LibrariesPage";
import { useLibraryActions } from "../useLibraryActions";
import type { LibraryOverview } from "../LibraryCard";
import { renderHook } from "@testing-library/react";

vi.mock("@tanstack/react-router", () => ({
  Link: ({ to, params: _params, ...props }: { to: string; params?: unknown }) => <a href={to} {...props} />,
}));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => true, useSession: () => ({ user: { id: "u1" } }) }));
vi.mock("@apollo/client/react", () => ({ useMutation: () => [vi.fn()] }));
vi.mock("@/lib/graphql/live/hooks", () => ({ useLiveQuery: () => ({ data: undefined }) }));
vi.mock("@/features/search/addScopes", () => ({ addHref: () => null, addScopeFor: () => null }));

const library = (id: string, name: string): LibraryOverview => ({
  id, name, userId: "u1", libraryType: "MOVIES", scanning: false, lastScannedAt: null,
  movies: { pageInfo: { totalCount: 3 }, edges: [] }, shows: { pageInfo: { totalCount: 0 }, edges: [] }, albums: { pageInfo: { totalCount: 0 }, edges: [] }, audiobooks: { pageInfo: { totalCount: 0 }, edges: [] },
}) as unknown as LibraryOverview;

const none = defineActions<LibraryOverview>("library", []);

function Grid({ favourites }: { favourites: string[] }) {
  useSpatialNavigation(true, true);
  return (
    <main>
      {["a", "b"].map((id) => <LibraryTile key={id} library={library(id, `Library ${id}`)} actions={none} favourite={favourites.includes(id)} canFavourite pending={false} disabled={false} onToggle={() => undefined} />)}
    </main>
  );
}

function lay(element: Element, left: number) {
  (element as HTMLElement).getBoundingClientRect = () => ({ x: left, y: 0, left, top: 0, right: left + 100, bottom: 100, width: 100, height: 100, toJSON: () => ({}) }) as DOMRect;
}

describe("library cards on the Libraries page", () => {
  it("is one stop per card: arrow keys go from a library to the next and never land on a star", () => {
    render(<Grid favourites={["a", "b"]} />);
    const links = screen.getAllByRole("link");
    links.forEach((link, index) => lay(link, index * 200));
    document.querySelectorAll("button").forEach((button, index) => lay(button, index * 200 + 130));
    links[0]!.focus();
    fireEvent.keyDown(links[0]!, { key: "ArrowRight" });
    expect(document.activeElement).toBe(links[1]);
    fireEvent.keyDown(links[1]!, { key: "ArrowLeft" });
    expect(document.activeElement).toBe(links[0]);
  });

  it("keeps the star out of the tab order and away from assistive tech, while a click still toggles", () => {
    const onToggle = vi.fn();
    render(<LibraryTile library={library("a", "Library a")} actions={none} favourite canFavourite pending={false} disabled={false} onToggle={onToggle} />);
    const star = document.querySelector<HTMLButtonElement>("button")!;
    expect(star).toHaveAttribute("tabindex", "-1");
    expect(star).toHaveAttribute("data-nav-skip");
    expect(star).toHaveAttribute("aria-hidden", "true");
    fireEvent.click(star);
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it("shows the mark filled for a favourite, and only on hover for the rest", () => {
    const { rerender } = render(<LibraryTile library={library("a", "Library a")} actions={none} favourite canFavourite pending={false} disabled={false} onToggle={() => undefined} />);
    expect(document.querySelector("[data-favourite]")).toHaveAttribute("data-favourite", "true");
    expect(document.querySelector("[data-favourite]")!.className).not.toContain("opacity-0");
    rerender(<LibraryTile library={library("a", "Library a")} actions={none} favourite={false} canFavourite pending={false} disabled={false} onToggle={() => undefined} />);
    expect(document.querySelector("[data-favourite]")!.className).toContain("opacity-0");
  });

  it("moves toggling into the card's menu", async () => {
    const toggle = vi.fn(async () => undefined);
    const state = { favourite: false };
    const { result } = renderHook(() => useLibraryActions<LibraryOverview>({ settings: vi.fn(), favourites: { isFavourite: () => state.favourite, canChange: (l) => l.userId === "u1", toggle } }));
    const row = library("a", "Library a");
    const names = () => resolveActions(result.current, [row], "menu").flatMap((group) => group.actions.map((action) => action.label));
    expect(names()).toContain("Add to favourites");
    state.favourite = true;
    expect(names()).toContain("Remove from favourites");
    const group = resolveActions(result.current, [row], "menu").flatMap((g) => g.actions).find((action) => action.id === "favourite")!;
    await act(async () => { await group.definition.run?.([row], {} as never); });
    expect(toggle).toHaveBeenCalledWith(row);
    const other = { ...row, userId: "someone" } as LibraryOverview;
    expect(resolveActions(result.current, [other], "menu").flatMap((g) => g.actions).some((action) => action.id === "favourite")).toBe(false);
  });
});
