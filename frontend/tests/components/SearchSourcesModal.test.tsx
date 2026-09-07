// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
const state = vi.hoisted(() => ({ query: vi.fn(), addTorrent: vi.fn(), toast: vi.fn() }));
vi.mock("../../src/lib/graphql/client", () => ({ apolloClient: { query: state.query }, useMutation: () => [state.addTorrent] }));
vi.mock("@heroui/toast", () => ({ addToast: state.toast }));
import { SearchSourcesModal } from "../../src/components/SearchSourcesModal";
const release = { title: "Fallout.S01E08.1080p", guid: "release", sourceId: "indexer", sourceName: "Indexer", magnetUri: "magnet:?xt=urn:btih:test-fixture", size: 100, sizeFormatted: "100 B", seeders: 10 };
const result = { data: { searchSources: { sourcesSearched: 1, sources: [{ sourceName: "Indexer", releases: [release], error: null }] } } };
const initialSearch = { query: "Fallout", season: 1, episode: "8", categories: [5000] };
beforeEach(() => { vi.clearAllMocks(); state.query.mockResolvedValue(result); });
afterEach(cleanup);
it("searches for the selected episode and retains its library/show when downloading a chosen release", async () => {
  state.addTorrent.mockResolvedValue({ data: { addTorrent: { success: true, torrent: { name: release.title } } } });
  render(<SearchSourcesModal isOpen onClose={() => {}} initialSearch={initialSearch} title="Search for Fallout S01E08" libraryId="tv-library" showId="fallout" />);
  await screen.findByText(release.title);
  expect(screen.getByRole("textbox", { name: "Search sources" })).toHaveProperty("value", "Fallout S01E08");
  expect(state.query).toHaveBeenCalledWith(expect.objectContaining({ variables: { input: initialSearch } }));
  expect(state.addTorrent).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: `Download ${release.title}` }));
  await waitFor(() => expect(state.addTorrent).toHaveBeenCalledWith(expect.objectContaining({ variables: { input: expect.objectContaining({ libraryId: "tv-library", showId: "fallout", magnet: release.magnetUri, sourceIndexerId: "indexer" }) } })));
});
it("searches the edited episode and removes the episode restriction when the token is deleted", async () => {
  render(<SearchSourcesModal isOpen onClose={() => {}} initialSearch={initialSearch} />);
  await screen.findByText(release.title);
  const input = screen.getByRole("textbox", { name: "Search sources" });
  fireEvent.change(input, { target: { value: "Fallout S02E01" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  await waitFor(() => expect(state.query).toHaveBeenLastCalledWith(expect.objectContaining({ variables: { input: { query: "Fallout", season: 2, episode: "1", categories: [5000] } } })));
  await screen.findByText(release.title);
  fireEvent.change(input, { target: { value: "Fallout" } });
  fireEvent.keyDown(input, { key: "Enter" });
  await waitFor(() => expect(state.query).toHaveBeenLastCalledWith(expect.objectContaining({ variables: { input: { query: "Fallout", season: undefined, episode: undefined, categories: [5000] } } })));
});
it("shows a search failure and permits retry instead of leaving the spinner running", async () => {
  state.query.mockRejectedValueOnce(new Error("Source unavailable"));
  render(<SearchSourcesModal isOpen onClose={() => {}} initialSearch={initialSearch} />);
  expect((await screen.findByRole("alert")).textContent).toContain("Source unavailable");
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  await screen.findByText(release.title);
  expect(state.query).toHaveBeenCalledTimes(2);
});
it("explains that sources are missing instead of redirecting out of the episode page", async () => {
  state.query.mockResolvedValue({ data: { searchSources: { sourcesSearched: 0, sources: [] } } });
  render(<SearchSourcesModal isOpen onClose={() => {}} initialSearch={initialSearch} />);
  expect((await screen.findByRole("alert")).textContent).toContain("No enabled sources");
});
