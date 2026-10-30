import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState, type ComponentProps } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { EntityEpisodeUpdateDocument, EntityEpisodeUpdateManyDocument, type ContentStatus, type PlaybackProgressFieldsFragment } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { EpisodeList } from "../EpisodeList";
import type { EpisodeRow } from "../ShowPage";

let isAdmin = true;
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => isAdmin, useSession: () => ({ status: "authenticated", user: { id: "episode-user", role: isAdmin ? "admin" : "member" }, expiresAt: null }) }));

vi.mock("@/features/downloads/ReleaseSearchDialog", () => ({ ReleaseSearchDialog: vi.fn(() => null) }));

const episode = (season: number, number: number, overrides: Partial<EpisodeRow> = {}): EpisodeRow =>
  ({
    __typename: "Episode",
    id: `ep-${season}-${number}`,
    showId: "show-1",
    season,
    episode: number,
    absoluteNumber: null,
    title: `Episode ${number}`,
    overview: null,
    airDate: "2026-01-01",
    airStamp: null,
    runtime: 45,
    tvmazeId: null,
    tmdbId: null,
    tvdbId: null,
    wanted: false,
    ignored: null,
    mediaFileId: null,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    mediaFile: null,
    ...overrides,
  }) as EpisodeRow;

const bulkMock = (where: unknown, input: unknown, affectedCount = 3) => {
  const called = vi.fn(() => ({ data: { updateEpisodes: { __typename: "UpdateEpisodesResult", success: true, error: null, affectedCount } } }));
  return { called, mock: { request: { query: EntityEpisodeUpdateManyDocument, variables: { where, input } }, result: called } };
};

const singleMock = (id: string, input: unknown) => {
  const called = vi.fn(() => ({ data: { updateEpisode: { __typename: "UpdateEpisodeResult", success: true, error: null, episode: null } } }));
  return { called, mock: { request: { query: EntityEpisodeUpdateDocument, variables: { id, input } }, result: called } };
};

const mount = (episodes: EpisodeRow[], mocks: unknown[] = [], onChanged = vi.fn(), props: Partial<ComponentProps<typeof EpisodeList>> = {}) =>
  renderWithProviders(
    <EpisodeList
      episodes={episodes}
      statuses={new Map<string, ContentStatus>([["ep-1-1", "AVAILABLE"]])}
      progressByFile={new Map()}
      loading={false}
      onPlay={() => {}}
      showId="show-1"
      showName="Andor"
      libraryId="lib-1"
      onChanged={onChanged}
      {...props}
    />,
    { mocks: mocks as never },
  );

let tableWidth = 1920;

// jsdom has no layout. Supply dimensions while retaining the real Glass table,
// row/card renderers, action runner and context menus.
beforeEach(() => {
  isAdmin = true;
  tableWidth = 1920;
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    const width = this.hasAttribute("data-measure") ? 110 : tableWidth;
    return { x: 0, y: 0, top: 0, left: 0, bottom: 800, right: width, width, height: 800, toJSON: () => ({}) };
  });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 32 : tableWidth; });
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 48 : 800; });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(() => tableWidth);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(800);
});
afterEach(() => vi.restoreAllMocks());

const rowFor = (season: number, number: number) => within(screen.getByRole("grid", { name: `Season ${season} episodes` })).getByRole("row", { name: new RegExp(`S${String(season).padStart(2, "0")}E${String(number).padStart(2, "0")}`) });

describe("EpisodeList seasons", () => {
  it("groups episodes into seasons and summarises each one", async () => {
    await mount([episode(1, 1, { mediaFileId: "file-1" }), episode(1, 2, { wanted: true }), episode(1, 3), episode(2, 1, { ignored: true })]);
    const headings = screen.getAllByRole("heading", { level: 3 });
    expect(headings.map((heading) => heading.textContent)).toEqual(["Season 1", "Season 2"]);
    expect(headings[0]!.parentElement).toHaveTextContent("1 of 3 · 2 missing · 1 wanted");
    expect(headings[1]!.parentElement).toHaveTextContent("0 of 1 · 0 missing · 1 ignored");
  });

  it("labels season zero as specials", async () => {
    await mount([episode(0, 1)]);
    expect(screen.getByRole("heading", { level: 3, name: "Specials" })).toBeInTheDocument();
  });

  it("wants every missing episode in a season", async () => {
    const { called, mock } = bulkMock({ showId: { eq: "show-1" }, season: { eq: 1 }, mediaFileId: { isNull: true } }, { wanted: true, ignored: false });
    const onChanged = vi.fn();
    await mount([episode(1, 1, { mediaFileId: "file-1" }), episode(1, 2)], [mock], onChanged);
    await userEvent.click(screen.getByRole("button", { name: /Want all missing/ }));
    await vi.waitFor(() => expect(called).toHaveBeenCalled());
    expect(await screen.findByText("Season 1: missing episodes wanted (3)")).toBeInTheDocument();
    expect(onChanged).toHaveBeenCalled();
  });

  it("unwants the whole season, files included", async () => {
    const { called, mock } = bulkMock({ showId: { eq: "show-1" }, season: { eq: 1 } }, { wanted: false });
    await mount([episode(1, 1, { wanted: true })], [mock]);
    await userEvent.click(screen.getByRole("button", { name: /Unwant all/ }));
    await vi.waitFor(() => expect(called).toHaveBeenCalled());
  });

  it("ignores a season and clears wanted at the same time", async () => {
    const { called, mock } = bulkMock({ showId: { eq: "show-1" }, season: { eq: 1 } }, { ignored: true, wanted: false });
    await mount([episode(1, 1, { wanted: true })], [mock]);
    await userEvent.click(screen.getByRole("button", { name: /Ignore season/ }));
    await vi.waitFor(() => expect(called).toHaveBeenCalled());
  });

  it("offers to unignore a season that is ignored end to end", async () => {
    const { called, mock } = bulkMock({ showId: { eq: "show-1" }, season: { eq: 1 } }, { ignored: false });
    await mount([episode(1, 1, { ignored: true }), episode(1, 2, { ignored: true })], [mock]);
    expect(screen.queryByRole("button", { name: /Ignore season/ })).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: /Unignore season/ }));
    await vi.waitFor(() => expect(called).toHaveBeenCalled());
  });

  it("disables 'want all missing' when nothing is missing", async () => {
    await mount([episode(1, 1, { mediaFileId: "file-1" })]);
    const want = screen.getByRole("button", { name: /Want all missing/ });
    expect(want).toHaveAttribute("aria-disabled", "true");
    expect(want).toHaveAttribute("aria-description", "No missing episodes in this season");
    await userEvent.click(want);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Unwant all/ })).toBeEnabled();
  });

  it("reports a failed season update instead of throwing", async () => {
    const mock = {
      request: { query: EntityEpisodeUpdateManyDocument, variables: { where: { showId: { eq: "show-1" }, season: { eq: 1 } }, input: { wanted: false } } },
      result: { data: { updateEpisodes: { __typename: "UpdateEpisodesResult", success: false, error: "Season is locked", affectedCount: 0 } } },
    };
    const onChanged = vi.fn();
    await mount([episode(1, 1, { wanted: true })], [mock], onChanged);
    await userEvent.click(screen.getByRole("button", { name: /Unwant all/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Season is locked");
    expect(onChanged).not.toHaveBeenCalled();
    expect(screen.queryByText(/no longer wanted/)).not.toBeInTheDocument();
  });

  it("hides every action from a non-admin", async () => {
    isAdmin = false;
    await mount([episode(1, 1)]);
    expect(screen.queryByRole("button", { name: /Want all missing/ })).toBeNull();
    expect(screen.queryByRole("button", { name: "Want" })).toBeNull();
  });
});

describe("EpisodeList rows", () => {
  it("marks an episode wanted", async () => {
    const { called, mock } = singleMock("ep-1-1", { wanted: true });
    await mount([episode(1, 1)], [mock]);
    await userEvent.click(screen.getByRole("button", { name: "Want" }));
    await vi.waitFor(() => expect(called).toHaveBeenCalled());
  });

  it("ignores an episode and drops the wanted flag with it", async () => {
    const { called, mock } = singleMock("ep-1-1", { ignored: true, wanted: false });
    await mount([episode(1, 1, { wanted: true })], [mock]);
    await userEvent.click(screen.getByRole("button", { name: "Ignore" }));
    await vi.waitFor(() => expect(called).toHaveBeenCalled());
  });

  it("unignores an episode without touching anything else", async () => {
    const { called, mock } = singleMock("ep-1-1", { ignored: false });
    await mount([episode(1, 1, { ignored: true })], [mock]);
    await userEvent.click(screen.getByRole("button", { name: "Unignore" }));
    await vi.waitFor(() => expect(called).toHaveBeenCalled());
  });

  it("shows the status chip and file details, and only offers play when there is a file", async () => {
    await mount([
      episode(1, 1, {
        mediaFileId: "file-1",
        mediaFile: { __typename: "MediaFile", id: "file-1", duration: 2700, resolution: "1080p", videoCodec: "hevc", audioCodec: "eac3", size: 2_500_000_000, qualityStatus: "optimal" },
      } as Partial<EpisodeRow>),
      episode(1, 2),
    ]);
    const available = rowFor(1, 1);
    const missing = rowFor(1, 2);
    expect(within(available).getByRole("button", { name: "Play" })).toBeEnabled();
    expect(available).toHaveTextContent("1080p · HEVC · 2.3 GB");
    expect(available).toHaveTextContent("Optimal");
    expect(available).toHaveTextContent("Available");
    expect(within(missing).queryByRole("button", { name: "Play" })).not.toBeInTheDocument();
    expect(within(missing).getByRole("button", { name: "Want" })).toBeEnabled();
    expect(within(missing).getByRole("button", { name: "Find" })).toBeEnabled();
    expect(within(missing).getByRole("button", { name: "Ignore" })).toBeEnabled();
  });

  it("says so when a season has no episodes", async () => {
    await mount([]);
    expect(screen.getByText("No episodes in this season")).toBeInTheDocument();
  });
});

describe("EpisodeList Glass DataTable integration", () => {
  it("renders two season grids with available and missing rows, visible actions and page-owned scrolling", async () => {
    const onPlay = vi.fn();
    const available = episode(1, 1, { title: "First light", mediaFileId: "file-1" });
    const { container } = await mount([available, episode(1, 2), episode(2, 1)], [], vi.fn(), { onPlay });
    expect(screen.getAllByRole("grid")).toHaveLength(2);
    expect(within(screen.getByRole("grid", { name: "Season 1 episodes" })).getAllByRole("row")).toHaveLength(3);
    expect(within(screen.getByRole("grid", { name: "Season 2 episodes" })).getAllByRole("row")).toHaveLength(2);
    expect(container.querySelectorAll('[data-height="auto"]')).toHaveLength(2);
    expect(screen.queryByRole("button", { name: "Next page" })).not.toBeInTheDocument();
    expect(within(rowFor(1, 1)).queryByRole("button", { name: "Want" })).not.toBeInTheDocument();
    const play = within(rowFor(1, 1)).getByRole("button", { name: "Play" });
    expect(play).toBeVisible();
    play.focus();
    await userEvent.keyboard("{Enter}");
    expect(onPlay).toHaveBeenCalledExactlyOnceWith(available);
    expect(within(rowFor(2, 1)).queryByRole("button", { name: "Play" })).not.toBeInTheDocument();
    expect(within(rowFor(2, 1)).getByRole("button", { name: "Want" })).toBeVisible();
    expect(within(rowFor(2, 1)).getByRole("button", { name: "Find" })).toBeVisible();
    expect(within(rowFor(2, 1)).getByRole("button", { name: "Ignore" })).toBeVisible();
    expect(container.querySelector("button button")).not.toBeInTheDocument();
  });

  it("renders the library's narrow cards with play and missing-episode actions", async () => {
    tableWidth = 375;
    const onPlay = vi.fn();
    const available = episode(1, 1, { mediaFileId: "file-1" });
    const { container } = await mount([available, episode(1, 2), episode(2, 1)], [], vi.fn(), { onPlay });
    expect(container.querySelectorAll('[data-mode="cards"]')).toHaveLength(2);
    expect(screen.queryByRole("columnheader")).not.toBeInTheDocument();
    await userEvent.click(within(rowFor(1, 1)).getByRole("button", { name: "Play" }));
    expect(onPlay).toHaveBeenCalledExactlyOnceWith(available);
    const missing = rowFor(1, 2);
    expect(within(missing).getByRole("button", { name: "Want" })).toBeVisible();
    expect(within(missing).getByRole("button", { name: "Find" })).toBeVisible();
    expect(within(missing).getByRole("button", { name: "Ignore" })).toBeVisible();
    expect(rowFor(2, 1)).toHaveTextContent("No file");
  });

  it("preserves the focused Want control when mutation results update the row and its cells", async () => {
    const { mock } = singleMock("ep-1-1", { wanted: false });
    function ChangingEpisode() {
      const [row, setRow] = useState(episode(1, 1, { wanted: true }));
      return <EpisodeList episodes={[row]} statuses={new Map([[row.id, row.wanted ? "WANTED" : "MISSING"]])} progressByFile={new Map()} loading={false}
        onPlay={() => {}} showId="show-1" showName="Andor" libraryId="lib-1" onChanged={() => setRow((previous) => ({ ...previous, wanted: false }))} />;
    }
    await renderWithProviders(<ChangingEpisode />, { mocks: [mock] });
    const before = screen.getByRole("button", { name: "Wanted" });
    await userEvent.click(before);
    const after = await screen.findByRole("button", { name: "Want" });
    expect(after).toBe(before);
    expect(after).toHaveFocus();
    expect(screen.getByText("Missing", { exact: true })).toBeInTheDocument();
  });

  it("keeps row Play visible but disabled until saved playback state is ready", async () => {
    const onPlay = vi.fn();
    await mount([episode(1, 1, { mediaFileId: "file-1" })], [], vi.fn(), { onPlay, playbackUnavailable: "Loading watch progress" });
    const play = screen.getByRole("button", { name: "Play" });
    expect(play).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(play);
    expect(onPlay).not.toHaveBeenCalled();
  });

  it("retains member playback while hiding all acquisition and season mutations", async () => {
    isAdmin = false;
    const onPlay = vi.fn();
    const available = episode(1, 1, { mediaFileId: "file-1" });
    await mount([available, episode(1, 2), episode(2, 1, { ignored: true })], [], vi.fn(), { onPlay });
    expect(screen.queryByRole("button", { name: /^(Want|Wanted|Find|Ignore|Unignore|Want all missing|Unwant all|Ignore season|Unignore season)$/ })).not.toBeInTheDocument();
    await userEvent.click(within(rowFor(1, 1)).getByRole("button", { name: "Play" }));
    expect(onPlay).toHaveBeenCalledExactlyOnceWith(available);
    await userEvent.click(within(rowFor(1, 1)).getByRole("button", { name: /^Actions for/ }));
    expect(await screen.findByRole("menuitem", { name: "Play" })).toBeInTheDocument();
    expect(screen.queryByRole("menuitem", { name: /Want|Find|Ignore/ })).not.toBeInTheDocument();
  });

  it("uses the same wanted mutation from the row menu", async () => {
    const { called, mock } = singleMock("ep-2-1", { wanted: true });
    const onChanged = vi.fn();
    await mount([episode(1, 1, { mediaFileId: "file-1" }), episode(2, 1)], [mock], onChanged);
    await userEvent.click(within(rowFor(2, 1)).getByRole("button", { name: /^Actions for/ }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Want" }));
    await vi.waitFor(() => expect(called).toHaveBeenCalledOnce());
    expect(onChanged).toHaveBeenCalledOnce();
  });

  it("uses the shared actions from the keyboard context menu", async () => {
    const { called, mock } = singleMock("ep-1-1", { ignored: true, wanted: false });
    await mount([episode(1, 1, { wanted: true })], [mock]);
    const row = rowFor(1, 1);
    row.focus();
    await userEvent.keyboard("{Shift>}{F10}{/Shift}");
    await userEvent.click(await screen.findByRole("menuitem", { name: "Ignore" }));
    await vi.waitFor(() => expect(called).toHaveBeenCalledOnce());
  });

  it("passes the selected episode, show and library through to release search", async () => {
    await mount([episode(1, 1, { mediaFileId: "file-1" }), episode(2, 7)]);
    await userEvent.click(within(rowFor(2, 7)).getByRole("button", { name: "Find" }));
    expect(vi.mocked(ReleaseSearchDialog).mock.lastCall?.[0]).toMatchObject({
      isOpen: true, query: "Andor", season: 2, episode: 7, libraryId: "lib-1", target: { showId: "show-1", episodeId: "ep-2-7" },
    });
  });

  it("unwants a wanted episode without changing its ignore flag", async () => {
    const { called, mock } = singleMock("ep-1-1", { wanted: false });
    const onChanged = vi.fn();
    await mount([episode(1, 1, { wanted: true })], [mock], onChanged);
    await userEvent.click(within(rowFor(1, 1)).getByRole("button", { name: "Wanted" }));
    await vi.waitFor(() => expect(called).toHaveBeenCalledOnce());
    expect(onChanged).toHaveBeenCalledOnce();
  });

  it("scopes a missing-only bulk update to the selected season and show", async () => {
    const { called, mock } = bulkMock({ showId: { eq: "show-1" }, season: { eq: 2 }, mediaFileId: { isNull: true } }, { wanted: true, ignored: false }, 1);
    await mount([episode(1, 1), episode(2, 1, { mediaFileId: "file-2" }), episode(2, 2)], [mock]);
    await userEvent.click(within(screen.getByRole("region", { name: "Season 2 episodes" })).getByRole("button", { name: "Want all missing" }));
    await vi.waitFor(() => expect(called).toHaveBeenCalledOnce());
    expect(await screen.findByText("Season 2: missing episodes wanted (1)")).toBeInTheDocument();
  });

  it("rejects a failed single mutation without notifying the parent or reporting success", async () => {
    const mock = {
      request: { query: EntityEpisodeUpdateDocument, variables: { id: "ep-1-1", input: { wanted: true } } },
      result: { data: { updateEpisode: { __typename: "UpdateEpisodeResult", success: false, error: "Episode is locked", episode: null } } },
    };
    const onChanged = vi.fn();
    await mount([episode(1, 1)], [mock], onChanged);
    await userEvent.click(within(rowFor(1, 1)).getByRole("button", { name: "Want" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Episode is locked");
    expect(screen.getAllByRole("alert")).toHaveLength(1);
    expect(onChanged).not.toHaveBeenCalled();
  });

  it("shows accessible saved progress, watched state, dates and runtime in real table cells", async () => {
    const progressByFile = new Map<string | null | undefined, PlaybackProgressFieldsFragment>([
      ["file-1", { currentPosition: 1350, duration: 2700, isWatched: false } as PlaybackProgressFieldsFragment],
      ["file-2", { currentPosition: 1, duration: null, isWatched: true } as PlaybackProgressFieldsFragment],
    ]);
    await mount([episode(1, 1, { mediaFileId: "file-1" }), episode(1, 2, { mediaFileId: "file-2" })], [], vi.fn(), { progressByFile });
    const progress = within(rowFor(1, 1)).getByRole("meter", { name: "Playback progress for S01E01" });
    expect(progress).toHaveAttribute("aria-valuenow", "50");
    expect(progress).toHaveAttribute("aria-valuetext", "50%");
    expect(within(rowFor(1, 2)).getByRole("meter", { name: "Playback progress for S01E02" })).toHaveAttribute("aria-valuenow", "100");
    expect(rowFor(1, 2)).toHaveTextContent("Watched");
    expect(rowFor(1, 1)).toHaveTextContent(/Jan 1, 2026/);
    expect(rowFor(1, 1)).toHaveTextContent(/45\s*min/);
  });

  it("uses user/show/season-specific saved table preferences", async () => {
    window.localStorage.setItem("glass-ux.table.episodes:episode-user:lib-1:show-1:season:1.v1", JSON.stringify({ density: "compact", widths: {}, hidden: ["overview"], pinned: null }));
    await mount([episode(1, 1), episode(2, 1)]);
    expect(screen.getByRole("grid", { name: "Season 1 episodes" }).closest('[data-height="auto"]')).toHaveAttribute("data-density", "compact");
    expect(screen.getByRole("grid", { name: "Season 2 episodes" }).closest('[data-height="auto"]')).not.toHaveAttribute("data-density");
  });

  it("keeps initial loading inside an accessible table instead of exposing empty content", async () => {
    const { container } = await mount([], [], vi.fn(), { loading: true });
    expect(screen.getByRole("grid", { name: "Andor episodes" })).toHaveAttribute("aria-busy", "true");
    expect(container.querySelector('[data-height="auto"]')).toBeInTheDocument();
    expect(screen.queryByText("No episodes in this season")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Want all missing|Play/ })).not.toBeInTheDocument();
  });
});

describe("EpisodeList row view", () => {
  beforeEach(() => { isAdmin = true; });

  it("plays a tile with a file and opens actions from a missing one", async () => {
    const onPlay = vi.fn();
    const playable = episode(1, 1, { mediaFileId: "file-1", title: "Pilot" });
    const missing = episode(1, 2, { title: "Second" });
    await mount([playable, missing], [], vi.fn(), { view: "row", onPlay });
    await userEvent.click(screen.getByRole("button", { name: "Play S01E01 · Pilot" }));
    expect(onPlay).toHaveBeenCalledWith(playable);
    await userEvent.click(screen.getByRole("button", { name: /^S01E02 · Second/ }));
    const menu = await screen.findByRole("menu");
    expect(within(menu).getByRole("menuitem", { name: "Find" })).toBeInTheDocument();
  });

  it("enters the row at the next episode to watch and shows its summary", async () => {
    const watched = episode(1, 1, { mediaFileId: "file-1", title: "Pilot" });
    const next = episode(1, 2, { mediaFileId: "file-2", title: "Second", overview: "Things happen." });
    const progress = new Map([["file-1", { __typename: "PlaybackProgress", id: "p1", mediaFileId: "file-1", currentPosition: 100, duration: 100, isWatched: true } as unknown as PlaybackProgressFieldsFragment]]);
    await mount([watched, next], [], vi.fn(), { view: "row", progressByFile: progress });
    expect(screen.getByRole("button", { name: "Play S01E02 · Second" })).toHaveAttribute("data-nav-default");
    expect(screen.getByText("Things happen.")).toBeInTheDocument();
  });

  it("keeps a member's missing episode browsable without an empty menu", async () => {
    isAdmin = false;
    await mount([episode(1, 1, { title: "Later" })], [], vi.fn(), { view: "row" });
    const tile = screen.getByRole("button", { name: /^S01E01 · Later/ });
    expect(tile).toHaveAttribute("aria-disabled", "true");
    expect(tile).not.toHaveAttribute("aria-haspopup");
  });
});
