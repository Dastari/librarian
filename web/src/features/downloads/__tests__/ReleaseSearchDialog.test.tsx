import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { AddTorrentDocument, EntitySourceListDocument, SearchSourcesDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { ReleaseSearchDialog } from "../ReleaseSearchDialog";

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
  guid,
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
  limit: 100,
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
        sources: [
          {
            __typename: "SourceSearchResult",
            sourceId: "src-1",
            sourceName: "Tracker",
            elapsedMs: 120,
            fromCache: false,
            error: null,
            releases,
            ...sourceOverrides,
          },
        ],
      },
    },
  },
});

const source = (id: string) => ({
  __typename: "SourceEdge" as const,
  node: {
    __typename: "Source" as const,
    id,
    name: "Tracker",
    sourceType: "torznab",
    definitionId: "generic",
    enabled: true,
    priority: 1,
    mediaTypes: ["tv"],
    siteUrl: "https://tracker",
    supportsSearch: true,
    supportsTvSearch: true,
    supportsMovieSearch: true,
    supportsMusicSearch: false,
    supportsBookSearch: false,
    settings: "{}",
    lastError: null,
    errorCount: 0,
    lastSuccessAt: null,
    lastErrorAt: null,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
});

const sourcesMock = (edges: unknown[]) => ({
  request: { query: EntitySourceListDocument, variables: { where: { enabled: { eq: true } }, page: { limit: 50, offset: 0 } } },
  result: {
    data: { sources: { __typename: "SourceConnection", edges, pageInfo: { __typename: "PageInfo", hasNextPage: false, hasPreviousPage: false, totalCount: edges.length } } },
  },
});

const mount = (mocks: unknown[]) =>
  renderWithProviders(
    <ReleaseSearchDialog isOpen onOpenChange={() => {}} query="Andor" season={2} episode={5} libraryId="lib-1" target={{ showId: "show-1", episodeId: "ep-1" }} />,
    { mocks: [sourcesMock([source("src-1")]), ...mocks] as never },
  );

const releaseRow = async (title: string) => (await screen.findByText(title)).closest("tr")!;

describe("ReleaseSearchDialog results", () => {
  it("lists the releases with the source, size, seeds and peers", async () => {
    await mount([searchMock([release("a")])]);
    const row = await releaseRow("Release a");
    expect(row).toHaveTextContent("Tracker");
    expect(row).toHaveTextContent("1.9 GB");
    expect(within(row).getByText("10")).toBeInTheDocument();
    expect(within(row).getByText("2")).toBeInTheDocument();
    expect(await screen.findByText("1 releases from 1 sources in 120 ms")).toBeInTheDocument();
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
    const titles = screen.getAllByRole("row").slice(1).map((row) => row.querySelector("span.break-words")?.textContent);
    expect(titles).toEqual(["Release optimal-high", "Release optimal-low", "Release suboptimal", "Release rejected"]);
  });

  it("reports a source that failed while others answered", async () => {
    await mount([searchMock([release("a")], { error: "429 Too Many Requests" })]);
    expect(await screen.findByText("Tracker: 429 Too Many Requests")).toBeInTheDocument();
  });

  it("points at settings when nothing is configured to search", async () => {
    await renderWithProviders(
      <ReleaseSearchDialog isOpen onOpenChange={() => {}} query="Andor" season={2} episode={5} libraryId="lib-1" target={{ showId: "show-1", episodeId: "ep-1" }} />,
      { mocks: [sourcesMock([]), searchMock([])] as never },
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
    await userEvent.click(screen.getByRole("radio", { name: "2160p" }));
    expect(screen.getByText("1 of 4")).toBeInTheDocument();
    expect(screen.queryByText("Release hd")).toBeNull();
  });

  it("filters by codec", async () => {
    await mount([searchMock(mixed)]);
    await screen.findByText("Release uhd");
    await userEvent.click(screen.getByRole("radio", { name: "HEVC" }));
    expect(screen.getByText("1 of 4")).toBeInTheDocument();
    expect(screen.getByText("Release uhd")).toBeInTheDocument();
  });

  it("filters season packs in and out", async () => {
    await mount([searchMock(mixed)]);
    await screen.findByText("Release uhd");
    await userEvent.click(screen.getByRole("radio", { name: "Season packs" }));
    expect(screen.getByText("1 of 4")).toBeInTheDocument();
    expect(screen.getByText("Release pack")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("radio", { name: "Single" }));
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
    await userEvent.click(screen.getByRole("radio", { name: "2160p" }));
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
    expect(await screen.findByText("Disk is full")).toBeInTheDocument();
  });

  it("cannot grab a release with neither a magnet nor a link", async () => {
    await mount([searchMock([release("empty", { magnetUri: null, link: null })])]);
    await screen.findByText("Release empty");
    expect(screen.getByText("Grab").closest("button")).toBeDisabled();
  });
});
