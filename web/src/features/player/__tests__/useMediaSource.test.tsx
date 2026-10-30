import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resolvePlaybackSource, type PlaybackSource } from "@/lib/api/urls";

import { useMediaSource } from "../useMediaSource";

const hls = vi.hoisted(() => ({ instances: [] as Array<{
  emit: (event: string, data?: { fatal: boolean; type: string }) => void;
  startLoad: ReturnType<typeof vi.fn>;
  recoverMediaError: ReturnType<typeof vi.fn>;
  destroy: ReturnType<typeof vi.fn>;
  loadSource: ReturnType<typeof vi.fn>;
}> }));
vi.mock("@/lib/api/urls", () => ({ resolvePlaybackSource: vi.fn() }));
vi.mock("hls.js", () => ({
  default: class {
    static isSupported = () => true;
    static Events = { ERROR: "error", MEDIA_ATTACHED: "attached" };
    static ErrorTypes = { NETWORK_ERROR: "network", MEDIA_ERROR: "media" };
    handlers = new Map<string, (event: string, data?: { fatal: boolean; type: string }) => void>();
    startLoad = vi.fn();
    recoverMediaError = vi.fn();
    destroy = vi.fn();
    loadSource = vi.fn();
    constructor() { hls.instances.push(this); }
    on(event: string, handler: (event: string, data?: { fatal: boolean; type: string }) => void) { this.handlers.set(event, handler); }
    attachMedia() { this.emit("attached"); }
    emit(event: string, data?: { fatal: boolean; type: string }) { this.handlers.get(event)?.(event, data); }
  },
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((settle) => { resolve = settle; });
  return { promise, resolve };
}
const source = (url: string, isHls = false): PlaybackSource => ({ url, isHls, duration: 600, info: null });
let audio: HTMLAudioElement;
let element: { current: HTMLAudioElement };
beforeEach(() => {
  hls.instances.length = 0;
  vi.mocked(resolvePlaybackSource).mockReset();
  audio = document.createElement("audio");
  element = { current: audio };
  vi.spyOn(audio, "load").mockImplementation(() => {});
  vi.spyOn(audio, "canPlayType").mockReturnValue("");
});
afterEach(() => vi.restoreAllMocks());

describe("useMediaSource generations", () => {
  it("ignores an old resolver and removes its metadata listener during rapid file changes", async () => {
    const old = deferred<PlaybackSource>();
    vi.mocked(resolvePlaybackSource).mockReturnValueOnce(old.promise).mockResolvedValueOnce(source("/new"));
    const { result, rerender } = renderHook(({ id, start }) => useMediaSource(id, element, start), { initialProps: { id: "old", start: 90 } });
    const oldSignal = vi.mocked(resolvePlaybackSource).mock.calls[0]![1]!;
    rerender({ id: "new", start: 12 });
    expect(oldSignal.aborted).toBe(true);
    await waitFor(() => expect(result.current.source?.url).toBe("/new"));
    await act(async () => old.resolve(source("/old")));
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    expect(audio.getAttribute("src")).toBe("/new");
    expect(audio.currentTime).toBe(12);
    expect(result.current.loading).toBe(false);
  });

  it("preserves position for audio selection/retry keys and resets position for a different file", async () => {
    vi.mocked(resolvePlaybackSource).mockImplementation(async (id) => source(`/${id}`));
    const { result, rerender } = renderHook(({ id, stream, retryKey }) => useMediaSource(id, element, 30, { audioStreamIndex: stream, retryKey }), {
      initialProps: { id: "a", stream: 0, retryKey: 0 },
    });
    await waitFor(() => expect(result.current.source?.url).toBe("/a"));
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    audio.currentTime = 140;
    rerender({ id: "a", stream: 2, retryKey: 0 });
    await waitFor(() => expect(resolvePlaybackSource).toHaveBeenLastCalledWith("a", expect.any(AbortSignal), { audioStreamIndex: 2 }));
    await waitFor(() => expect(result.current.source?.url).toBe("/a"));
    audio.currentTime = 0;
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    expect(audio.currentTime).toBe(140);
    audio.currentTime = 165;
    rerender({ id: "a", stream: 2, retryKey: 1 });
    await waitFor(() => expect(resolvePlaybackSource).toHaveBeenCalledTimes(3));
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    expect(audio.currentTime).toBe(165);
    rerender({ id: "b", stream: 2, retryKey: 1 });
    await waitFor(() => expect(result.current.source?.url).toBe("/b"));
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    expect(audio.currentTime).toBe(30);
  });

  it("marks load cleanup as a transition before synchronous pause events fire", async () => {
    vi.mocked(resolvePlaybackSource).mockResolvedValue(source("/track"));
    const { result, rerender } = renderHook(({ id }) => useMediaSource(id, element), { initialProps: { id: "a" } });
    await waitFor(() => expect(result.current.source).not.toBeNull());
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    expect(result.current.isTransitioning()).toBe(false);
    const pauses: boolean[] = [];
    audio.addEventListener("pause", () => pauses.push(result.current.isTransitioning()));
    vi.mocked(audio.load).mockImplementation(() => { audio.dispatchEvent(new Event("pause")); });
    rerender({ id: "b" });
    expect(pauses).toEqual([true]);
  });

  it("exposes request and native errors, and retries without stale metadata listeners", async () => {
    vi.mocked(resolvePlaybackSource).mockRejectedValueOnce(new Error("Unavailable")).mockResolvedValue(source("/audio"));
    const { result, unmount } = renderHook(() => useMediaSource("file", element, 40));
    await waitFor(() => expect(result.current.error).toBe("Unavailable"));
    expect(result.current.loading).toBe(false);
    act(() => result.current.retry());
    await waitFor(() => expect(result.current.source?.url).toBe("/audio"));
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    expect(audio.currentTime).toBe(40);
    act(() => audio.dispatchEvent(new Event("error")));
    expect(result.current.error).toMatch(/could not be played/);
    expect(result.current.loading).toBe(false);
    unmount();
    audio.currentTime = 3;
    audio.dispatchEvent(new Event("loadedmetadata"));
    expect(audio.currentTime).toBe(3);
  });

  it("uses a finite shared budget for fatal HLS network/media recovery and starts a fresh budget on Retry", async () => {
    vi.mocked(resolvePlaybackSource).mockResolvedValue(source("/hls", true));
    const { result } = renderHook(() => useMediaSource("file", element));
    await waitFor(() => expect(hls.instances).toHaveLength(1));
    const first = hls.instances[0]!;
    act(() => {
      first.emit("error", { fatal: true, type: "network" });
      first.emit("error", { fatal: true, type: "media" });
      first.emit("error", { fatal: true, type: "network" });
      first.emit("error", { fatal: true, type: "media" });
    });
    expect(first.startLoad).toHaveBeenCalledTimes(1);
    expect(first.recoverMediaError).toHaveBeenCalledTimes(1);
    expect(first.destroy).toHaveBeenCalledTimes(1);
    expect(result.current.error).toMatch(/Playback failed/);
    expect(result.current.loading).toBe(false);
    act(() => result.current.retry());
    await waitFor(() => expect(hls.instances).toHaveLength(2));
    expect(result.current.error).toBeNull();
    act(() => hls.instances[1]!.emit("error", { fatal: true, type: "network" }));
    expect(hls.instances[1]!.startLoad).toHaveBeenCalledTimes(1);
    act(() => first.emit("error", { fatal: true, type: "network" }));
    expect(first.startLoad).toHaveBeenCalledTimes(1);
  });

  it("freezes the retry position and suppresses zero-position events from terminal HLS destruction", async () => {
    vi.mocked(resolvePlaybackSource).mockResolvedValue(source("/hls", true));
    const { result } = renderHook(() => useMediaSource("file", element, 20));
    await waitFor(() => expect(hls.instances).toHaveLength(1));
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    audio.currentTime = 137;
    const teardownEvents: boolean[] = [];
    for (const event of ["pause", "timeupdate", "durationchange", "ended"]) {
      audio.addEventListener(event, () => teardownEvents.push(result.current.isTransitioning()));
    }
    vi.mocked(audio.load).mockImplementation(() => {
      audio.currentTime = 0;
      for (const event of ["pause", "timeupdate", "durationchange", "ended"]) audio.dispatchEvent(new Event(event));
    });
    const first = hls.instances[0]!;
    first.destroy.mockImplementation(() => audio.load());
    act(() => first.emit("error", { fatal: true, type: "unrecoverable" }));
    expect(result.current.error).toMatch(/Playback failed/);
    expect(teardownEvents).toEqual([true, true, true, true]);
    expect(audio.currentTime).toBe(0);
    // Delayed terminal events must not replace the frozen position either.
    act(() => {
      audio.dispatchEvent(new Event("timeupdate"));
      audio.dispatchEvent(new Event("canplay"));
      audio.dispatchEvent(new Event("playing"));
    });
    expect(result.current.isTransitioning()).toBe(true);
    act(() => result.current.retry());
    await waitFor(() => expect(hls.instances).toHaveLength(2));
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    expect(audio.currentTime).toBe(137);
    expect(result.current.isTransitioning()).toBe(false);
    expect(first.destroy).toHaveBeenCalledTimes(1);
  });

  it("restarts a repeated file for a new activation while preserving position for retries within it", async () => {
    vi.mocked(resolvePlaybackSource).mockResolvedValue(source("/same-file"));
    const { result, rerender } = renderHook(({ activationKey, start }) => useMediaSource("same-file", element, start, { activationKey }), {
      initialProps: { activationKey: 1, start: 12 },
    });
    await waitFor(() => expect(result.current.source).not.toBeNull());
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    audio.currentTime = 140;
    rerender({ activationKey: 2, start: 0 });
    await waitFor(() => expect(resolvePlaybackSource).toHaveBeenCalledTimes(2));
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    expect(audio.currentTime).toBe(0);
    audio.currentTime = 25;
    act(() => result.current.retry());
    await waitFor(() => expect(resolvePlaybackSource).toHaveBeenCalledTimes(3));
    act(() => audio.dispatchEvent(new Event("loadedmetadata")));
    expect(audio.currentTime).toBe(25);
  });
});
