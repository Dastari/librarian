import { act, fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { CastAdapter } from "glass-ux/media-player";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { MediaFilePreviewsDocument } from "@/graphql/generated/graphql";
import { readPref } from "@/lib/prefs";
import { renderWithProviders } from "@/test";

import { subtitleTracks, VideoPlayer } from "../VideoPlayer";

const mocks = vi.hoisted(() => ({
  retry: vi.fn(),
  source: vi.fn(),
  connect: vi.fn(),
  audio: [] as Array<{ streamIndex: number; title: string; language: string; isDefault: boolean }>,
  subtitles: [] as Array<{ id: string; title: string; language: string; codec: string }>,
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
  useMediaSource: (fileId: string | null, _element: unknown, _start: unknown, options: { audioStreamIndex?: number }) => {
    mocks.source(fileId, options.audioStreamIndex);
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
    expect(mocks.source).toHaveBeenLastCalledWith("file-1", 2);
    expect(onBack).not.toHaveBeenCalled();
  });

  it("offers fill mode only for a file with black bars, and remembers it on this device", async () => {
    window.localStorage.clear();
    const previews = {
      request: { query: MediaFilePreviewsDocument, variables: { id: "file-1" } },
      result: { data: { mediaFile: { __typename: "MediaFile", id: "file-1", previewsCheckedAt: "2026-11-09T00:00:00Z", pictureCrop: { __typename: "PictureCrop", left: 0, top: 138, width: 1920, height: 804, frameWidth: 1920, frameHeight: 1080 }, storyboard: null } } },
    };
    await renderWithProviders(<VideoPlayer item={item} onBack={vi.fn()} />, { mocks: [previews] });
    await userEvent.click(await screen.findByRole("button", { name: "Picture size" }));
    await userEvent.click(await screen.findByRole("menuitemradio", { name: /Fill the screen/ }));
    expect(readPref("player.pictureMode", "fit")).toBe("fill");
  });

  it("has no picture size menu for a file without black bars", async () => {
    await renderWithProviders(<VideoPlayer item={item} onBack={vi.fn()} />);
    expect(screen.queryByRole("button", { name: "Picture size" })).not.toBeInTheDocument();
  });

  it("has no audio menu for a file with one track", async () => {
    mocks.audio = [{ streamIndex: 1, title: "English", language: "en", isDefault: true }];
    await renderWithProviders(<VideoPlayer item={item} onBack={vi.fn()} />);
    expect(screen.queryByRole("button", { name: "Audio track" })).not.toBeInTheDocument();
  });

  it("gives the player only the subtitles the server can deliver as WebVTT", () => {
    expect(subtitleTracks("file-1", [
      { id: "sub-1", title: "English", language: "en", codec: "subrip" },
      { id: "sub-2", title: "Signs", language: "en", codec: "hdmv_pgs_subtitle" },
    ])).toEqual([{ id: "sub-1", src: "/api/media/file-1/subtitles/sub-1.vtt", label: "English", language: "en" }]);
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
