import { screen } from "@testing-library/react";
import type { ActionSet } from "glass-ux/actions";
import { describe, expect, it, vi } from "vitest";

import type { PlaylistEntity } from "@/features/personal/PlaylistIdentity";
import { usePlaylistActions } from "@/features/personal/usePlaylistActions";
import { playerStore } from "@/features/player/store";
import { renderWithProviders } from "@/test";

import { useAlbumActions } from "../useAlbumActions";
import { useAudiobookActions } from "../useAudiobookActions";
import { useMovieActions } from "../useMovieActions";
import { useShowActions } from "../useShowActions";

vi.mock("@/lib/auth/useSession", () => ({
  useSession: () => ({ user: { id: "user-1", role: "admin" }, status: "authenticated" }),
  useIsAdmin: () => true,
}));
vi.mock("@/features/personal/usePersonalLibrary", () => ({ useHomeShelves: () => ({ rows: [], complete: true, save: vi.fn() }) }));

const members = vi.hoisted(() => ({ fetchPlaylistTargets: vi.fn() }));
vi.mock("@/features/personal/usePlaylists", async (original) => ({ ...(await original<typeof import("@/features/personal/usePlaylists")>()), fetchPlaylistTargets: members.fetchPlaylistTargets }));
vi.mock("@/lib/auth/session", () => ({ session: { onChange: () => () => {} } }));

const settings = vi.fn();

/** Renders a hook's action set as a list of its action ids, so a test can read what an entity offers. */
function Offers({ use }: { use: () => ActionSet<never> }) {
  const set = use();
  return <ul aria-label="actions">{set.actions.map((action) => <li key={action.id}>{action.id}</li>)}</ul>;
}
async function offered(use: () => ActionSet<never>) {
  await renderWithProviders(<Offers use={use} />);
  return screen.getAllByRole("listitem").map((item) => item.textContent!);
}

describe("every playable entity offers the queue and playlist actions once", () => {
  const sets: Array<[string, () => ActionSet<never>, boolean]> = [
    ["movie", () => useMovieActions({ settings }).actions as ActionSet<never>, true],
    ["show", () => useShowActions({ settings }).actions as ActionSet<never>, true],
    ["album", () => useAlbumActions({ settings }).actions as ActionSet<never>, true],
    ["audiobook", () => useAudiobookActions({ settings }).actions as ActionSet<never>, true],
    ["playlist", () => usePlaylistActions({ settings }) as ActionSet<never>, false],
  ];

  it.each(sets)("%s has Play next and Add to queue, once each", async (_name, use) => {
    const ids = await offered(use);
    expect(ids.filter((id) => id === "play-next")).toHaveLength(1);
    expect(ids.filter((id) => id === "enqueue")).toHaveLength(1);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it.each(sets.filter(([, , addable]) => addable))("%s has Add to playlist, once", async (_name, use) => {
    expect((await offered(use)).filter((id) => id === "add-to-playlist")).toHaveLength(1);
  });

  it("a playlist cannot be added to a playlist, but plays, shuffles and queues", async () => {
    const ids = await offered(sets.find(([name]) => name === "playlist")![1]);
    expect(ids).not.toContain("add-to-playlist");
    expect(ids).toEqual(expect.arrayContaining(["play", "shuffle", "play-next", "enqueue"]));
  });
});

describe("playlist Play next and Add to queue", () => {
  const list = (id: string, kind: PlaylistEntity["kind"]) => ({ id, kind, name: id }) as PlaylistEntity;
  async function actionsFor() {
    let set!: ActionSet<PlaylistEntity>;
    function Capture() { set = usePlaylistActions<PlaylistEntity>({ settings }); return null; }
    await renderWithProviders(<Capture />);
    return (id: string) => set.actions.find((action) => action.id === id)!;
  }

  it("is hidden for a smart collection, which has no entries, and offered for a selection of playlists", async () => {
    const action = await actionsFor();
    expect(action("enqueue").visible!([list("a", "SMART")])).toBe(false);
    expect(action("enqueue").visible!([list("a", "MANUAL"), list("b", "MANUAL")])).toBe(true);
    expect(action("enqueue").bulk).toBe(true);
    expect(action("play").visible!([list("a", "MANUAL"), list("b", "MANUAL")])).toBe(false);
  });

  it("says so for an empty playlist rather than queueing nothing", async () => {
    const track = (id: string) => ({ mediaFileId: `file-${id}`, title: id, entity: { kind: "track" as const, id } });
    playerStore.playQueue([track("now"), track("later")]);
    members.fetchPlaylistTargets.mockResolvedValue([]);
    const action = await actionsFor();
    // An empty playlist has nothing to queue, and says so rather than queueing nothing.
    await expect(action("play-next").run!([list("empty", "MANUAL")], {} as never)).rejects.toThrow("Nothing here is ready to play");
    expect(playerStore.getSnapshot().queue.map((item) => item.entity.id)).toEqual(["now", "later"]);
    playerStore.stopAudio();
  });
});
