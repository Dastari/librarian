import { beforeEach, describe, expect, it } from "vitest";

import type { PlayItem } from "../store";
import { clearVideoSession, readMiniRect, readVideoSession, saveMiniRect, saveVideoSession } from "../videoSession";

const item: PlayItem = { mediaFileId: "file", title: "A film", entity: { kind: "movie", id: "movie" } };

describe("the mini player's saved session", () => {
  beforeEach(() => window.localStorage.clear());

  it("comes back for the same person, and not for anyone else", () => {
    saveVideoSession("me", { item, position: 120, playing: true });
    expect(readVideoSession("me")).toEqual({ item, position: 120, playing: true });
    expect(readVideoSession("someone-else")).toBeNull();
  });

  it("is gone once closed", () => {
    saveVideoSession("me", { item, position: 120, playing: true });
    clearVideoSession("me");
    expect(readVideoSession("me")).toBeNull();
  });

  it("ignores anything stored that it can't use", () => {
    window.localStorage.setItem("librarian.player.mini.me", "{not json");
    expect(readVideoSession("me")).toBeNull();
    window.localStorage.setItem("librarian.player.mini.me", JSON.stringify({ item: { title: "no file" } }));
    expect(readVideoSession("me")).toBeNull();
  });

  it("remembers where the window was", () => {
    expect(readMiniRect()).toBeUndefined();
    saveMiniRect({ x: 10, y: 20, width: 480, height: 270 });
    expect(readMiniRect()).toEqual({ x: 10, y: 20, width: 480, height: 270 });
  });
});
