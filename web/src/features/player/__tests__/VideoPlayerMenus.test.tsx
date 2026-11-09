import { act, fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { CastAdapter } from "glass-ux/media-player";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { subtitleTracks, VideoPlayer } from "../VideoPlayer";

const mocks = vi.hoisted(() => ({
  retry: vi.fn(),
  source: vi.fn(),
  connect: vi.fn(),
  audio: [] as Array<{ streamIndex: number; title: string; language: string; isDefault: boolean }>,
  subtitles: [] as Array<{ id: string; title: string; language: string; codec: string; sourceType: string }>,
}));

vi.mock("@/features/settings/useSettingsConnection", () => ({
  useSettingsConnection: (_document: unknown, _variables: unknown, select: (data: Record<string, unknown>) => unknown) => {
    const probe = { audioStreams: "audio", subtitles: "subtitles", mediaChapters: "chapters" };
    const kind = select(probe);
    return { rows: kind === "audio" ? mocks.audio : kind === "subtitles" ? mocks.subtitles : [], error: null };
  },
}));
vi.mock("@/lib/input-mode", () => ({ useInputMode: () => ({ mode: "pointer" }) }));
vi.mock("../CastButton", () => ({ CastButton: () => null }));
vi.mock("../useMediaSource", () => ({
  useMediaSource: (fileId: string | null, _element: unknown, _start: unknown, options: { audioStreamIndex?: number; burnSubtitleId?: string }) => {
    mocks.source(fileId, options.audioStreamIndex, options.burnSubtitleId);
    return { source: null, loading: false, error: null, duration: 300, hls: null, retry: mocks.retry, isTransitioning: () => false };
  },
}));
vi.mock("../useProgressSync", () => ({ useProgressSync: () => ({ loading: false, resumePosition: 0, report: vi.fn(), flush: vi.fn() }) }));
vi.mock("../useCastAdapter", () => ({
  useCastAdapter: () => {
    const adapter: CastAdapter = {
      getSession: () => ({ status: "available" }), subscribe: () => () => undefined, connect: mocks.connect, disconnect: vi.fn(),
      load: vi.fn(), play: vi.fn(), pause: vi.fn(), seek: vi.fn(), stop: vi.fn(), setVolume: vi.fn(),
    };
    return { adapter, pickerOpen: false, setPickerOpen: vi.fn(), refresh: vi.fn() };
  },
}));

const item = { mediaFileId: "file-1", title: "Film", entity: { kind: "movie" as const, id: "movie-1" } };

describe("VideoPlayer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.audio = [];
    mocks.subtitles = [];
  });

  it("offers the file's audio tracks and re-requests the source for the one chosen", async () => {
    mocks.audio = [{ streamIndex: 1, title: "English", language: "en", isDefault: true }, { streamIndex: 2, title: "French", language: "fr", isDefault: false }];
    const onBack = vi.fn();
    await renderWithProviders(<VideoPlayer item={item} onBack={onBack} />);
    await userEvent.click(screen.getByRole("button", { name: "Audio track" }));
    await userEvent.click(await screen.findByRole("menuitemradio", { name: "French" }));
    expect(mocks.source).toHaveBeenLastCalledWith("file-1", 2, undefined);
    expect(onBack).not.toHaveBeenCalled();
  });

  it("has no audio menu for a file with one track", async () => {
    mocks.audio = [{ streamIndex: 1, title: "English", language: "en", isDefault: true }];
    await renderWithProviders(<VideoPlayer item={item} onBack={vi.fn()} />);
    expect(screen.queryByRole("button", { name: "Audio track" })).not.toBeInTheDocument();
  });

  it("gives the player only the subtitles the server can deliver as WebVTT", () => {
    expect(subtitleTracks("file-1", [
      { id: "sub-1", title: "English", language: "en", codec: "subrip", sourceType: "embedded" },
      { id: "sub-2", title: "Signs", language: "en", codec: "hdmv_pgs_subtitle", sourceType: "embedded" },
    ])).toEqual([{ id: "sub-1", src: "/api/media/file-1/subtitles/sub-1.vtt", label: "English", language: "en", kind: "subtitles" }]);
  });

  it("burns a picture subtitle in by asking for a new stream, and offers the search", async () => {
    // jsdom has no text tracks, so only a picture subtitle is rendered here.
    mocks.subtitles = [{ id: "sub-2", title: "", language: "eng", codec: "hdmv_pgs_subtitle", sourceType: "embedded" }];
    await renderWithProviders(<VideoPlayer item={item} onBack={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: "Subtitle options" }));
    await userEvent.click(await screen.findByRole("menuitemradio", { name: "English" }));
    expect(mocks.source).toHaveBeenLastCalledWith("file-1", undefined, "sub-2");
    await userEvent.click(screen.getByRole("button", { name: "Subtitle options" }));
    expect(await screen.findByRole("menuitem", { name: /Find subtitles/ })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: /Later by/ })).toBeInTheDocument();
  });

  it("leaves on Back and Escape, but not from inside a menu", async () => {
    mocks.audio = [{ streamIndex: 1, title: "English", language: "en", isDefault: true }, { streamIndex: 2, title: "French", language: "fr", isDefault: false }];
    const onBack = vi.fn();
    await renderWithProviders(<VideoPlayer item={item} onBack={onBack} />);
    await userEvent.click(screen.getByRole("button", { name: "Audio track" }));
    await screen.findByRole("menu");
    await userEvent.keyboard("{Escape}");
    expect(onBack).not.toHaveBeenCalled();
    await act(async () => { fireEvent.keyDown(window, { key: "GoBack" }); });
    expect(onBack).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(onBack).toHaveBeenCalledTimes(2);
  });

  it("opens the receiver picker from the player's cast button", async () => {
    await renderWithProviders(<VideoPlayer item={item} onBack={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: "Cast to a device" }));
    expect(mocks.connect).toHaveBeenCalledTimes(1);
  });
});
