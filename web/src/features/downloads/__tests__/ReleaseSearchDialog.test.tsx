import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AddTorrentDocument, GrabRecommendationDocument, GrabRecommendationPlanDocument, HasEnabledSourcesDocument, SearchSourcesDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";
import { layoutColumn, RemoteNavigation, remoteStopElements, walkRemote } from "@/test/remote";

import { ReleaseSearchDialog } from "../ReleaseSearchDialog";

// What the server recommends for the target; nothing unless a test says otherwise.
const recommendedState = vi.hoisted(() => ({ value: null as unknown }));
vi.mock("@/features/recommendations/useRecommended", () => ({
  useRecommended: () => recommendedState.value ?? { plan: null, releases: [], forTarget: 0, searching: false, none: null, rows: [], loaded: true, refetch: () => {} },
}));

// The public Glass table uses measured viewport geometry to render its rows.
beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ width: 1200, height: 700, top: 0, left: 0, bottom: 700, right: 1200, x: 0, y: 0, toJSON: () => ({}) });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 32 : 1200; });
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 48 : 700; });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1200);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(700);
});
afterEach(() => {
  vi.restoreAllMocks();
  recommendedState.value = null;
});

type Parsed = Partial<{
  resolution: string | null;
  codec: string | null;
  hdrType: string | null;
  sourceType: string | null;
  audio: string | null;
  releaseGroup: string | null;
  languages: string[];
  isSeasonPack: boolean;
  isProper: boolean;
  isRepack: boolean;
  season: number | null;
  episodes: number[];
  year: number | null;
}>;

const parsed = (overrides: Parsed = {}) => ({
  __typename: "ParsedRelease" as const,
  resolution: "1080p",
  codec: "h264",
  hdrType: null,
  sourceType: "web",
  audio: null,
  releaseGroup: null,
  languages: [] as string[],
  isSeasonPack: false,
  isProper: false,
  isRepack: false,
  season: null,
  episodes: [] as number[],
  year: null,
  ...overrides,
});

const release = (guid: string, overrides: Partial<Record<string, unknown>> = {}) => ({
  __typename: "SourceRelease" as const,
  title: `Release ${guid}`,
  format: null,
  formatReason: null,
  packHint: null,
  relevance: null,
  otherBookNumber: null,
  guid,
  stableId: `${String(overrides.sourceId ?? "src-1")}:${guid}`,
  link: `https://tracker/${guid}.torrent`,
  magnetUri: `magnet:?xt=urn:btih:${guid}`,
  infoHash: guid,
  details: `https://tracker/details/${guid}`,
  publishDate: "2026-09-01T00:00:00Z",
  categories: ["TV"],
  size: 2_000_000_000,
  sizeFormatted: "1.9 GB",
  seeders: 10,
  leechers: 2,
  peers: 12,
  grabs: 100,
  isFreeleech: false,
  imdbId: null,
  poster: null,
  description: null,
  sourceId: "src-1",
  sourceName: "Tracker",
  profileMatch: "optimal",
  rejectReasons: [] as string[],
  fromFeed: false,
  feedItemId: null as string | null,
  ...overrides,
  parsed: parsed((overrides.parsed as Parsed) ?? {}),
});

const searchInput = (overrides: Partial<Record<string, unknown>> = {}) => ({
  query: "Andor",
  imdbId: null,
  season: 2,
  episode: "5",
  year: null,
  artist: null,
  album: null,
  author: null,
  showId: "show-1",
  movieId: null,
  albumId: null,
  audiobookId: null,
  autoWiden: false,
  limit: null,
  ...overrides,
});

const searchMock = (releases: unknown[], sourceOverrides: Partial<Record<string, unknown>> = {}) => ({
  request: { query: SearchSourcesDocument, variables: { input: searchInput() } },
  result: {
    data: {
      searchSources: {
        __typename: "SearchSourcesResult",
        totalReleases: releases.length,
        totalElapsedMs: 120,
        sourcesSearched: 1,
        searchedTerm: "Andor",
        widened: false,
        sources: [
          {
            __typename: "SourceSearchResult",
            sourceId: "src-1",
            sourceName: "Tracker",
            elapsedMs: 120,
            fromCache: false,
            fromFeed: false,
            error: null,
            pageInfo: { support: "UNSUPPORTED", state: "UNKNOWN", hasMore: null, nextCursor: null, retryCursor: null, returnedCount: releases.length },
            releases,
            ...sourceOverrides,
          },
        ],
      },
    },
  },
});

// Release search needs only this member-safe boolean, never a Source row.
const sourcesMock = (enabled: boolean) => ({
  request: { query: HasEnabledSourcesDocument, variables: {} },
  result: { data: { hasEnabledSources: enabled } },
});

const mount = (mocks: unknown[]) =>
  renderWithProviders(
    <ReleaseSearchDialog isOpen onOpenChange={() => {}} query="Andor" season={2} episode={5} libraryId="lib-1" target={{ showId: "show-1", episodeId: "ep-1" }} />,
    { mocks: [sourcesMock(true), ...mocks] as never },
  );

const releaseRow = async (title: string) => (await screen.findByText(title)).closest<HTMLElement>('[role="row"]')!;

describe("ReleaseSearchDialog results", () => {
  it("reads the member-safe capability without requiring an admin Source projection", async () => {
    const capability = vi.fn(() => ({ data: { hasEnabledSources: true } }));
    await renderWithProviders(
      <ReleaseSearchDialog isOpen onOpenChange={() => {}} query="Andor" season={2} episode={5} libraryId="lib-1" target={{ showId: "show-1", episodeId: "ep-1" }} />,
      { mocks: [{ ...sourcesMock(true), result: capability }, searchMock([release("member")])] as never },
    );
    await releaseRow("Release member");
    expect(capability).toHaveBeenCalledOnce();
  });

  it("lists the releases with the source, size, seeds and peers", async () => {
    await mount([searchMock([release("a")])]);
    const row = await releaseRow("Release a");
    expect(row).toHaveTextContent("Tracker");
    expect(row).toHaveTextContent("1.9 GB");
    expect(within(row).getByText("10")).toBeInTheDocument();
    expect(within(row).getByText("2")).toBeInTheDocument();
    expect(await screen.findByText("1 returned releases from 1 sources · latest response 120 ms")).toBeInTheDocument();
  });

  it("turns everything the backend parsed into tags", async () => {
    await mount([
      searchMock([
        release("a", {
          isFreeleech: true,
          parsed: { resolution: "2160p", codec: "hevc", hdrType: "hdr10", sourceType: "bluray", audio: "atmos", releaseGroup: "NTb", languages: ["en", "fr"], isSeasonPack: true, season: 2, isProper: true, isRepack: true },
        }),
      ]),
    ]);
    const row = await releaseRow("Release a");
    for (const tag of ["2160p", "bluray", "HEVC", "HDR10", "ATMOS", "English", "French", "Season 2 pack", "PROPER", "REPACK", "NTb", "Freeleech"]) {
      expect(within(row).getByText(tag)).toBeInTheDocument();
    }
  });

  it("labels a season pack without a season number", async () => {
    await mount([searchMock([release("a", { parsed: { isSeasonPack: true, season: null } })])]);
    expect(within(await releaseRow("Release a")).getByText("Season pack")).toBeInTheDocument();
  });

  it("shows the profile verdict and falls back to a dash without one", async () => {
    await mount([
      searchMock([
        release("a", { profileMatch: "optimal" }),
        release("b", { profileMatch: "suboptimal" }),
        release("c", { profileMatch: "rejected", rejectReasons: ["Below the minimum size", "Blocked group"] }),
        release("d", { profileMatch: null }),
      ]),
    ]);
    expect(within(await releaseRow("Release a")).getByText("Optimal")).toBeInTheDocument();
    expect(within(await releaseRow("Release b")).getByText("Suboptimal")).toBeInTheDocument();
    expect(within(await releaseRow("Release c")).getByText("Rejected")).toBeInTheDocument();
    expect(within(await releaseRow("Release d")).getByText("—")).toBeInTheDocument();
  });

  /*
   * HeroUI tooltips do not open under jsdom (react-aria's hover handling needs real pointer
   * events; a bare `<Tooltip><Button/></Tooltip>` probe fails the same way), so the reject-reason
   * tooltip is covered by the Playwright suite instead. The trigger is a real pressable Button
   * with the reasons in its accessible name, which is asserted below.
   */
  it.skip("explains a rejection on hover", async () => {
    await mount([searchMock([release("a", { profileMatch: "rejected", rejectReasons: ["Below the minimum size", "Blocked group"] })])]);
    await userEvent.hover(await screen.findByText("Rejected"));
    expect(await screen.findByText("Below the minimum size · Blocked group")).toBeInTheDocument();
  });

  it("exposes the rejection reasons on the verdict trigger", async () => {
    await mount([searchMock([release("a", { profileMatch: "rejected", rejectReasons: ["Below the minimum size", "Blocked group"] })])]);
    expect(await screen.findByRole("button", { name: "Why rejected: Below the minimum size, Blocked group" })).toBeInTheDocument();
  });

  it("orders by the profile verdict and then by seeders", async () => {
    await mount([
      searchMock([
        release("rejected", { profileMatch: "rejected", seeders: 900 }),
        release("optimal-low", { profileMatch: "optimal", seeders: 3 }),
        release("suboptimal", { profileMatch: "suboptimal", seeders: 500 }),
        release("optimal-high", { profileMatch: "optimal", seeders: 40 }),
      ]),
    ]);
    await screen.findByText("Release optimal-high");
    const titles = screen.getAllByRole("row").slice(1).map((row) => within(row).getByText(/^Release /).textContent);
    expect(titles).toEqual(["Release optimal-high", "Release optimal-low", "Release suboptimal", "Release rejected"]);
  });

  it("reports a source that failed while others answered", async () => {
    await mount([searchMock([release("a")], { error: "429 Too Many Requests" })]);
    expect(await screen.findByText("Tracker: 429 Too Many Requests")).toBeInTheDocument();
  });

  it("uses the member-safe false capability when nothing is configured to search", async () => {
    await renderWithProviders(
      <ReleaseSearchDialog isOpen onOpenChange={() => {}} query="Andor" season={2} episode={5} libraryId="lib-1" target={{ showId: "show-1", episodeId: "ep-1" }} />,
      { mocks: [sourcesMock(false), searchMock([])] as never },
    );
    expect(await screen.findByText("No sources configured")).toBeInTheDocument();
  });

  it("shows the search failure with a retry", async () => {
    await mount([{ request: { query: SearchSourcesDocument, variables: { input: searchInput() } }, error: new Error("Indexer timed out") }]);
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Indexer timed out");
  });

  it("says when nothing came back at all", async () => {
    await mount([searchMock([])]);
    expect(await screen.findByText("No releases found")).toBeInTheDocument();
  });
});

describe("ReleaseSearchDialog result scope", () => {
  it("labels source results as potentially partial", async () => {
    await mount([searchMock([release("single")])]);
    expect(await screen.findByText("Source results may be partial. Filters apply to returned releases.")).toBeInTheDocument();
  });

  it("counts all returned releases beyond the old 100 record limit", async () => {
    await mount([searchMock(Array.from({ length: 125 }, (_, index) => release(String(index))))]);
    expect(await screen.findByText("125 of 125")).toBeInTheDocument();
  });
});

describe("ReleaseSearchDialog filters", () => {
  const mixed = [
    release("uhd", { profileMatch: "optimal", seeders: 50, parsed: { resolution: "2160p", codec: "hevc", hdrType: "hdr10", languages: ["en"] } }),
    release("hd", { profileMatch: "optimal", seeders: 40, parsed: { resolution: "1080p", codec: "h264", languages: ["fr"] } }),
    release("pack", { profileMatch: "optimal", seeders: 30, isFreeleech: true, parsed: { resolution: "1080p", codec: "h264", isSeasonPack: true, season: 2, languages: ["en"] } }),
    release("dead", { profileMatch: "optimal", seeders: 0, parsed: { resolution: "720p", codec: "av1" } }),
  ];

  it("hides unseeded releases by default and counts what is shown", async () => {
    await mount([searchMock(mixed)]);
    expect(await screen.findByText("3 of 4")).toBeInTheDocument();
    expect(screen.queryByText("Release dead")).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "Seeded" }));
    expect(await screen.findByText("4 of 4")).toBeInTheDocument();
  });

  it("filters by resolution", async () => {
    await mount([searchMock(mixed)]);
    await screen.findByText("Release uhd");
    await userEvent.click(screen.getByRole("button", { name: "2160p" }));
    expect(screen.getByText("1 of 4")).toBeInTheDocument();
    expect(screen.queryByText("Release hd")).toBeNull();
  });

  it("filters by codec", async () => {
    await mount([searchMock(mixed)]);
    await screen.findByText("Release uhd");
    await userEvent.click(screen.getByRole("button", { name: "HEVC" }));
    expect(screen.getByText("1 of 4")).toBeInTheDocument();
    expect(screen.getByText("Release uhd")).toBeInTheDocument();
  });

  it("filters season packs in and out", async () => {
    await mount([searchMock(mixed)]);
    await screen.findByText("Release uhd");
    await userEvent.click(screen.getByRole("button", { name: "Season packs" }));
    expect(screen.getByText("1 of 4")).toBeInTheDocument();
    expect(screen.getByText("Release pack")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Single" }));
    expect(screen.getByText("2 of 4")).toBeInTheDocument();
    expect(screen.queryByText("Release pack")).toBeNull();
  });

  it("filters to HDR and to freeleech", async () => {
    await mount([searchMock(mixed)]);
    await screen.findByText("Release uhd");
    await userEvent.click(screen.getByRole("button", { name: "HDR" }));
    expect(screen.getByText("1 of 4")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "HDR" }));
    await userEvent.click(screen.getByRole("button", { name: "Freeleech" }));
    expect(screen.getByText("1 of 4")).toBeInTheDocument();
    expect(screen.getByText("Release pack")).toBeInTheDocument();
  });

  it("explains an over-tight filter", async () => {
    await mount([searchMock(mixed)]);
    await screen.findByText("Release uhd");
    await userEvent.click(screen.getByRole("button", { name: "2160p" }));
    await userEvent.click(screen.getByRole("button", { name: "Freeleech" }));
    expect(await screen.findByText("Nothing matches these filters")).toBeInTheDocument();
  });
});

describe("ReleaseSearchDialog grabbing", () => {
  const grabbed = () =>
    vi.fn(() => ({ data: { addTorrent: { __typename: "AddTorrentResult", success: true, error: null, torrent: null } } }));

  it("records a single episode grab against the episode", async () => {
    const add = grabbed();
    await mount([
      searchMock([release("single")]),
      {
        request: {
          query: AddTorrentDocument,
          variables: {
            input: {
              magnet: "magnet:?xt=urn:btih:single",
              url: null,
              feedItemId: null,
              libraryId: "lib-1",
              movieId: null,
              showId: "show-1",
              episodeId: "ep-1",
              albumId: null,
              trackId: null,
              audiobookId: null,
              chapterId: null,
              season: null,
              sourceIndexerId: "src-1",
              sourceUrl: "https://tracker/details/single",
            },
          },
        },
        result: add,
      },
    ]);
    await screen.findByText("Release single");
    await userEvent.click(screen.getByRole("button", { name: /Grab/ }));
    await vi.waitFor(() => expect(add).toHaveBeenCalled());
  });

  it("records a season pack against the season, not the episode", async () => {
    const add = grabbed();
    await mount([
      searchMock([release("pack", { magnetUri: null, parsed: { isSeasonPack: true, season: 4 } })]),
      {
        request: {
          query: AddTorrentDocument,
          variables: {
            input: {
              magnet: null,
              url: "https://tracker/pack.torrent",
              feedItemId: null,
              libraryId: "lib-1",
              movieId: null,
              showId: "show-1",
              episodeId: null,
              albumId: null,
              trackId: null,
              audiobookId: null,
              chapterId: null,
              season: 4,
              sourceIndexerId: "src-1",
              sourceUrl: "https://tracker/details/pack",
            },
          },
        },
        result: add,
      },
    ]);
    await screen.findByText("Release pack");
    await userEvent.click(screen.getByRole("button", { name: /Grab/ }));
    await vi.waitFor(() => expect(add).toHaveBeenCalled());
  });

  it("reports a refused grab", async () => {
    await mount([
      searchMock([release("single")]),
      {
        request: {
          query: AddTorrentDocument,
          variables: {
            input: {
              magnet: "magnet:?xt=urn:btih:single",
              url: null,
              feedItemId: null,
              libraryId: "lib-1",
              movieId: null,
              showId: "show-1",
              episodeId: "ep-1",
              albumId: null,
              trackId: null,
              audiobookId: null,
              chapterId: null,
              season: null,
              sourceIndexerId: "src-1",
              sourceUrl: "https://tracker/details/single",
            },
          },
        },
        result: { data: { addTorrent: { __typename: "AddTorrentResult", success: false, error: "Disk is full", torrent: null } } },
      },
    ]);
    await screen.findByText("Release single");
    await userEvent.click(screen.getByRole("button", { name: /Grab/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Disk is full");
  });

  it("cannot grab a release with neither a magnet nor a link", async () => {
    await mount([searchMock([release("empty", { magnetUri: null, link: null })])]);
    await screen.findByText("Release empty");
    expect(screen.getByRole("button", { name: "Grab" })).toBeDisabled();
  });
});

describe("release selection actions", () => {
  it("confirms a multi-release acquisition once and requests exactly the selected releases", async () => {
    const added = vi.fn(() => ({ data: { addTorrent: { __typename: "AddTorrentResult", success: true, error: null, torrent: null } } }));
    const addMock = (id: string) => ({
      request: { query: AddTorrentDocument, variables: { input: { magnet: `magnet:?xt=urn:btih:${id}`, url: null, feedItemId: null, libraryId: "lib-1", movieId: null, showId: "show-1", episodeId: "ep-1", albumId: null, trackId: null, audiobookId: null, chapterId: null, season: null, sourceIndexerId: "src-1", sourceUrl: `https://tracker/details/${id}` } } }, result: added,
    });
    await mount([searchMock([release("one"), release("two"), release("unselected")]), addMock("one"), addMock("two")]);
    await userEvent.click(await screen.findByRole("checkbox", { name: "Select Release one" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Release two" }));
    await userEvent.click(screen.getByRole("button", { name: "Actions for 2 selected releases" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Grab 2 releases" }));
    const confirmation = await screen.findByRole("alertdialog");
    expect(confirmation).toHaveTextContent("Each selected release starts a separate download");
    expect(added).not.toHaveBeenCalled();
    await userEvent.click(within(confirmation).getByRole("button", { name: "Download" }));
    await vi.waitFor(() => expect(added).toHaveBeenCalledTimes(2));
    await vi.waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  });
});

describe("ReleaseSearchDialog feed releases", () => {
  it("grabs a feed release by its catalogue item and never sends a link", async () => {
    const add = vi.fn(() => ({ data: { addTorrent: { __typename: "AddTorrentResult", success: true, error: null, torrent: null } } }));
    const feedRelease = release("feed-1", { sourceId: "feed-a", sourceName: "TV feed (feed)", fromFeed: true, feedItemId: "item-9", link: null, magnetUri: null });
    await mount([
      searchMock([feedRelease], { sourceId: "feed-a", sourceName: "TV feed (feed)", fromFeed: true }),
      {
        request: {
          query: AddTorrentDocument,
          variables: {
            input: {
              magnet: null,
              url: null,
              feedItemId: "item-9",
              libraryId: "lib-1",
              movieId: null,
              showId: "show-1",
              episodeId: "ep-1",
              albumId: null,
              trackId: null,
              audiobookId: null,
              chapterId: null,
              season: null,
              sourceIndexerId: null,
              sourceUrl: "https://tracker/details/feed-1",
            },
          },
        },
        result: add,
      },
    ]);
    await screen.findByText("Release feed-1");
    expect(screen.getByText("From feed")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /Grab/ }));
    await vi.waitFor(() => expect(add).toHaveBeenCalled());
  });
});

describe("ReleaseSearchDialog recommended releases", () => {
  const rec = (id: string, extra: Record<string, unknown> = {}) => ({
    __typename: "ReleaseRecommendation", id, userId: "me", libraryId: "lib-1", scope: "season_pack", status: "ready", targetLabel: "Andor Season 2", showId: "show-1", season: 2,
    episodeId: null, movieId: null, albumId: null, trackId: null, audiobookId: null, artistId: null, title: `Andor.S02.${id}.1080p.WEB-DL`, sourceKind: "indexer", sourceId: "src-1", sourceName: "Tracker",
    feedItemId: null, infoHash: `hash-${id}`, guid: `guid-${id}`, size: 12_000_000_000, seeders: 40, quality: "1080p WEB-DL", coversCount: 6, missingCount: 8, coversIds: "[]",
    reasons: ["1080p WEB-DL fits the 'HD' profile", "40 seeders"], note: null, planId: "plan-1", planPosition: 0, coversSeasons: "[2]", coversAlbums: null, fileIndices: null,
    selectedSize: null, downloadNote: null, rulesVersion: 4, foundAt: "2026-11-09T00:00:00Z", expiresAt: "2026-11-10T00:00:00Z", createdAt: "2026-11-09T00:00:00Z", updatedAt: "2026-11-09T00:00:00Z", ...extra,
  });
  const plan = rec("plan-1", { scope: "plan", planId: null, title: null, coversCount: 8, missingCount: 10, note: "Still missing: S03E01–E02 (no usable release found)" });
  const withPlan = (releases: unknown[], planRow: unknown = plan) => {
    recommendedState.value = { plan: planRow, releases, forTarget: releases.length, searching: false, none: null, rows: [planRow, ...releases], loaded: true, refetch: () => {} };
  };

  it("pins the recommended releases first, as ordinary rows that say Recommended and what they cover", async () => {
    withPlan([rec("a"), rec("b", { scope: "episode", targetLabel: "Andor S02E07", coversCount: 1, planPosition: 1 })]);
    // The live search also returned release a: it is shown once, as the recommended row.
    await mount([searchMock([release("live", { title: "Andor.S02E05.720p" }), release("dupe", { infoHash: "hash-a", title: "Andor.S02.a.1080p.WEB-DL" })])]);
    const row = await releaseRow("Andor.S02.a.1080p.WEB-DL");
    expect(within(row).getByText("Recommended")).toBeInTheDocument();
    expect(within(row).getByText("S02 · adds 6")).toBeInTheDocument();
    expect(row).toHaveTextContent("Tracker");
    expect(screen.getAllByText("Andor.S02.a.1080p.WEB-DL")).toHaveLength(1);
    expect(within(await releaseRow("Andor.S02.b.1080p.WEB-DL")).getByText("S02E07")).toBeInTheDocument();
    // Pinned above the search results.
    const rows = screen.getAllByRole("row").map((node) => node.textContent ?? "");
    const first = rows.findIndex((text) => text.includes("Andor.S02.a.1080p.WEB-DL"));
    const live = rows.findIndex((text) => text.includes("Andor.S02E05.720p"));
    expect(first).toBeGreaterThan(-1);
    expect(first).toBeLessThan(live);
    // What the plan leaves uncovered is a quiet line, not a panel.
    expect(screen.getByText("Still missing: S03E01–E02 (no usable release found)")).toBeInTheDocument();
  });

  it("downloads the whole plan from the table's Download recommended action, after confirming", async () => {
    withPlan([rec("a"), rec("b", { planPosition: 1 })]);
    const grabbedPlan = vi.fn(() => ({ data: { grabRecommendationPlan: { __typename: "GrabRecommendationPlanPayload", success: true, error: null, started: 2, failed: [], torrentIds: ["t1", "t2"] } } }));
    await mount([searchMock([]), { request: { query: GrabRecommendationPlanDocument, variables: { id: "plan-1" } }, result: grabbedPlan }]);
    await userEvent.click(await screen.findByRole("button", { name: /Download recommended: 2 releases cover 8 of 10 missing/ }));
    const confirmation = await screen.findByRole("dialog", { name: /Download 2 recommended releases/ });
    expect(grabbedPlan).not.toHaveBeenCalled();
    await userEvent.click(within(confirmation).getByRole("button", { name: "Download recommended" }));
    await vi.waitFor(() => expect(grabbedPlan).toHaveBeenCalledOnce());
    expect(await screen.findByText("Downloading 2 recommended releases")).toBeInTheDocument();
  });

  it("grabs one recommended row through its recommendation, never through a link", async () => {
    withPlan([rec("a")]);
    const grabbedOne = vi.fn(() => ({ data: { grabRecommendation: { __typename: "GrabRecommendationPayload", success: true, error: null, torrentId: "t1", torrentName: "Andor S02" } } }));
    await mount([searchMock([]), { request: { query: GrabRecommendationDocument, variables: { id: "a" } }, result: grabbedOne }]);
    const row = await releaseRow("Andor.S02.a.1080p.WEB-DL");
    await userEvent.click(within(row).getByRole("button", { name: /Grab/ }));
    await vi.waitFor(() => expect(grabbedOne).toHaveBeenCalledOnce());
    expect(await screen.findByText("Downloading Andor S02")).toBeInTheDocument();
  });

  it("offers no Download recommended without a plan", async () => {
    recommendedState.value = { plan: null, releases: [rec("movie", { scope: "movie", planId: null, showId: null, season: null, coversSeasons: null })], forTarget: 1, searching: false, none: null, rows: [], loaded: true, refetch: () => {} };
    await mount([searchMock([release("x")])]);
    expect(within(await releaseRow("Andor.S02.movie.1080p.WEB-DL")).getByText("Recommended")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Download recommended/ })).toBeNull();
  });
});

describe("ReleaseSearchDialog on a TV remote", () => {
  it("stops on the search controls and each release row, never on a column header or a row's own buttons", async () => {
    window.localStorage.setItem("librarian.inputMode", "tv");
    await renderWithProviders(
      <>
        <RemoteNavigation />
        <ReleaseSearchDialog isOpen onOpenChange={() => {}} query="Andor" season={2} episode={5} libraryId="lib-1" target={{ showId: "show-1", episodeId: "ep-1" }} />
      </>,
      { mocks: [sourcesMock(true), searchMock([release("a"), release("b", { profileMatch: "rejected", rejectReasons: ["Blocked group"] })])] as never, shell: true },
    );
    await releaseRow("Release b");
    const dialog = screen.getByRole("dialog");
    layoutColumn(dialog);
    const stops = remoteStopElements(dialog);
    expect(stops.some((stop) => stop.closest('[role="columnheader"], .fa-dt-head'))).toBe(false);
    const rows = stops.filter((stop) => stop.matches('.fa-dt-grid [role="row"][data-row-id]'));
    expect(rows.map((row) => row.textContent)).toEqual([expect.stringContaining("Release a"), expect.stringContaining("Release b")]);
    // The verdict's reasons button and the row's Grab and ⋯ are inside the row: not stops.
    expect(stops.filter((stop) => stop.closest('.fa-dt-grid [role="row"]') && !rows.includes(stop))).toEqual([]);
    // Walking down from the dialog's first stop reaches both releases.
    const walk = walkRemote(stops[0]!);
    expect(walk.filter((stop) => stop.startsWith("row:"))).toHaveLength(2);
  });
});
