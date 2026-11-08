import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { mediaHookHarness } from "@/features/media/__tests__/mediaHookHarness";

import { playerStore, type PlayItem } from "../store";
import { VideoPlayer } from "../VideoPlayer";
import { WatchPage } from "../WatchPage";

vi.mock("@tanstack/react-router", () => ({ useNavigate: () => vi.fn() }));
// Replace only the media engine boundary; WatchPage, Apollo and the external player store keep their production behavior.
vi.mock("../VideoPlayer", () => ({ VideoPlayer: vi.fn(() => null) }));

const videoProps = () => vi.mocked(VideoPlayer).mock.calls.at(-1)![0];
const episode = (number: number): PlayItem => ({
  mediaFileId: `file-${number}`, title: "Example Show", subtitle: `S01 · E${String(number).padStart(2, "0")}  Episode ${number}`,
  entity: { kind: "episode", id: `episode-${number}` }, showId: "show-a", href: "/shows/show-a",
});
const track: PlayItem = { mediaFileId: "song", title: "Song", entity: { kind: "track", id: "track-1" }, albumId: "album-1" };

beforeEach(() => {
  vi.clearAllMocks();
  playerStore.setContinuationResolver(null);
  playerStore.setVideo(null);
  playerStore.stopAudio();
});
afterEach(() => {
  cleanup();
  playerStore.setContinuationResolver(null);
  playerStore.setVideo(null);
  playerStore.stopAudio();
});

function watch(file: string) {
  const harness = mediaHookHarness();
  render(<WatchPage mediaFileId={file} onBack={vi.fn()} />, { wrapper: harness.wrapper });
}

describe("WatchPage and the play queue", () => {
  it("offers the queue's next entry as Next and plays it when the video ends", () => {
    playerStore.playQueue([episode(1), episode(2)]);
    watch("file-1");
    expect(videoProps().nextLabel).toBe("Next: E02");
    expect(videoProps().onNext).toBeTypeOf("function");
    act(() => videoProps().onEnded!());
    expect(playerStore.getSnapshot().index).toBe(1);
    expect(playerStore.getSnapshot().video?.entity.id).toBe("episode-2");
  });

  it("goes on to audio when the next entry is audio, which switches the video off", () => {
    playerStore.playQueue([episode(1), track]);
    watch("file-1");
    expect(videoProps().nextLabel).toBe("Next: Song");
    act(() => videoProps().onNext!());
    const state = playerStore.getSnapshot();
    expect(state.video).toBeNull();
    expect(state.queue[state.index]!.mediaFileId).toBe("song");
    expect(state.transport).toMatchObject({ playing: true, loading: true });
  });

  it("offers Next at the end of the queue only while something may continue it", async () => {
    playerStore.setContinuationResolver(async () => []);
    playerStore.playQueue([episode(3)]);
    watch("file-3");
    expect(videoProps().nextLabel).toBe("Next episode");
    await act(async () => { await playerStore.advance(); });
    expect(videoProps().onNext).toBeUndefined();
    expect(playerStore.getSnapshot().index).toBe(0);
  });

  it("carries on with the continuation's episodes when the last queued one ends", async () => {
    playerStore.setContinuationResolver(async (last) => (last.entity.id === "episode-3" ? [episode(4)] : []));
    playerStore.playQueue([episode(3)]);
    watch("file-3");
    await act(async () => { await playerStore.advance(); });
    expect(playerStore.getSnapshot().video?.entity.id).toBe("episode-4");
    expect(playerStore.getSnapshot().queue).toHaveLength(2);
  });

  it("offers no Next for a movie, which nothing follows", () => {
    playerStore.setContinuationResolver(async () => []);
    playerStore.playQueue([{ mediaFileId: "film", title: "Film", entity: { kind: "movie", id: "movie-1" } }]);
    watch("film");
    expect(videoProps().onNext).toBeUndefined();
  });
});
