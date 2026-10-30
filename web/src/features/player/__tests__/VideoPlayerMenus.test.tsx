import { act, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { VideoPlayer } from "../VideoPlayer";

const media = vi.hoisted(() => {
  const listeners = new Map<string, Set<() => void>>();
  return {
    listeners,
    hls: {
      subtitleTracks: [] as Array<{ name: string; lang: string }>,
      audioTracks: [] as Array<{ name: string; lang: string }>,
      audioTrack: 0,
      subtitleTrack: -1,
      subtitleDisplay: false,
      on(event: string, callback: () => void) {
        const handlers = listeners.get(event) ?? new Set();
        handlers.add(callback);
        listeners.set(event, handlers);
      },
      off(event: string, callback: () => void) { listeners.get(event)?.delete(callback); },
    },
  };
});
vi.mock("@/lib/graphql/live/hooks", () => ({ useLiveQuery: () => ({ data: { mediaChapters: { edges: [] } } }) }));
vi.mock("@/features/settings/useSettingsConnection", () => ({ useSettingsConnection: () => ({ rows: [], error: null }) }));
vi.mock("../CastButton", () => ({ CastButton: () => null }));
vi.mock("../useMediaSource", () => ({ useMediaSource: () => ({ source: null, loading: false, error: null, duration: 300, hls: media.hls, retry: vi.fn(), isTransitioning: () => false }) }));
vi.mock("../useProgressSync", () => ({ useProgressSync: () => ({ loading: false, resumePosition: 0, report: vi.fn(), flush: vi.fn().mockResolvedValue(undefined) }) }));

describe("VideoPlayer menus", () => {
  beforeEach(() => {
    media.listeners.clear();
    media.hls.audioTracks = [{ name: "English", lang: "en" }, { name: "French", lang: "fr" }];
    media.hls.subtitleTracks = [];
    media.hls.audioTrack = 0;
    media.hls.subtitleTrack = -1;
    media.hls.subtitleDisplay = false;
  });

  it("changes playback speed and audio, closes with Escape, and returns focus without leaving playback", async () => {
    const onBack = vi.fn();
    const { container } = await renderWithProviders(<VideoPlayer item={{ mediaFileId: "file-1", title: "Film", entity: { kind: "movie", id: "movie-1" } }} onBack={onBack} />);
    const video = container.querySelector("video")!;
    const settings = screen.getByRole("button", { name: "Playback settings" });
    await userEvent.click(settings);
    const faster = await screen.findByRole("menuitemradio", { name: "1.5×" });
    faster.focus();
    await userEvent.keyboard("{Enter}");
    expect(video.playbackRate).toBe(1.5);
    await userEvent.click(settings);
    await screen.findByRole("menu");
    await userEvent.keyboard("{Escape}");
    await vi.waitFor(() => expect(settings).toHaveFocus());
    expect(onBack).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole("button", { name: "Audio track" }));
    const french = await screen.findByRole("menuitemradio", { name: "French" });
    french.focus();
    await userEvent.keyboard("{Enter}");
    expect(media.hls.audioTrack).toBe(1);
    expect(onBack).not.toHaveBeenCalled();
  });

  it("offers tracks discovered after an initially empty manifest and unregisters listeners", async () => {
    media.hls.audioTracks = [];
    const { unmount } = await renderWithProviders(<VideoPlayer item={{ mediaFileId: "late-tracks", title: "Film", entity: { kind: "movie", id: "movie-1" } }} onBack={vi.fn()} />);
    expect(screen.queryByRole("button", { name: "Audio track" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Subtitles" })).not.toBeInTheDocument();
    act(() => {
      media.hls.audioTracks = [{ name: "English", lang: "en" }, { name: "French", lang: "fr" }];
      media.hls.subtitleTracks = [{ name: "English captions", lang: "en" }];
      for (const event of ["hlsAudioTracksUpdated", "hlsSubtitleTracksUpdated"]) {
        media.listeners.get(event)?.forEach((handler) => handler());
      }
    });
    expect(screen.getByRole("button", { name: "Audio track" })).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: "Subtitles" }));
    await userEvent.click(await screen.findByRole("menuitemradio", { name: "English captions" }));
    expect(media.hls.subtitleTrack).toBe(0);
    expect(media.hls.subtitleDisplay).toBe(true);
    await userEvent.click(screen.getByRole("button", { name: "Subtitles" }));
    await userEvent.click(await screen.findByRole("menuitemradio", { name: "Off" }));
    expect(media.hls.subtitleTrack).toBe(-1);
    expect(media.hls.subtitleDisplay).toBe(false);
    unmount();
    expect([...media.listeners.values()].every((handlers) => handlers.size === 0)).toBe(true);
  });
});
