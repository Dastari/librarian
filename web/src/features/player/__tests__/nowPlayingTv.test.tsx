import { act, fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { NowPlayingRailItem, useNowPlayingShortcut } from "../nowPlaying";
import { PlayerDock } from "../PlayerDock";
import { playerStore, type PlayItem } from "../store";

const view = vi.hoisted(() => ({ tv: true, desktop: true }));
vi.mock("@/lib/input-mode", async (importOriginal) => ({ ...await importOriginal<typeof import("@/lib/input-mode")>(), useIsTvMode: () => view.tv }));
vi.mock("@/hooks/useMediaQuery", async (importOriginal) => ({ ...await importOriginal<typeof import("@/hooks/useMediaQuery")>(), useIsDesktop: () => view.desktop }));
vi.mock("../CastButton", () => ({ CastButton: () => null }));
vi.mock("../useMediaSource", () => ({ useMediaSource: () => ({ source: null, loading: false, error: null, duration: 180, hls: null, retry: vi.fn(), isTransitioning: () => false }) }));
vi.mock("../useProgressSync", async (importOriginal) => ({ ...await importOriginal<typeof import("../useProgressSync")>(), useProgressSync: () => ({ loading: false, resumePosition: 0, report: vi.fn(), describe: vi.fn(), flush: vi.fn().mockResolvedValue(undefined) }) }));
vi.mock("glass-ux/shell", () => ({ Rail: { Item: ({ label, icon: _icon, active: _active, ...rest }: Record<string, unknown> & { label: string }) => <button type="button" aria-label={label} {...rest} /> } }));

const album: PlayItem[] = [
  { mediaFileId: "a", title: "Track one", entity: { kind: "track", id: "t1" } },
  { mediaFileId: "b", title: "Track two", entity: { kind: "track", id: "t2" } },
];

function AlbumPage() {
  useNowPlayingShortcut();
  return (
    <>
      <NowPlayingRailItem />
      <button type="button" onClick={() => playerStore.playQueue(album)}>Play album</button>
      <button type="button" onClick={() => playerStore.enqueue([album[1]!])}>Add to queue</button>
      <PlayerDock />
    </>
  );
}

/** The dock's Play/Pause (the phone bar has its own, hidden by CSS). */
const dockStart = () => document.querySelector<HTMLElement>("[data-dock-start]")!;

beforeEach(() => {
  view.tv = true;
  view.desktop = true;
  vi.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue(undefined);
});
afterEach(() => { act(() => playerStore.stopAudio()); vi.restoreAllMocks(); });

describe("now playing on a TV", () => {
  it("opens in full when audio starts, with Play/Pause focused, and Back returns to the Play button", async () => {
    await renderWithProviders(<AlbumPage />);
    const play = screen.getByRole("button", { name: "Play album" });
    play.focus();
    await userEvent.keyboard("{Enter}");
    expect(await screen.findByRole("dialog", { name: "Now playing" })).toBeInTheDocument();
    await vi.waitFor(() => expect(dockStart()).toHaveFocus());
    // The remote's Back arrives as Escape.
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "Now playing" })).not.toBeInTheDocument();
    expect(playerStore.getSnapshot().sheetOpen).toBe(false);
    await vi.waitFor(() => expect(play).toHaveFocus());
    // Still playing, in the bar.
    expect(screen.getByRole("region", { name: "Now playing" })).toBeInTheDocument();
  });

  it("leaves the screen alone for Add to queue while something plays", async () => {
    await renderWithProviders(<AlbumPage />);
    await userEvent.click(screen.getByRole("button", { name: "Play album" }));
    await vi.waitFor(() => expect(dockStart()).toHaveFocus());
    await userEvent.keyboard("{Escape}");
    expect(playerStore.getSnapshot().sheetOpen).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Add to queue" }));
    expect(playerStore.getSnapshot().sheetOpen).toBe(false);
  });

  it("does not take over the screen outside TV mode", async () => {
    view.tv = false;
    await renderWithProviders(<AlbumPage />);
    await userEvent.click(screen.getByRole("button", { name: "Play album" }));
    expect(playerStore.getSnapshot().sheetOpen).toBe(false);
    expect(screen.queryByRole("button", { name: /^Now playing:/ })).not.toBeInTheDocument();
  });

  it("shows a rail entry while something plays that opens now playing, and Up from the bar goes back", async () => {
    await renderWithProviders(<AlbumPage />);
    expect(screen.queryByRole("button", { name: /^Now playing:/ })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Play album" }));
    await userEvent.keyboard("{Escape}");
    const entry = screen.getByRole("button", { name: "Now playing: Track one" });
    entry.focus();
    await userEvent.keyboard("{Enter}");
    expect(await screen.findByRole("dialog", { name: "Now playing" })).toBeInTheDocument();
    const pause = dockStart();
    await vi.waitFor(() => expect(pause).toHaveFocus());
    await userEvent.keyboard("{Escape}");
    await vi.waitFor(() => expect(entry).toHaveFocus());
    // Reaching the bar and pressing Up returns to where focus came from.
    pause.focus();
    fireEvent.keyDown(pause, { key: "ArrowUp" });
    expect(entry).toHaveFocus();
  });

  it("opens now playing with the Info key or N from anywhere", async () => {
    await renderWithProviders(<AlbumPage />);
    await userEvent.click(screen.getByRole("button", { name: "Play album" }));
    await userEvent.keyboard("{Escape}");
    screen.getByRole("button", { name: "Add to queue" }).focus();
    fireEvent.keyDown(window, { key: "Info" });
    expect(playerStore.getSnapshot().sheetOpen).toBe(true);
    await userEvent.keyboard("{Escape}");
    fireEvent.keyDown(window, { key: "n" });
    expect(playerStore.getSnapshot().sheetOpen).toBe(true);
  });
});
