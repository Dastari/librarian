import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { audioScrub, handleMediaKey, useGlobalMediaKeys, videoKeyTarget, type PlayerKeyTarget } from "../mediaKeys";
import { playerStore } from "../store";
import { audioCommands } from "../usePlayer";

const track = (id: string) => ({ mediaFileId: id, title: `Track ${id}`, entity: { kind: "track" as const, id } });

afterEach(() => {
  act(() => playerStore.stopAudio());
  playerStore.setVideo(null);
  videoKeyTarget.current = null;
  audioScrub.current = null;
  delete audioCommands.seek;
});

describe("media keys", () => {
  it("drive the audio player without anything focused", () => {
    playerStore.playQueue([track("a"), track("b")]);
    playerStore.updateTransport({ playing: true });
    expect(handleMediaKey("MediaPlayPause")).toBe(true);
    expect(playerStore.getSnapshot().transport.playing).toBe(false);
    handleMediaKey("MediaPlay");
    expect(playerStore.getSnapshot().transport.playing).toBe(true);
    handleMediaKey("MediaPause");
    expect(playerStore.getSnapshot().transport.playing).toBe(false);
    handleMediaKey("MediaTrackNext");
    expect(playerStore.getSnapshot().index).toBe(1);
    handleMediaKey("MediaTrackPrevious");
    expect(playerStore.getSnapshot().index).toBe(0);
    handleMediaKey("MediaStop");
    expect(playerStore.getSnapshot().queue).toEqual([]);
  });

  it("scrub audio with Rewind and Fast-forward through the seek bar, or seek at once without one", () => {
    playerStore.playQueue([track("a")]);
    playerStore.updateTransport({ position: 40 });
    const seek = vi.fn();
    audioCommands.seek = seek;
    handleMediaKey("MediaFastForward");
    expect(seek).toHaveBeenCalledWith(50);
    const scrub = vi.fn();
    audioScrub.current = scrub;
    handleMediaKey("MediaRewind", true);
    expect(scrub).toHaveBeenCalledWith(-1, true);
  });

  it("go to the video player while it holds the screen", () => {
    const video: PlayerKeyTarget = { toggle: vi.fn(), play: vi.fn(), pause: vi.fn(), stop: vi.fn(), next: vi.fn(), previous: vi.fn(), scrub: vi.fn() };
    videoKeyTarget.current = video;
    playerStore.playQueue([{ mediaFileId: "film", title: "Film", entity: { kind: "movie", id: "m" } }]);
    handleMediaKey("MediaPlayPause");
    handleMediaKey("MediaFastForward", true);
    handleMediaKey("MediaTrackNext");
    expect(video.toggle).toHaveBeenCalledTimes(1);
    expect(video.scrub).toHaveBeenCalledWith(1, true);
    expect(video.next).toHaveBeenCalledTimes(1);
  });

  it("are ignored with nothing playing, and other keys are not theirs", () => {
    expect(handleMediaKey("MediaPlayPause")).toBe(false);
    playerStore.playQueue([track("a")]);
    expect(handleMediaKey("ArrowLeft")).toBe(false);
  });

  it("are caught on the window before any control and not passed on", () => {
    playerStore.playQueue([track("a")]);
    playerStore.updateTransport({ playing: true });
    const { unmount } = renderHook(() => useGlobalMediaKeys());
    const button = document.createElement("button");
    document.body.append(button);
    const seen = vi.fn();
    button.addEventListener("keydown", seen);
    button.focus();
    const event = new KeyboardEvent("keydown", { key: "MediaPlayPause", bubbles: true, cancelable: true });
    act(() => { button.dispatchEvent(event); });
    expect(event.defaultPrevented).toBe(true);
    expect(seen).not.toHaveBeenCalled();
    expect(playerStore.getSnapshot().transport.playing).toBe(false);
    unmount();
    button.remove();
  });
});
