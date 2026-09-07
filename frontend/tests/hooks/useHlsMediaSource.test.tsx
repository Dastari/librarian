// @vitest-environment jsdom
import { createRef } from "react";
import { renderHook, cleanup } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useHlsMediaSource } from "../../src/hooks/useHlsMediaSource";
const state = vi.hoisted(() => ({ instances: [] as any[] }));
vi.mock("hls.js", () => ({
  default: class {
    static isSupported = () => true;
    static Events = { ERROR: "error" };
    destroy = vi.fn();
    loadSource = vi.fn();
    attachMedia = vi.fn();
    callback?: (...args: any[]) => void;
    constructor() {
      state.instances.push(this);
    }
    on(_event: string, callback: (...args: any[]) => void) {
      this.callback = callback;
    }
  },
}));
afterEach(() => {
  cleanup();
  state.instances.length = 0;
});
it("keeps HLS attached across renders while reporting errors to the latest callback", () => {
  const ref = createRef<HTMLVideoElement>();
  ref.current = document.createElement("video");
  const first = vi.fn();
  const latest = vi.fn();
  const { rerender, unmount } = renderHook(
    ({ onError }) => useHlsMediaSource(ref, "/movie/playlist.m3u8", onError),
    { initialProps: { onError: first } },
  );
  rerender({ onError: latest });
  expect(state.instances).toHaveLength(1);
  const hls = state.instances[0];
  expect(hls.destroy).not.toHaveBeenCalled();
  hls.callback("error", { fatal: true, type: "networkError" });
  expect(latest).toHaveBeenCalledOnce();
  expect(first).not.toHaveBeenCalled();
  unmount();
  expect(hls.destroy).toHaveBeenCalledOnce();
});
