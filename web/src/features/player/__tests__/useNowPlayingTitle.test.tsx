import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { playerStore, type PlayItem } from "../store";
import { NowPlayingTitle, nowPlayingTitle } from "../useNowPlayingTitle";

const episode: PlayItem = { mediaFileId: "f1", title: "Stargate Atlantis", subtitle: "S05 · E03  The Seer", entity: { kind: "episode", id: "e1" } };
const track: PlayItem = { mediaFileId: "f2", title: "Teardrop", subtitle: "Massive Attack", entity: { kind: "track", id: "t1" } };
const book: PlayItem = {
  mediaFileId: "f3",
  title: "Old Man's War",
  subtitle: "John Scalzi",
  entity: { kind: "chapter", id: "c1" },
  chapters: [{ start: 0, title: "Opening Credits" }, { start: 120, title: "Chapter 1" }],
};

function playVideo(video: HTMLVideoElement, playing: boolean) {
  Object.defineProperty(video, "paused", { configurable: true, get: () => !playing });
  video.dispatchEvent(new Event(playing ? "playing" : "pause"));
}

beforeEach(() => {
  document.title = "Librarian";
  playerStore.stopAudio();
  playerStore.setVideo(null);
});

afterEach(() => {
  cleanup();
  playerStore.stopAudio();
  playerStore.setVideo(null);
  document.querySelectorAll("video").forEach((video) => video.remove());
});

describe("nowPlayingTitle", () => {
  it("names an episode with its code and title, marked playing or paused", () => {
    const state = { ...playerStore.getSnapshot(), video: episode };
    expect(nowPlayingTitle(state, true)).toBe("▶ Stargate Atlantis · S05 · E03 · The Seer");
    expect(nowPlayingTitle(state, false)).toBe("⏸ Stargate Atlantis · S05 · E03 · The Seer");
  });

  it("names an audiobook's current chapter", () => {
    const base = playerStore.getSnapshot();
    const state = { ...base, queue: [book], index: 0, transport: { ...base.transport, playing: true, position: 200 } };
    expect(nowPlayingTitle(state, false)).toBe("▶ Old Man's War · Chapter 1");
  });

  it("is null when nothing is loaded", () => {
    expect(nowPlayingTitle(playerStore.getSnapshot(), false)).toBeNull();
  });
});

describe("NowPlayingTitle", () => {
  it("follows audio play and pause, ignores position ticks and restores the page title", () => {
    render(<NowPlayingTitle />);
    expect(document.title).toBe("Librarian");

    act(() => playerStore.playQueue([track]));
    expect(document.title).toBe("▶ Teardrop · Massive Attack");

    act(() => playerStore.updateTransport({ playing: false }));
    expect(document.title).toBe("⏸ Teardrop · Massive Attack");
    act(() => playerStore.updateTransport({ playing: true, position: 42 }));
    expect(document.title).toBe("▶ Teardrop · Massive Attack");

    act(() => playerStore.stopAudio());
    expect(document.title).toBe("Librarian");
  });

  it("moves to the next chapter of an audiobook as playback crosses it", () => {
    render(<NowPlayingTitle />);
    act(() => playerStore.playQueue([book]));
    expect(document.title).toBe("▶ Old Man's War · Opening Credits");
    act(() => playerStore.updateTransport({ position: 130 }));
    expect(document.title).toBe("▶ Old Man's War · Chapter 1");
  });

  it("follows the video element's play state", () => {
    render(<NowPlayingTitle />);
    const video = document.createElement("video");
    document.body.append(video);
    act(() => playerStore.setVideo(episode));
    expect(document.title).toBe("⏸ Stargate Atlantis · S05 · E03 · The Seer");
    act(() => playVideo(video, true));
    expect(document.title).toBe("▶ Stargate Atlantis · S05 · E03 · The Seer");
    act(() => playVideo(video, false));
    expect(document.title).toBe("⏸ Stargate Atlantis · S05 · E03 · The Seer");
    act(() => playerStore.setVideo(null));
    expect(document.title).toBe("Librarian");
  });

  it("keeps a title a route sets while something plays, and returns to it", async () => {
    render(<NowPlayingTitle />);
    act(() => playerStore.playQueue([track]));
    document.title = "Settings · Librarian";
    // The observer puts the playing title back and remembers the route's.
    await act(async () => { await Promise.resolve(); });
    expect(document.title).toBe("▶ Teardrop · Massive Attack");
    act(() => playerStore.stopAudio());
    expect(document.title).toBe("Settings · Librarian");
  });
});
