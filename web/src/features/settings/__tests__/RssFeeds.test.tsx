import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { RssFeeds, feedState } from "../RssFeeds";

const state = vi.hoisted(() => ({ admin: true, create: vi.fn(), update: vi.fn(), refresh: vi.fn() }));
vi.mock("@/lib/auth/useSession", () => ({
  useIsAdmin: () => state.admin,
  useSession: () => ({ user: { id: "me", role: state.admin ? "admin" : "member" } }),
}));
vi.mock("../useSettingsConnection", () => ({
  useSettingsConnection: () => ({
    rows: [
      { id: "feed-a", userId: "me", libraryId: null, name: "TV releases", url: "https://feeds.example.com/tv?key=secret", enabled: true, pollIntervalMinutes: 30, postDownloadAction: null, lastPolledAt: "2026-11-08T12:00:00Z", lastSuccessfulAt: "2026-11-08T12:00:00Z", lastError: null, consecutiveFailures: 0, createdAt: "2026-11-01T00:00:00Z", updatedAt: "2026-11-08T12:00:00Z" },
      { id: "feed-b", userId: "me", libraryId: null, name: "Movie releases", url: "https://other.example.com/movies", enabled: true, pollIntervalMinutes: 60, postDownloadAction: null, lastPolledAt: "2026-11-08T12:00:00Z", lastSuccessfulAt: null, lastError: "the feed answered HTTP 503", consecutiveFailures: 3, createdAt: "2026-11-01T00:00:00Z", updatedAt: "2026-11-08T12:00:00Z" },
    ],
    loading: false, error: undefined, refetch: state.refresh,
  }),
}));
vi.mock("@apollo/client/react", async (original) => {
  const docs = await import("@/graphql/generated/graphql");
  return { ...await original<typeof import("@apollo/client/react")>(), useMutation: (document: unknown) => [
    document === docs.EntityRssFeedCreateDocument ? state.create : document === docs.EntityRssFeedUpdateDocument ? state.update : vi.fn(), { loading: false },
  ] };
});

beforeEach(() => {
  vi.clearAllMocks();
  state.admin = true;
  state.create.mockResolvedValue({ data: { createRssFeed: { success: true } } });
  state.update.mockResolvedValue({ data: { updateRssFeed: { success: true } } });
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ x: 0, y: 0, top: 0, left: 0, bottom: 800, right: 1440, width: 1440, height: 800, toJSON: () => ({}) });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(1440);
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1440);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(800);
});
afterEach(() => vi.restoreAllMocks());

describe("feedState", () => {
  it("tells a healthy feed from a failing one and a disabled one", () => {
    expect(feedState({ enabled: true, lastError: null, consecutiveFailures: 0 }).label).toBe("Enabled");
    expect(feedState({ enabled: true, lastError: "boom", consecutiveFailures: 3 }).label).toBe("Failing ×3");
    expect(feedState({ enabled: false, lastError: "boom", consecutiveFailures: 3 }).label).toBe("Disabled");
  });
});

describe("RssFeeds", () => {
  it("lists the feeds by host, so an address carrying a key is not on show", async () => {
    await renderWithProviders(<RssFeeds />, { router: false });
    expect(await screen.findByText("TV releases")).toBeInTheDocument();
    expect(screen.getByText("feeds.example.com")).toBeInTheDocument();
    expect(screen.queryByText(/secret/)).not.toBeInTheDocument();
    // A failing feed says why under its name, in place of its host.
    expect(screen.getByText("the feed answered HTTP 503")).toBeInTheDocument();
    expect(screen.queryByText("other.example.com")).not.toBeInTheDocument();
  });

  it("refuses an address that is not http or https, and a name-less feed", async () => {
    await renderWithProviders(<RssFeeds />, { router: false });
    await userEvent.click(await screen.findByRole("button", { name: /Add feed/ }));
    await userEvent.type(await screen.findByLabelText(/Feed address/), "ftp://example.com/feed");
    await userEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(await screen.findByText("Give the feed a name")).toBeInTheDocument();
    expect(screen.getByText("Enter an http or https address")).toBeInTheDocument();
    expect(state.create).not.toHaveBeenCalled();
  });

  it("refuses an interval below the floor the server enforces", async () => {
    await renderWithProviders(<RssFeeds />, { router: false });
    await userEvent.click(await screen.findByRole("button", { name: /Add feed/ }));
    await userEvent.type(await screen.findByLabelText(/^Name/), "New feed");
    await userEvent.type(screen.getByLabelText(/Feed address/), "https://example.com/feed");
    const minutes = screen.getByLabelText(/Read every/);
    await userEvent.clear(minutes);
    await userEvent.type(minutes, "5");
    await userEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(await screen.findByText("At least 15 minutes")).toBeInTheDocument();
    expect(state.create).not.toHaveBeenCalled();
  });

  it("adds a feed owned by the signed-in administrator", async () => {
    await renderWithProviders(<RssFeeds />, { router: false });
    await userEvent.click(await screen.findByRole("button", { name: /Add feed/ }));
    await userEvent.type(await screen.findByLabelText(/^Name/), "  New feed ");
    await userEvent.type(screen.getByLabelText(/Feed address/), "https://example.com/feed.xml");
    await userEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() => expect(state.create).toHaveBeenCalledTimes(1));
    expect(state.create).toHaveBeenCalledWith({
      variables: { input: { name: "New feed", url: "https://example.com/feed.xml", pollIntervalMinutes: 30, enabled: true, userId: "me" } },
    });
    await waitFor(() => expect(state.refresh).toHaveBeenCalled());
  });

  it("offers a member no way to add or change a feed", async () => {
    state.admin = false;
    await renderWithProviders(<RssFeeds />, { router: false });
    expect(await screen.findByText("TV releases")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Add feed/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("checkbox", { name: "Select TV releases" })).not.toBeInTheDocument();
  });
});
