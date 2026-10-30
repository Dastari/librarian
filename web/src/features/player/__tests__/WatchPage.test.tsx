import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ShowEpisodesDocument, type ShowEpisodesQuery } from "@/graphql/generated/graphql";
import { mediaHookHarness } from "@/features/media/__tests__/mediaHookHarness";

import { playerStore, type PlayItem } from "../store";
import { VideoPlayer } from "../VideoPlayer";
import { WatchPage } from "../WatchPage";

const { navigate } = vi.hoisted(() => ({ navigate: vi.fn() }));
vi.mock("@tanstack/react-router", () => ({ useNavigate: () => navigate }));
// Replace only the media engine boundary; WatchPage, Apollo, the complete-show
// loader, and the external player store retain their production behavior.
vi.mock("../VideoPlayer", () => ({ VideoPlayer: vi.fn(() => null) }));

type Episode = ShowEpisodesQuery["episodes"]["edges"][number]["node"];
const episodes: Episode[] = Array.from({ length: 103 }, (_, index) => ({
  id: `episode-${index + 1}`, showId: "show-a", season: Math.ceil((index + 1) / 20), episode: index + 1,
  title: `Episode ${index + 1}`, absoluteNumber: null, overview: null, airDate: null, airStamp: null,
  runtime: null, stillUrl: null, tvmazeId: null, tmdbId: null, tvdbId: null, wanted: false, ignored: false,
  mediaFileId: `file-${index + 1}`, mediaFile: null, createdAt: "2026-01-01", updatedAt: "2026-01-01",
}));
const page = (rows: Episode[], hasNextPage = false) => ({
  episodes: {
    __typename: "EpisodeConnection",
    edges: rows.map((node) => ({ __typename: "EpisodeEdge", node: { __typename: "Episode", ...node } })),
    pageInfo: { __typename: "PageInfo", hasNextPage, totalCount: 103 },
  },
});
const videoProps = () => vi.mocked(VideoPlayer).mock.calls.at(-1)![0];
const currentItem = (number: number): PlayItem => ({
  mediaFileId: `file-${number}`, title: "Example Show", artwork: "/show-poster.jpg",
  entity: { kind: "episode", id: `episode-${number}` }, href: "/shows/show-a",
});

beforeEach(() => {
  vi.clearAllMocks();
  playerStore.setVideo(null);
  playerStore.stopAudio();
});
afterEach(() => {
  cleanup();
  playerStore.setVideo(null);
  playerStore.stopAudio();
});

async function loadCompleteShow(current: number, rows = episodes) {
  const harness = mediaHookHarness();
  const item = currentItem(current);
  playerStore.setVideo(item);
  render(<WatchPage mediaFileId={item.mediaFileId} onBack={vi.fn()} />, { wrapper: harness.wrapper });
  await waitFor(() => expect(harness.requests).toHaveLength(1));
  expect(harness.requests[0]!.operation.operationName).toBe("ShowEpisodes");
  expect(harness.requests[0]!.operation.variables).toEqual({ showId: "show-a" });
  expect(videoProps().onNext).toBeUndefined();
  expect(videoProps().onEnded).toBeUndefined();

  await act(async () => harness.requests[0]!.resolve(page(rows.slice(0, 100), true)));
  await waitFor(() => expect(harness.requests).toHaveLength(2));
  expect(harness.requests[1]!.operation.operationName).toBe("ShowEpisodesPage");
  expect(harness.requests[1]!.operation.variables).toEqual({ showId: "show-a", offset: 100 });
  expect(videoProps().onNext).toBeUndefined();
  expect(videoProps().onEnded).toBeUndefined();
  expect(videoProps().nextLabel).toBeUndefined();
  expect(playerStore.getSnapshot().video).toEqual(item);
  expect(navigate).not.toHaveBeenCalled();

  await act(async () => harness.requests[1]!.resolve(page(rows.slice(100))));
  await waitFor(() => expect(harness.client.readQuery({ query: ShowEpisodesDocument, variables: { showId: "show-a" } })?.episodes.edges).toHaveLength(103));
  expect(harness.client.readQuery({ query: ShowEpisodesDocument, variables: { showId: "show-a" } })?.episodes.pageInfo.hasNextPage).toBe(false);
  return { ...harness, item };
}

describe("WatchPage next episode", () => {
  it("plays the next episode beyond the first 100 after the complete show loads", async () => {
    await loadCompleteShow(101);
    await waitFor(() => expect(videoProps().onNext).toBeTypeOf("function"));
    expect(videoProps().nextLabel).toBe("Next: E102");
    expect(videoProps().onEnded).toBe(videoProps().onNext);
    act(() => videoProps().onNext!());
    expect(playerStore.getSnapshot().video).toEqual({
      ...currentItem(102), subtitle: "S06 · E102  Episode 102",
    });
    expect(navigate).toHaveBeenCalledExactlyOnceWith({
      to: "/watch/$mediaFileId", params: { mediaFileId: "file-102" }, replace: true,
    });
  });

  it("skips unavailable later episodes when automatically advancing", async () => {
    await loadCompleteShow(101, episodes.map((episode) => episode.id === "episode-102" ? { ...episode, mediaFileId: null } : episode));
    await waitFor(() => expect(videoProps().onEnded).toBeTypeOf("function"));
    expect(videoProps().nextLabel).toBe("Next: E103");
    act(() => videoProps().onEnded!());
    expect(playerStore.getSnapshot().video?.entity.id).toBe("episode-103");
    expect(playerStore.getSnapshot().video?.mediaFileId).toBe("file-103");
    expect(navigate).toHaveBeenCalledExactlyOnceWith({
      to: "/watch/$mediaFileId", params: { mediaFileId: "file-103" }, replace: true,
    });
  });

  it.each([
    [103, "the last episode"],
    [999, "an episode absent from the complete connection"],
  ] as const)("offers no Next for %s (%s), without wrapping to the first episode", async (current, _description) => {
    const { item } = await loadCompleteShow(current);
    expect(videoProps().onNext).toBeUndefined();
    expect(videoProps().onEnded).toBeUndefined();
    expect(videoProps().nextLabel).toBeUndefined();
    expect(videoProps().item).toEqual(item);
    expect(playerStore.getSnapshot().video).toEqual(item);
    expect(navigate).not.toHaveBeenCalled();
  });
});
