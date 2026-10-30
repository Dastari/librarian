import { act, fireEvent, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { PlayerDock } from "../PlayerDock";
import { playerStore, type PlayItem } from "../store";

const engine = vi.hoisted(() => ({
  source: null as { url: string; isHls: boolean; duration: number; info: null } | null,
  error: null as string | null,
  loading: false,
  transitioning: false,
  retry: vi.fn(),
  cast: vi.fn(),
  sourceOptions: vi.fn(),
  progressOptions: vi.fn(),
  report: vi.fn(),
  flush: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("../CastButton", () => ({ CastButton: (props: { item?: PlayItem; position?: number; onStarted?: () => void }) => { engine.cast(props); return null; } }));
vi.mock("../useMediaSource", () => {
  const isTransitioning = () => engine.transitioning;
  return { useMediaSource: (...args: unknown[]) => { engine.sourceOptions(...args); return { source: engine.source, loading: engine.loading, error: engine.error, duration: 180, hls: null, retry: engine.retry, isTransitioning }; } };
});
vi.mock("../useProgressSync", () => ({ useProgressSync: (...args: unknown[]) => { engine.progressOptions(...args); return { loading: false, resumePosition: 0, report: engine.report, flush: engine.flush }; } }));

beforeEach(() => {
  engine.source = null;
  engine.error = null;
  engine.loading = false;
  engine.transitioning = false;
  vi.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue(undefined);
});

afterEach(() => { act(() => playerStore.stopAudio()); vi.restoreAllMocks(); });

describe("PlayerDock Glass actions", () => {
  it("opens now playing from the row and Queue, restores opener focus, and stops playback", async () => {
    playerStore.playQueue([{ mediaFileId: "file-a", title: "A track", entity: { kind: "track", id: "track-a" } }]);
    await renderWithProviders(<PlayerDock />);
    const dock = screen.getByRole("region", { name: "Now playing" });
    const opener = within(dock).getByRole("button", { name: "Open now playing" });
    opener.focus();
    await userEvent.keyboard("{Enter}");
    expect(await screen.findByRole("dialog", { name: "Now playing" })).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await vi.waitFor(() => expect(opener).toHaveFocus());
    const queue = within(dock).getByRole("button", { name: "Queue" });
    await userEvent.click(queue);
    expect(await screen.findByRole("dialog", { name: "Now playing" })).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await vi.waitFor(() => expect(queue).toHaveFocus());
    await userEvent.click(within(dock).getByRole("button", { name: "Stop" }));
    expect(playerStore.getSnapshot().queue).toEqual([]);
    expect(screen.queryByRole("region", { name: "Now playing" })).not.toBeInTheDocument();
  });
});


describe("audio reliability", () => {
  const queue = ["a", "b"].map((id) => ({ mediaFileId: id, title: `Track ${id}`, entity: { kind: "track" as const, id } }));
  const attach = () => { engine.source = { url: "/audio", isHls: false, duration: 180, info: null }; };

  it("passes the active item and position to casting and pauses only after success", async () => {
    playerStore.playQueue(queue);
    playerStore.updateTransport({ position: 32 });
    await renderWithProviders(<PlayerDock />);
    expect(engine.cast.mock.lastCall?.[0]).toMatchObject({ item: queue[0], position: 32 });
    expect(playerStore.getSnapshot().transport.playing).toBe(true);
    act(() => engine.cast.mock.lastCall![0].onStarted());
    expect(playerStore.getSnapshot().transport.playing).toBe(false);
  });

  it("rejects an old cast completion immediately on skip, including a repeated identical file entry", async () => {
    const repeated = queue[0]!;
    playerStore.playQueue([repeated, queue[1]!, repeated]);
    await renderWithProviders(<PlayerDock />);
    const origin = engine.cast.mock.lastCall![0];
    // Invoke before React rerenders: the callback must consult the live store.
    act(() => { playerStore.skip(1); origin.onStarted(); });
    expect(playerStore.getSnapshot().transport.playing).toBe(true);
    act(() => { playerStore.skip(1); origin.onStarted(); });
    expect(playerStore.getSnapshot().transport.playing).toBe(true);
    expect(engine.cast.mock.lastCall![0].activationKey).not.toBe(origin.activationKey);
    act(() => engine.cast.mock.lastCall![0].onStarted());
    expect(playerStore.getSnapshot().transport.playing).toBe(false);
  });

  it("preserves a current selection and passes new same-file activations to source and progress hooks", async () => {
    const repeated = queue[0]!;
    playerStore.playQueue([repeated, repeated]);
    await renderWithProviders(<PlayerDock />);
    act(() => playerStore.updateTransport({ playing: false, position: 41, duration: 180, buffered: 80, loading: false }));
    const current = playerStore.getSnapshot();
    act(() => playerStore.jumpTo(0));
    expect(playerStore.getSnapshot()).toBe(current);
    expect(engine.sourceOptions.mock.lastCall![3]).toEqual({ activationKey: current.audioActivation });
    act(() => playerStore.jumpTo(1));
    const next = playerStore.getSnapshot();
    expect(next.transport).toMatchObject({ playing: true, position: 0, loading: true });
    expect(next.audioActivation).not.toBe(current.audioActivation);
    expect(engine.sourceOptions.mock.lastCall![3]).toEqual({ activationKey: next.audioActivation });
    expect(engine.progressOptions.mock.lastCall).toEqual([repeated, next.audioActivation]);
  });

  it("ignores pause during a source transition and records a real pause", async () => {
    attach();
    playerStore.playQueue(queue);
    const view = await renderWithProviders(<PlayerDock />);
    const audio = view.container.querySelector("audio")!;
    engine.transitioning = true;
    fireEvent.pause(audio);
    expect(playerStore.getSnapshot().transport.playing).toBe(true);
    expect(engine.flush).not.toHaveBeenCalled();
    engine.transitioning = false;
    fireEvent.pause(audio);
    expect(playerStore.getSnapshot().transport.playing).toBe(false);
    expect(engine.report).toHaveBeenLastCalledWith(0, 180, false);
    expect(engine.flush).toHaveBeenCalledWith(true);
  });

  it("shows a source error and retries with playing intent restored", async () => {
    engine.error = "The server could not prepare this file.";
    playerStore.playQueue(queue);
    await renderWithProviders(<PlayerDock />);
    expect(screen.getByRole("alert")).toHaveTextContent(engine.error);
    expect(playerStore.getSnapshot().transport.playing).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(engine.retry).toHaveBeenCalledTimes(1);
    expect(playerStore.getSnapshot().transport).toMatchObject({ playing: true, loading: true, error: null });
  });

  it("shows play() rejection as a transport error with a working Retry action", async () => {
    attach();
    vi.mocked(HTMLMediaElement.prototype.play).mockRejectedValueOnce(new DOMException("A gesture is required", "NotAllowedError"));
    playerStore.playQueue(queue);
    await renderWithProviders(<PlayerDock />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Playback could not start");
    await userEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(engine.retry).toHaveBeenCalledTimes(1);
    expect(playerStore.getSnapshot().transport.error).toBeNull();
    expect(playerStore.getSnapshot().transport.playing).toBe(true);
  });

  it("does not apply an obsolete play() rejection to the next item", async () => {
    attach();
    let rejectOld!: (reason: Error) => void;
    vi.mocked(HTMLMediaElement.prototype.play).mockReturnValueOnce(new Promise<void>((_resolve, reject) => { rejectOld = reject; }));
    playerStore.playQueue(queue);
    const view = await renderWithProviders(<PlayerDock />);
    act(() => playerStore.skip(1));
    engine.transitioning = true;
    fireEvent.pause(view.container.querySelector("audio")!);
    await act(async () => rejectOld(new Error("Old source stopped")));
    expect(playerStore.getSnapshot().transport).toMatchObject({ playing: true, error: null });
  });
});
