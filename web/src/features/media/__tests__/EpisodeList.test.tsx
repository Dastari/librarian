import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { EntityEpisodeUpdateDocument, EntityEpisodeUpdateManyDocument, type ContentStatus } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { EpisodeList } from "../EpisodeList";
import type { EpisodeRow } from "../ShowPage";

let isAdmin = true;
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => isAdmin, useSession: () => ({ status: "authenticated", user: null, expiresAt: null }) }));

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

const mount = (episodes: EpisodeRow[], mocks: unknown[] = [], onChanged = vi.fn()) =>
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
    />,
    { mocks: mocks as never },
  );

beforeEach(() => {
  isAdmin = true;
});

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
    expect(screen.getByRole("button", { name: /Want all missing/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Unwant all/ })).toBeEnabled();
  });

  it("reports a failed season update instead of throwing", async () => {
    const mock = {
      request: { query: EntityEpisodeUpdateManyDocument, variables: { where: { showId: { eq: "show-1" }, season: { eq: 1 } }, input: { wanted: false } } },
      result: { data: { updateEpisodes: { __typename: "UpdateEpisodesResult", success: false, error: "Season is locked", affectedCount: 0 } } },
    };
    await mount([episode(1, 1, { wanted: true })], [mock]);
    await userEvent.click(screen.getByRole("button", { name: /Unwant all/ }));
    expect(await screen.findByText("Season is locked")).toBeInTheDocument();
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
    const items = screen.getAllByRole("listitem");
    expect(within(items[0]!).getByRole("button", { name: "Play episode 1" })).toBeEnabled();
    expect(items[0]!).toHaveTextContent("1080p · HEVC · 2.3 GB");
    expect(items[0]!).toHaveTextContent("Available");
    expect(within(items[1]!).getAllByRole("button")[0]).toBeDisabled();
  });

  it("says so when a season has no episodes", async () => {
    await mount([]);
    expect(screen.getByText("No episodes in this season")).toBeInTheDocument();
  });
});
