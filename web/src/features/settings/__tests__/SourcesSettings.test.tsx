import { screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { EntityRssFeedListDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { SourcesSettings, sourceRows } from "../SourcesSettings";

vi.mock("@/lib/auth/useSession", () => ({
  useIsAdmin: () => true,
  useSession: () => ({ user: { id: "me", role: "admin" } }),
}));

const source = (extra: Record<string, unknown>) => ({
  id: "src-a", name: "IPTorrents", sourceType: "torrent_indexer", definitionId: "iptorrents", enabled: true, priority: 1, mediaTypes: "movies,tv", siteUrl: "https://iptorrents.com/",
  supportsSearch: true, supportsTvSearch: true, supportsMovieSearch: true, supportsMusicSearch: false, supportsBookSearch: false, settings: null,
  lastError: null, errorCount: 0, lastSuccessAt: "2026-11-08T10:00:00Z", lastErrorAt: null, healthStatus: "ok", healthKind: null, lastCheckedAt: "2026-11-08T11:30:00Z", nextCheckAt: null, loginAttemptedAt: null,
  createdAt: "2026-11-01T00:00:00Z", updatedAt: "2026-11-08T11:30:00Z", ...extra,
});
const feed = (extra: Record<string, unknown>) => ({
  id: "feed-a", userId: "me", libraryId: null, name: "TV feed", urlDisplay: "https://iptorrents.com/t.rss?u=•••;tp=•••;download", enabled: true, pollIntervalMinutes: 30, postDownloadAction: null,
  lastPolledAt: "2026-11-08T11:45:00Z", lastSuccessfulAt: "2026-11-08T11:45:00Z", lastError: null, consecutiveFailures: 0, healthStatus: "ok", healthKind: null,
  createdAt: "2026-11-01T00:00:00Z", updatedAt: "2026-11-08T11:45:00Z", ...extra,
});

const state = vi.hoisted(() => ({ sources: [] as unknown[], feeds: [] as unknown[] }));
vi.mock("../useSettingsConnection", () => ({
  useSettingsConnection: (document: unknown) => ({
    rows: document === (globalThis as { __feedDoc?: unknown }).__feedDoc ? state.feeds : state.sources,
    loading: false, error: undefined, refetch: vi.fn(),
  }),
}));

beforeEach(() => {
  (globalThis as { __feedDoc?: unknown }).__feedDoc = EntityRssFeedListDocument;
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ x: 0, y: 0, top: 0, left: 0, bottom: 800, right: 1440, width: 1440, height: 800, toJSON: () => ({}) });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(1440);
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1440);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(800);
});
afterEach(() => vi.restoreAllMocks());

describe("sourceRows", () => {
  it("lists indexers first, then feeds, each with the state its health fields give", () => {
    const now = Date.parse("2026-11-08T12:00:00Z");
    const rows = sourceRows(
      [source({ id: "a" }) as never, source({ id: "b", name: "Torznab", healthStatus: "auth_expired", healthKind: "auth", lastError: "HTTP 401", errorCount: 2 }) as never],
      [feed({}) as never, feed({ id: "feed-b", name: "Movie feed", healthStatus: "failing", healthKind: "unreachable", lastError: "the feed answered HTTP 503", consecutiveFailures: 3 }) as never],
      now,
    );
    expect(rows.map((row) => `${row.kind}:${row.name}`)).toEqual(["source:IPTorrents", "source:Torznab", "feed:TV feed", "feed:Movie feed"]);
    expect(rows.map((row) => row.health.label)).toEqual(["Working", "Sign-in expired ×2", "Working", "Unreachable ×3"]);
    expect(rows[3]!.health.detail).toContain("the feed answered HTTP 503");
  });
});

describe("SourcesSettings", () => {
  it("shows a failing indexer in red with why, and a feed by its redacted address", async () => {
    state.sources = [source({ healthStatus: "auth_expired", healthKind: "auth", lastError: "IPTorrents can't sign in: its login has expired", errorCount: 1 })];
    state.feeds = [feed({})];
    await renderWithProviders(<SourcesSettings />, { router: false });
    expect(await screen.findByText("IPTorrents")).toBeInTheDocument();
    expect(screen.getByText(/IPTorrents can't sign in: its login has expired · last worked/)).toBeInTheDocument();
    expect(screen.getByText("TV feed")).toBeInTheDocument();
    // The feed shows the address with its key hidden, never a key.
    expect(screen.getByText(/t\.rss\?u=•••;tp=•••;download/)).toBeInTheDocument();
    expect(screen.queryByText(/0123456789abcdef/)).not.toBeInTheDocument();
  });
});
