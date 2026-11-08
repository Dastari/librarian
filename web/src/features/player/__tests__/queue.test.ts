import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { queueKey } from "../queuePersistence";
import { canAdvance, currentAudioItem, currentQueueItem, playerStore, type PlayItem } from "../store";

vi.mock("@/lib/auth/session", () => ({ session: { onChange: () => () => {} } }));

const track = (id: string): PlayItem => ({ mediaFileId: `file-${id}`, title: id, entity: { kind: "track", id }, albumId: "album" });
const episode = (id: string): PlayItem => ({ mediaFileId: `file-${id}`, title: id, entity: { kind: "episode", id }, showId: "show" });
const movie = (id: string): PlayItem => ({ mediaFileId: `file-${id}`, title: id, entity: { kind: "movie", id } });
const titles = () => playerStore.getSnapshot().queue.map((item) => item.title);
const current = () => currentQueueItem(playerStore.getSnapshot())?.title;

beforeEach(() => {
  window.localStorage.clear();
  playerStore.forgetPerson();
  playerStore.setContinuationResolver(null);
  playerStore.setVideo(null);
  playerStore.setVideoMini(false);
  playerStore.stopAudio();
});
afterEach(() => {
  playerStore.forgetPerson();
  playerStore.setContinuationResolver(null);
  vi.restoreAllMocks();
});

describe("queue editing", () => {
  it("fills the queue by playing, and starts at the chosen entry", () => {
    playerStore.playQueue(["a", "b", "c"].map(track), 1);
    expect(titles()).toEqual(["a", "b", "c"]);
    expect(current()).toBe("b");
    expect(playerStore.getSnapshot().transport).toMatchObject({ playing: true, loading: true });
  });

  it("inserts Play next entries right after the playing one, in the order given, before earlier entries", () => {
    playerStore.playQueue(["a", "b", "c"].map(track));
    playerStore.playNext(["x", "y"].map(track));
    expect(titles()).toEqual(["a", "x", "y", "b", "c"]);
    playerStore.playNext([track("z")]);
    expect(titles()).toEqual(["a", "z", "x", "y", "b", "c"]);
    expect(current()).toBe("a");
  });

  it("appends Add to queue entries at the end without disturbing what plays", () => {
    playerStore.playQueue(["a", "b"].map(track), 1);
    const activation = playerStore.getSnapshot().audioActivation;
    playerStore.enqueue(["x", "y"].map(track));
    expect(titles()).toEqual(["a", "b", "x", "y"]);
    expect(current()).toBe("b");
    expect(playerStore.getSnapshot().audioActivation).toBe(activation);
  });

  it("starts playback when Play next or Add to queue is used with nothing playing", () => {
    playerStore.enqueue(["a", "b"].map(track));
    expect(current()).toBe("a");
    expect(playerStore.getSnapshot().transport.playing).toBe(true);
    playerStore.stopAudio();
    playerStore.playNext([track("c")]);
    expect(current()).toBe("c");
  });

  it("gives every entry its own identity, so the same media queued twice is two entries", () => {
    const one = track("a");
    playerStore.playQueue([one]);
    playerStore.enqueue([one, one]);
    const ids = playerStore.getSnapshot().queue.map((item) => item.queueId);
    expect(new Set(ids).size).toBe(3);
  });

  describe("moving", () => {
    const setup = (index: number) => playerStore.playQueue(["a", "b", "c", "d", "e"].map(track), index);

    it("moves an entry and lets the playing one keep playing wherever it ends up", () => {
      setup(2);
      const activation = playerStore.getSnapshot().audioActivation;
      playerStore.move(2, 0);
      expect(titles()).toEqual(["c", "a", "b", "d", "e"]);
      expect(current()).toBe("c");
      playerStore.move(0, 4);
      expect(titles()).toEqual(["a", "b", "d", "e", "c"]);
      expect(current()).toBe("c");
      expect(playerStore.getSnapshot().audioActivation).toBe(activation);
    });

    it("follows the playing entry when another is moved across it", () => {
      setup(2);
      playerStore.move(0, 4);
      expect(titles()).toEqual(["b", "c", "d", "e", "a"]);
      expect(current()).toBe("c");
      playerStore.move(4, 0);
      expect(titles()).toEqual(["a", "b", "c", "d", "e"]);
      expect(current()).toBe("c");
      playerStore.move(4, 3);
      expect(current()).toBe("c");
    });

    it("ignores moves that go nowhere", () => {
      setup(0);
      const before = playerStore.getSnapshot().queue;
      playerStore.move(1, 1);
      playerStore.move(-1, 2);
      playerStore.move(1, 9);
      expect(playerStore.getSnapshot().queue).toBe(before);
    });
  });

  describe("removing", () => {
    it("keeps playing when an entry before or after the playing one is removed", () => {
      playerStore.playQueue(["a", "b", "c", "d"].map(track), 2);
      const activation = playerStore.getSnapshot().audioActivation;
      playerStore.removeAt(0);
      expect(titles()).toEqual(["b", "c", "d"]);
      expect(current()).toBe("c");
      playerStore.removeAt(2);
      expect(titles()).toEqual(["b", "c"]);
      expect(current()).toBe("c");
      expect(playerStore.getSnapshot().audioActivation).toBe(activation);
    });

    it("plays the next entry when the playing one is removed, and stops when none is left", () => {
      playerStore.playQueue(["a", "b"].map(track), 0);
      playerStore.removeAt(0);
      expect(current()).toBe("b");
      expect(playerStore.getSnapshot().transport.playing).toBe(true);
      playerStore.removeAt(0);
      expect(current()).toBeUndefined();
      expect(playerStore.getSnapshot().transport.playing).toBe(false);
    });
  });

  it("clears only what is after the playing entry", () => {
    playerStore.playQueue(["a", "b", "c", "d"].map(track), 1);
    playerStore.clearUpcoming();
    expect(titles()).toEqual(["a", "b"]);
    expect(current()).toBe("b");
  });
});

describe("mixed audio and video", () => {
  it("hands a video entry to the video player and stops the audio", () => {
    playerStore.playQueue([track("song"), episode("ep")], 1);
    const state = playerStore.getSnapshot();
    expect(state.video?.entity.id).toBe("ep");
    expect(currentAudioItem(state)).toBeNull();
    expect(state.transport.playing).toBe(false);
  });

  it("switches between players as the queue advances, and back", async () => {
    playerStore.playQueue([track("song"), episode("ep"), track("again")]);
    expect(currentAudioItem(playerStore.getSnapshot())?.title).toBe("song");
    expect(playerStore.getSnapshot().video).toBeNull();

    expect(await playerStore.advance()).toBe(true);
    expect(playerStore.getSnapshot().video?.entity.id).toBe("ep");
    expect(currentAudioItem(playerStore.getSnapshot())).toBeNull();

    expect(await playerStore.advance()).toBe(true);
    const state = playerStore.getSnapshot();
    expect(state.video).toBeNull();
    expect(currentAudioItem(state)?.title).toBe("again");
    expect(state.transport).toMatchObject({ playing: true, loading: true, position: 0 });
  });

  it("moves from one video straight to the next, keeping the floating window", () => {
    playerStore.playQueue([episode("one"), episode("two")]);
    playerStore.setVideoMini(true);
    playerStore.skip(1);
    expect(playerStore.getSnapshot().video?.entity.id).toBe("two");
    expect(playerStore.getSnapshot().videoMini).toBe(true);
  });

  it("starts audio over a video: the video stops and its window closes", () => {
    playerStore.playQueue([episode("one")]);
    playerStore.setVideoMini(true);
    playerStore.playQueue([track("song")]);
    expect(playerStore.getSnapshot().video).toBeNull();
    expect(playerStore.getSnapshot().videoMini).toBe(false);
  });
});

describe("when the queue runs out", () => {
  it("adds what the continuation finds and plays it", async () => {
    const resolver = vi.fn(async () => [track("next-1"), track("next-2")]);
    playerStore.setContinuationResolver(resolver);
    playerStore.playQueue([track("a")]);
    expect(await playerStore.advance()).toBe(true);
    expect(resolver).toHaveBeenCalledWith(expect.objectContaining({ title: "a" }));
    expect(titles()).toEqual(["a", "next-1", "next-2"]);
    expect(current()).toBe("next-1");
    expect(playerStore.getSnapshot().continuing).toBe(false);
  });

  it("stops, and remembers there is nothing more, when nothing continues it", async () => {
    const resolver = vi.fn(async () => []);
    playerStore.setContinuationResolver(resolver);
    playerStore.playQueue([track("a")]);
    expect(canAdvance(playerStore.getSnapshot())).toBe(true);
    expect(await playerStore.advance()).toBe(false);
    expect(playerStore.getSnapshot().transport.playing).toBe(false);
    expect(canAdvance(playerStore.getSnapshot())).toBe(false);
    // Not asked again until the queue changes.
    await playerStore.advance();
    expect(resolver).toHaveBeenCalledTimes(1);
    playerStore.enqueue([track("b")]);
    expect(canAdvance(playerStore.getSnapshot())).toBe(true);
  });

  it("stops when the lookup fails", async () => {
    playerStore.setContinuationResolver(async () => { throw new Error("offline"); });
    playerStore.playQueue([track("a")]);
    expect(await playerStore.advance()).toBe(false);
    expect(titles()).toEqual(["a"]);
  });

  it("leaves the queue alone when it was changed while the lookup ran", async () => {
    let finish!: (items: PlayItem[]) => void;
    playerStore.setContinuationResolver(() => new Promise<PlayItem[]>((resolve) => { finish = resolve; }));
    playerStore.playQueue([track("a")]);
    const advancing = playerStore.advance();
    expect(playerStore.getSnapshot().continuing).toBe(true);
    playerStore.enqueue([track("chosen")]);
    finish([track("found")]);
    expect(await advancing).toBe(false);
    expect(titles()).toEqual(["a", "chosen"]);
    expect(playerStore.getSnapshot().continuing).toBe(false);
  });

  it("offers nothing after a movie", async () => {
    playerStore.setContinuationResolver(async () => [track("never")]);
    playerStore.playQueue([movie("film")]);
    expect(canAdvance(playerStore.getSnapshot())).toBe(false);
  });

  it("is what Next does at the last entry", async () => {
    playerStore.setContinuationResolver(async () => [track("more")]);
    playerStore.playQueue([track("a")]);
    playerStore.skip(1);
    await vi.waitFor(() => expect(current()).toBe("more"));
  });
});

describe("saving the queue on this device", () => {
  const saved = (userId = "user-1") => JSON.parse(window.localStorage.getItem(queueKey(userId)) ?? "null") as { items: PlayItem[]; index: number } | null;

  it("keeps the queue and the playing entry as they change", () => {
    expect(playerStore.restore("user-1")).toBe(false);
    playerStore.playQueue(["a", "b", "c"].map(track), 1);
    expect(saved()).toMatchObject({ index: 1, items: [{ title: "a" }, { title: "b" }, { title: "c" }] });
    playerStore.move(0, 2);
    expect(saved()).toMatchObject({ index: 0, items: [{ title: "b" }, { title: "c" }, { title: "a" }] });
    playerStore.stopAudio();
    expect(saved()).toBeNull();
  });

  it("does not write before the saved queue has been looked at", () => {
    window.localStorage.setItem(queueKey("user-1"), JSON.stringify({ items: [track("old")], index: 0 }));
    playerStore.playQueue([track("new")]);
    expect(saved()?.items[0]?.title).toBe("old");
  });

  it("brings the queue back paused after a reload", () => {
    playerStore.restore("user-1");
    playerStore.playQueue(["a", "b", "c"].map(track), 1);
    playerStore.forgetPerson();
    playerStore.setVideo(null);
    // A reload starts from nothing.
    const stored = window.localStorage.getItem(queueKey("user-1"))!;
    playerStore.stopAudio();
    window.localStorage.setItem(queueKey("user-1"), stored);
    expect(playerStore.restore("user-1")).toBe(true);
    expect(titles()).toEqual(["a", "b", "c"]);
    expect(current()).toBe("b");
    expect(playerStore.getSnapshot().transport).toMatchObject({ playing: false, loading: false });
    expect(new Set(playerStore.getSnapshot().queue.map((item) => item.queueId)).size).toBe(3);
  });

  it("restores for the person it was saved for, and no one else", () => {
    window.localStorage.setItem(queueKey("user-1"), JSON.stringify({ items: [track("mine")], index: 0 }));
    expect(playerStore.restore("user-2")).toBe(false);
    expect(titles()).toEqual([]);
  });

  it("does not restore over a queue that is already playing", () => {
    window.localStorage.setItem(queueKey("user-1"), JSON.stringify({ items: [track("old")], index: 0 }));
    playerStore.playQueue([track("new")]);
    expect(playerStore.restore("user-1")).toBe(false);
    expect(titles()).toEqual(["new"]);
  });

  it("resumes at the next audio entry when the saved playing entry is a video with no window to come back to", () => {
    window.localStorage.setItem(queueKey("user-1"), JSON.stringify({ items: [track("a"), episode("ep"), track("b")], index: 1 }));
    expect(playerStore.restore("user-1")).toBe(true);
    expect(current()).toBe("b");
  });

  it("keeps a saved video entry that is the mini player coming back", () => {
    window.localStorage.setItem(queueKey("user-1"), JSON.stringify({ items: [track("a"), episode("ep"), track("b")], index: 1 }));
    const activation = playerStore.getSnapshot().audioActivation;
    expect(playerStore.restore("user-1", { keepVideo: "file-ep" })).toBe(true);
    expect(current()).toBe("ep");
    expect(playerStore.getSnapshot().audioActivation).toBe(activation);
  });

  it("restores nothing when only a video was saved and no window came back", () => {
    window.localStorage.setItem(queueKey("user-1"), JSON.stringify({ items: [episode("ep")], index: 0 }));
    expect(playerStore.restore("user-1")).toBe(false);
  });

  it("ignores a saved queue that is corrupt, or entries that are not media", () => {
    window.localStorage.setItem(queueKey("user-1"), "{not json");
    expect(playerStore.restore("user-1")).toBe(false);
    window.localStorage.setItem(queueKey("user-1"), JSON.stringify({ items: [{ nonsense: true }, track("ok")], index: 1 }));
    expect(playerStore.restore("user-1")).toBe(true);
    expect(titles()).toEqual(["ok"]);
  });

  it("stops saving for a person who signed out", () => {
    playerStore.restore("user-1");
    playerStore.playQueue([track("a")]);
    playerStore.forgetPerson();
    playerStore.enqueue([track("b")]);
    expect(saved()?.items).toHaveLength(1);
  });

  it("carries on playing when the browser refuses storage", () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new DOMException("full", "QuotaExceededError"); });
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => { throw new DOMException("blocked", "SecurityError"); });
    expect(playerStore.restore("user-1")).toBe(false);
    expect(() => playerStore.playQueue(["a", "b"].map(track))).not.toThrow();
    expect(() => playerStore.updateTransport({ volume: 0.5 })).not.toThrow();
    expect(current()).toBe("a");
  });

  it("saves only the playing entry and what follows when the queue is very long", () => {
    playerStore.restore("user-1");
    playerStore.playQueue(Array.from({ length: 700 }, (_, index) => track(`t${index}`)), 650);
    const stored = saved()!;
    expect(stored.items).toHaveLength(500);
    expect(stored.items[stored.index]!.title).toBe("t650");
  });
});
