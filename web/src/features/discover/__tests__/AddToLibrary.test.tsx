import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AddToLibrary, type AddTarget } from "../AddToLibrary";

const mocks = vi.hoisted(() => ({
  admin: true,
  libraries: [] as Array<{ id: string; name: string; libraryType: string }>,
  navigate: vi.fn(),
  addMovie: vi.fn(),
  updateMovie: vi.fn(),
  searchMissing: vi.fn(),
}));

vi.mock("@tanstack/react-router", async (original) => ({ ...(await original<typeof import("@tanstack/react-router")>()), useNavigate: () => mocks.navigate }));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => mocks.admin }));
vi.mock("@/lib/graphql/live/hooks", () => ({
  useLiveQuery: () => ({ loading: false, data: { libraries: { edges: mocks.libraries.map((node) => ({ node })) } } }),
}));
vi.mock("@/features/downloads/ReleaseSearchDialog", () => ({
  ReleaseSearchDialog: ({ isOpen, target }: { isOpen: boolean; target?: { movieId?: string } }) => (isOpen ? <div>Releases for {target?.movieId}</div> : null),
}));
vi.mock("@apollo/client/react", () => ({
  useMutation: (document: { definitions: Array<{ name?: { value: string } }> }) => {
    const name = document.definitions[0]?.name?.value;
    return [name === "AddMovie" ? mocks.addMovie : name === "EntityMovieUpdate" ? mocks.updateMovie : name === "SearchMissing" ? mocks.searchMissing : vi.fn()];
  },
}));

const movie: AddTarget = { kind: "movie", tmdbId: 27205, title: "Inception", year: 2010, imdbId: "tt1375666" };
const wrapper = ({ children }: { children: ReactNode }) => <TooltipProvider><ActionProvider navigate={vi.fn()}>{children}</ActionProvider></TooltipProvider>;
const films = { id: "l1", name: "Films", libraryType: "MOVIES" };

// jsdom has no Web Animations; Base UI waits on them when a dialog closes.
Element.prototype.getAnimations ??= () => [];

describe("AddToLibrary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.admin = true;
    mocks.libraries = [films];
    mocks.addMovie.mockResolvedValue({ data: { addMovie: { success: true, error: null, movie: { id: "m1" } } } });
    mocks.updateMovie.mockResolvedValue({ data: { updateMovie: { success: true, error: null } } });
    mocks.searchMissing.mockResolvedValue({ data: { searchMissing: { success: true, searched: 1, queued: 1, error: null } } });
  });

  const add = async () => {
    render(<AddToLibrary target={movie} />, { wrapper });
    await userEvent.click(screen.getByRole("button", { name: "Add to library" }));
  };

  it("adds straight away, without downloading, then asks what to do next", async () => {
    await add();
    expect(mocks.addMovie).toHaveBeenCalledWith({ variables: { libraryId: "l1", input: { tmdbId: 27205, monitored: false } } });
    expect(await screen.findByText("Inception is in your library")).toBeInTheDocument();
    expect(screen.getByText("Added to Films. Nothing has downloaded yet.")).toBeInTheDocument();
    expect(mocks.searchMissing).not.toHaveBeenCalled();
  });

  it("goes to the real page on Not now", async () => {
    await add();
    await userEvent.click(await screen.findByRole("button", { name: "Not now" }));
    expect(mocks.navigate).toHaveBeenCalledWith({ to: "/movies/$movieId", params: { movieId: "m1" }, replace: true });
  });

  it("turns on downloads and searches when choosing automatic", async () => {
    await add();
    await userEvent.click(await screen.findByText("Download automatically"));
    await waitFor(() => expect(mocks.searchMissing).toHaveBeenCalledWith({ variables: { input: { movieId: "m1" } } }));
    expect(mocks.updateMovie).toHaveBeenCalledWith({ variables: { id: "m1", input: { monitored: true, wanted: true } } });
    await waitFor(() => expect(mocks.navigate).toHaveBeenCalled());
  });

  it("opens release search against the new title", async () => {
    await add();
    await userEvent.click(await screen.findByText("Choose a release"));
    expect(await screen.findByText("Releases for m1")).toBeInTheDocument();
  });

  it("asks which library first when there is more than one", async () => {
    mocks.libraries = [films, { id: "l2", name: "Kids", libraryType: "MOVIES" }];
    await add();
    expect(await screen.findByText("Which library should it go in?")).toBeInTheDocument();
    expect(mocks.addMovie).not.toHaveBeenCalled();
  });

  it("explains itself when no library of the kind exists", () => {
    mocks.libraries = [{ id: "l3", name: "Series", libraryType: "TV" }];
    render(<AddToLibrary target={movie} />, { wrapper });
    expect(screen.getByRole("button", { name: "Add to library" })).toHaveAttribute("aria-disabled", "true");
  });

  it("is not offered to someone who cannot add", () => {
    mocks.admin = false;
    render(<AddToLibrary target={movie} />, { wrapper });
    expect(screen.queryByRole("button", { name: "Add to library" })).toBeNull();
  });
});
