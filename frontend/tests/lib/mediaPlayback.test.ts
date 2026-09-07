import { afterEach, expect, it, vi } from "vitest";
import { resolveMediaPlaybackSource } from "../../src/lib/api/mediaPlayback";
afterEach(() => vi.unstubAllGlobals());
it("keeps the full file duration when selecting a growing HLS playlist", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue({
      ok: true,
      json: async () => ({
        needs_hls: true,
        playback_url: "/api/media/one/hls/playlist.m3u8",
        duration: 8097,
      }),
    });
  vi.stubGlobal("fetch", fetcher);
  expect(await resolveMediaPlaybackSource("one")).toEqual({
    url: "/api/media/one/hls/playlist.m3u8",
    duration: 8097,
  });
  expect(fetcher).toHaveBeenCalledWith("/api/media/one/info", {
    credentials: "include",
  });
});
it("falls back to direct playback when metadata is unavailable", async () => {
  vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));
  expect(await resolveMediaPlaybackSource("one")).toEqual({
    url: "/api/media/one/stream",
  });
});
