import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AnalyzeMediaFileDocument, MatchMediaFileDocument, OrganizeMediaFileDocument, UnmatchMediaFileDocument, LibraryTracksDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";
import { FilesBrowser } from "../FilesBrowser";
import { TracksBrowser } from "../TracksBrowser";

const fixture = vi.hoisted(() => ({ admin: true, query: "", missing: false, analyze: vi.fn(), match: vi.fn(), organize: vi.fn(), unmatch: vi.fn(), refresh: vi.fn(), play: vi.fn(), enqueue: vi.fn() }));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => fixture.admin, useSession: () => ({ user: { id: "owner" } }) }));
vi.mock("@/lib/graphql/live/hooks", () => ({ useLiveQuery: () => ({ data: { library: { userId: "owner" } } }) }));
vi.mock("@/hooks/useContentStatuses", () => ({ useContentStatuses: () => new Map() }));
vi.mock("@/features/player/usePlayer", () => ({ usePlayer: () => ({ playAudio: fixture.play, enqueue: fixture.enqueue }) }));
vi.mock("@/features/libraries/ManualMatchDialog", () => ({ ManualMatchDialog: () => null }));
vi.mock("../useBrowserFilters", () => ({ useBrowserFilters: () => ({ query: fixture.query, debouncedQuery: fixture.query, availability: "all", setQuery: vi.fn(), setAvailability: vi.fn() }), titleFilter: () => fixture.query ? { contains: fixture.query } : undefined }));
vi.mock("@/hooks/useServerTable", () => ({ useServerTable: () => ({ sorting: [], orderBy: [], setSorting: vi.fn() }) }));
vi.mock("../useInfiniteConnection", () => ({ useInfiniteConnection: (doc: unknown) => ({
  rows: doc === LibraryTracksDocument ? [
    { id: "track-b", title: "Track B", albumId: "album", album: { id: "album", name: "Album" }, mediaFileId: "file-b", artistName: "Artist", trackNumber: 2, durationSecs: 100 },
    { id: "track-a", title: "Track A", albumId: "album", album: { id: "album", name: "Album" }, mediaFileId: fixture.missing ? null : "file-a", artistName: "Artist", trackNumber: 1, durationSecs: 100 },
  ] : [
    { id: "file-a", path: "/library/a.mkv", relativePath: "a.mkv", size: 1024, addedAt: "2026-10-31", movieId: null },
    { id: "file-b", path: "/library/b.mkv", relativePath: "b.mkv", size: 2048, addedAt: "2026-10-31", movieId: null },
  ], fetching: false, loading: false, totalCount: 2, hasMore: false, loadMore: vi.fn(), refetch: fixture.refresh, queryKey: fixture.query, loadRevision: 0,
}) }));
vi.mock("@apollo/client/react", async (original) => ({ ...await original<typeof import("@apollo/client/react")>(), useMutation: (doc: unknown) => [doc === AnalyzeMediaFileDocument ? fixture.analyze : doc === MatchMediaFileDocument ? fixture.match : doc === OrganizeMediaFileDocument ? fixture.organize : doc === UnmatchMediaFileDocument ? fixture.unmatch : vi.fn()] }));

beforeEach(() => {
  vi.clearAllMocks(); fixture.admin = true; fixture.query = ""; fixture.missing = false;
  fixture.analyze.mockResolvedValue({ data: { analyzeMediaFile: { success: true } } });
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ x: 0, y: 0, top: 0, left: 0, bottom: 800, right: 1440, width: 1440, height: 800, toJSON: () => ({}) });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(1440);
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1440);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(800);
});
afterEach(() => vi.restoreAllMocks());

async function selectAll(noun: string) {
  await userEvent.click(screen.getByRole("checkbox", { name: "Select all loaded rows" }));
  await userEvent.click(screen.getByRole("button", { name: `Actions for 2 selected ${noun}` }));
}
describe("library table selection actions", () => {
  it("plays exactly the selected tracks in visible order", async () => {
    await renderWithProviders(<TracksBrowser libraryId="library" />, { router: false });
    await selectAll("tracks");
    await userEvent.click(await screen.findByRole("menuitem", { name: "Play 2 selected tracks" }));
    await waitFor(() => expect(fixture.play).toHaveBeenCalledOnce());
    expect(fixture.play.mock.calls[0]![0].map((item: { mediaFileId: string }) => item.mediaFileId)).toEqual(["file-b", "file-a"]);
  });
  it("explains unplayable mixed selections rather than silently dropping a track", async () => {
    fixture.missing = true;
    await renderWithProviders(<TracksBrowser libraryId="library" />, { router: false });
    await selectAll("tracks");
    expect(await screen.findByRole("menuitem", { name: "Add to queue" })).toHaveAttribute("aria-disabled", "true");
    expect(fixture.enqueue).not.toHaveBeenCalled();
  });
  it("clears file selection when the server filter changes", async () => {
    const view = await renderWithProviders(<FilesBrowser libraryId="library" />, { router: false });
    await userEvent.click(screen.getByRole("checkbox", { name: "Select a.mkv" }));
    expect(screen.getByRole("checkbox", { name: "Select a.mkv" })).toBeChecked();
    fixture.query = "new query";
    view.rerender(<FilesBrowser libraryId="library" />);
    expect(screen.getByRole("checkbox", { name: "Select a.mkv" })).not.toBeChecked();
  });
  it("continues selected file analysis after a refused result and refreshes once", async () => {
    fixture.analyze.mockResolvedValueOnce({ data: { analyzeMediaFile: { success: false, message: "Scanner busy" } } });
    await renderWithProviders(<FilesBrowser libraryId="library" />, { router: false });
    await selectAll("files");
    await userEvent.click(await screen.findByRole("menuitem", { name: "Re-analyze 2 files" }));
    await waitFor(() => expect(fixture.analyze).toHaveBeenCalledTimes(2));
    expect(fixture.analyze).toHaveBeenCalledWith({ variables: { mediaFileId: "file-a", path: "/library/a.mkv" } });
    expect(fixture.analyze).toHaveBeenCalledWith({ variables: { mediaFileId: "file-b", path: "/library/b.mkv" } });
    expect(fixture.refresh).toHaveBeenCalledOnce();
  });
  it("retains useful read-only selection without exposing member management commands", async () => {
    fixture.admin = false;
    await renderWithProviders(<FilesBrowser libraryId="library" />, { router: false });
    await selectAll("files");
    expect(await screen.findByRole("menuitem", { name: "Copy file paths" })).toBeInTheDocument();
    expect(screen.queryByRole("menuitem", { name: /analyze|Organize|Unlink/ })).toBeNull();
  });
});
