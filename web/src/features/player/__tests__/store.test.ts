import { afterEach, describe, expect, it, vi } from "vitest";

import { currentAudioItem, playerStore, type PlayItem } from "../store";

const authority = vi.hoisted(() => ({ changed: undefined as (() => void) | undefined }));
vi.mock("@/lib/auth/session", () => ({ session: { onChange: (listener: () => void) => { authority.changed = listener; return () => {}; } } }));

const queue: PlayItem[] = ["one", "two", "three"].map((id) => ({ mediaFileId: id, title: id, entity: { kind: "track", id } }));
afterEach(() => playerStore.stopAudio());
const failedTransport = () => playerStore.updateTransport({ position: 80, duration: 180, buffered: 130, error: "Failed", playing: false });
const expectFreshTransport = () => expect(playerStore.getSnapshot().transport).toMatchObject({ position: 0, duration: 0, buffered: 0, error: null, loading: true, playing: true });

describe("audio queue transitions", () => {
  it("preserves the full transport and activation when selecting the current entry", () => {
    playerStore.playQueue(queue, 1);
    failedTransport();
    const before = playerStore.getSnapshot();
    playerStore.jumpTo(1);
    expect(playerStore.getSnapshot()).toBe(before);
  });

  it("starts distinct activations for repeated entries, including the identical item object", () => {
    const repeated = queue[0]!;
    playerStore.playQueue([repeated, repeated, { ...repeated }]);
    const first = playerStore.getSnapshot().audioActivation;
    failedTransport();
    playerStore.skip(1);
    expectFreshTransport();
    expect(playerStore.getSnapshot().audioActivation).toBeGreaterThan(first);
    const second = playerStore.getSnapshot().audioActivation;
    failedTransport();
    playerStore.removeAt(1);
    expectFreshTransport();
    expect(playerStore.getSnapshot().audioActivation).toBeGreaterThan(second);
    const third = playerStore.getSnapshot().audioActivation;
    playerStore.removeAt(0);
    expect(playerStore.getSnapshot().audioActivation).toBe(third);
    playerStore.playQueue([repeated]);
    expect(playerStore.getSnapshot().audioActivation).toBeGreaterThan(third);
  });
  it("resets the previous file's position/duration/error for skip and jump", () => {
    playerStore.playQueue(queue);
    failedTransport();
    playerStore.skip(1);
    expect(currentAudioItem(playerStore.getSnapshot())?.mediaFileId).toBe("two");
    expectFreshTransport();
    failedTransport();
    playerStore.jumpTo(2);
    expectFreshTransport();
  });
  it("resets the transport when removing the current item and stops when the queue becomes empty", () => {
    playerStore.playQueue(queue);
    failedTransport();
    playerStore.removeAt(0);
    expect(currentAudioItem(playerStore.getSnapshot())?.mediaFileId).toBe("two");
    expectFreshTransport();
    playerStore.removeAt(0);
    playerStore.removeAt(0);
    expect(currentAudioItem(playerStore.getSnapshot())).toBeNull();
    expect(playerStore.getSnapshot().transport).toMatchObject({ playing: false, loading: false, position: 0, error: null });
  });
  it("preserves the active transport when removing another item before it", () => {
    playerStore.playQueue(queue, 1);
    playerStore.updateTransport({ position: 50, duration: 180, buffered: 90, loading: false });
    playerStore.removeAt(0);
    expect(currentAudioItem(playerStore.getSnapshot())?.mediaFileId).toBe("two");
    expect(playerStore.getSnapshot().transport).toMatchObject({ playing: true, position: 50, duration: 180, buffered: 90, loading: false });
  });
  it("clears private audio/video identity on a principal or authority replacement", () => {
    playerStore.playQueue(queue);
    playerStore.setVideo({ ...queue[0]!, entity: { kind: "movie", id: "private-movie" } });
    playerStore.setSheetOpen(true);
    authority.changed!();
    expect(playerStore.getSnapshot()).toMatchObject({ video: null, queue: [], index: -1, sheetOpen: false });
    expect(playerStore.getSnapshot().transport).toMatchObject({ playing: false, loading: false, position: 0, error: null });
  });
  it("clears stale transport when asked to play an empty queue", () => {
    playerStore.playQueue(queue);
    failedTransport();
    playerStore.playQueue([]);
    expect(currentAudioItem(playerStore.getSnapshot())).toBeNull();
    expect(playerStore.getSnapshot().transport).toMatchObject({ playing: false, loading: false, error: null });
  });
});
