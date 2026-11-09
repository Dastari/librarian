import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { MoveLibraryPanel } from "../MoveLibraryPanel";

const mocks = vi.hoisted(() => ({ query: vi.fn(), move: vi.fn(), scan: vi.fn(), success: vi.fn(), danger: vi.fn() }));
vi.mock("@apollo/client/react", () => ({
  useApolloClient: () => ({ query: mocks.query }),
  useMutation: (document: { definitions: Array<{ name?: { value: string } }> }) => {
    const name = document.definitions[0]?.name?.value;
    return [name === "MoveLibrary" ? mocks.move : mocks.scan];
  },
}));
vi.mock("@/features/files/FolderPickerDialog", () => ({ FolderPickerDialog: () => null }));
vi.mock("@/lib/graphql/live/hooks", () => ({ useLiveQuery: () => ({ data: undefined, loading: false, refetch: vi.fn() }) }));
vi.mock("glass-ux/toast", async (original) => ({ ...(await original<typeof import("glass-ux/toast")>()), toast: { success: mocks.success, danger: mocks.danger } }));

Element.prototype.getAnimations ??= () => [];
const wrapper = ({ children }: { children: ReactNode }) => <TooltipProvider><ActionProvider>{children}</ActionProvider></TooltipProvider>;
const library = { id: "lib-1", name: "Movies", path: "/data/media/Movies", scanning: false };
const plan = (over: Record<string, unknown> = {}) => ({
  oldPath: "/data/media/Movies", newPath: "/mnt/new/Movies", fileCount: 10, matched: 10, missing: 0, missingFiles: [], changed: 0, changedFiles: [],
  extra: 0, extraFiles: [], blocked: null, canApply: true, ...over,
});

async function checkFor(path: string) {
  await userEvent.type(screen.getByLabelText(/Folder/), path);
  await userEvent.click(screen.getByRole("button", { name: "Check the files" }));
}

describe("moving a library", () => {
  beforeEach(() => vi.clearAllMocks());

  it("says the files must be copied first, and where the library is now", () => {
    render(<MoveLibraryPanel library={library} />, { wrapper });
    expect(screen.getByText(/Librarian does not copy your files\./)).toBeInTheDocument();
    expect(screen.getByText("/data/media/Movies")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Check the files" })).toBeDisabled();
  });

  it("checks the new folder, then moves the library and starts a verifying scan", async () => {
    mocks.query.mockResolvedValue({ data: { libraryMovePlan: plan() } });
    mocks.move.mockResolvedValue({ data: { moveLibrary: { success: true, error: null, movedFiles: 10, movedFolders: 4, closedIssues: 0 } } });
    mocks.scan.mockResolvedValue({ data: { scanLibrary: { success: true } } });
    const onMoved = vi.fn();
    render(<MoveLibraryPanel library={library} onMoved={onMoved} />, { wrapper });
    await checkFor("/mnt/new/Movies");
    await waitFor(() => expect(mocks.query).toHaveBeenCalledWith(expect.objectContaining({ variables: { libraryId: "lib-1", newPath: "/mnt/new/Movies" } })));
    expect(await screen.findByText(/10 of 10 files found at/)).toBeInTheDocument();
    expect(screen.getByText("Every file is there. Nothing else is on the new location.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Move library" }));
    await waitFor(() => expect(mocks.move).toHaveBeenCalledWith({ variables: { input: { libraryId: "lib-1", newPath: "/mnt/new/Movies", allowMissing: false } } }));
    await waitFor(() => expect(mocks.scan).toHaveBeenCalledWith({ variables: { id: "lib-1" } }));
    expect(mocks.success).toHaveBeenCalledWith("A scan has started to verify the move.");
    expect(onMoved).toHaveBeenCalledOnce();
  });

  it("names what is missing and will not move until told to", async () => {
    mocks.query.mockResolvedValue({ data: { libraryMovePlan: plan({ matched: 8, missing: 2, missingFiles: ["A (2001)/A (2001).mkv", "B (2002)/B (2002).mkv"], canApply: false }) } });
    mocks.move.mockResolvedValue({ data: { moveLibrary: { success: true, error: null, movedFiles: 8, movedFolders: 0, closedIssues: 0 } } });
    mocks.scan.mockResolvedValue({ data: { scanLibrary: { success: true } } });
    render(<MoveLibraryPanel library={library} />, { wrapper });
    await checkFor("/mnt/new/Movies");
    expect(await screen.findByText("2 files missing")).toBeInTheDocument();
    expect(screen.getByText("A (2001)/A (2001).mkv")).toBeInTheDocument();
    const move = screen.getByRole("button", { name: "Move library" });
    expect(move).toBeDisabled();
    await userEvent.click(screen.getByRole("switch", { name: "Move even though files are missing" }));
    expect(move).toBeEnabled();
    await userEvent.click(move);
    await waitFor(() => expect(mocks.move).toHaveBeenCalledWith({ variables: { input: { libraryId: "lib-1", newPath: "/mnt/new/Movies", allowMissing: true } } }));
  });

  it("shows why a folder cannot be used and offers no move", async () => {
    mocks.query.mockResolvedValue({ data: { libraryMovePlan: plan({ blocked: "The new folder cannot be read: no such folder", matched: 0, canApply: false }) } });
    render(<MoveLibraryPanel library={library} />, { wrapper });
    await checkFor("/nowhere");
    expect(await screen.findByText("The new folder cannot be read: no such folder")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Move library" })).not.toBeInTheDocument();
  });

  it("says why a move failed and stays put", async () => {
    mocks.query.mockResolvedValue({ data: { libraryMovePlan: plan() } });
    mocks.move.mockResolvedValue({ data: { moveLibrary: { success: false, error: "A scan of this library is running", movedFiles: 0, movedFolders: 0, closedIssues: 0 } } });
    render(<MoveLibraryPanel library={library} />, { wrapper });
    await checkFor("/mnt/new/Movies");
    await userEvent.click(await screen.findByRole("button", { name: "Move library" }));
    expect(await screen.findByText("A scan of this library is running")).toBeInTheDocument();
    expect(mocks.scan).not.toHaveBeenCalled();
  });

  it("will not move while the library is scanning", async () => {
    mocks.query.mockResolvedValue({ data: { libraryMovePlan: plan() } });
    render(<MoveLibraryPanel library={{ ...library, scanning: true }} />, { wrapper });
    await checkFor("/mnt/new/Movies");
    expect(await screen.findByRole("button", { name: "Move library" })).toBeDisabled();
    expect(screen.getByText("A scan is running; wait for it to finish.")).toBeInTheDocument();
  });
});
