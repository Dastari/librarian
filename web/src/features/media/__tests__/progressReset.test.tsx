import { act, renderHook, waitFor } from "@testing-library/react";
import { defineActions, resolveActions } from "glass-ux/actions";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { PlaybackProgressFieldsFragment } from "@/graphql/generated/graphql";

import { filesWithProgress, hasSavedProgress, resetLabel, resetProgressAction, resetWhere, useResetProgress, type ProgressReset } from "../progressReset";
import { useMediaProgress } from "../useMediaProgress";
import { mediaHookHarness } from "./mediaHookHarness";

let userId: string | undefined = "user-a";
vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: userId ? { id: userId } : null }), useIsAdmin: () => false }));
beforeEach(() => { userId = "user-a"; });

const saved = (currentPosition: number, isWatched = false) => ({ currentPosition, isWatched });
const progressRow = (mediaFileId: string, overrides: Partial<PlaybackProgressFieldsFragment> = {}): PlaybackProgressFieldsFragment => ({
  id: `progress-${mediaFileId}`, userId: "user-a", mediaFileId, isWatched: false, currentPosition: 20, duration: 100, progressPercent: 20,
  watchedAt: null, createdAt: "2026-01-01", updatedAt: "2026-01-01", ...overrides,
});
const rows = (...nodes: PlaybackProgressFieldsFragment[]) => ({ playbackProgresses: { __typename: "PlaybackProgressConnection", edges: nodes.map((node) => ({ __typename: "PlaybackProgressEdge", node: { __typename: "PlaybackProgress", ...node } })) } });
const deleted = (success = true, error: string | null = null) => ({ deletePlaybackProgresses: { __typename: "DeletePlaybackProgressesResult", success, error } });

describe("which progress counts", () => {
  it("is a finished mark or a saved position, and nothing else", () => {
    expect(hasSavedProgress(undefined)).toBe(false);
    expect(hasSavedProgress(null)).toBe(false);
    expect(hasSavedProgress(saved(0))).toBe(false);
    expect(hasSavedProgress(saved(12))).toBe(true);
    // "Mark as watched" leaves a finished row with no position.
    expect(hasSavedProgress(saved(0, true))).toBe(true);
  });

  it("lists each file with progress once, in order, ignoring files without a file or progress", () => {
    const progress = new Map([["a", saved(5)], ["b", saved(0)], ["c", saved(0, true)]]);
    expect(filesWithProgress(["c", null, "a", undefined, "b", "a", "missing"], progress)).toEqual(["c", "a"]);
  });
});

describe("the reset action", () => {
  type Row = { id: string; file: string | null };
  const progress = new Map([["file-1", saved(30)], ["file-3", saved(0, true)]]);
  const reset: ProgressReset<Row> = { files: (items) => filesWithProgress(items.map((item) => item.file), progress) };
  const build = (media: "video" | "audio", over: ProgressReset<Row> | undefined = reset, run = vi.fn(async () => {})) =>
    ({ run, set: defineActions<Row>("row", resetProgressAction(media, over, run)) });
  const labels = (set: ReturnType<typeof build>["set"], rowsIn: Row[], surface: "menu" | "bulk" = "menu") =>
    resolveActions(set, rowsIn, surface).flatMap((group) => group.actions.map((action) => action.label));

  it("is worded for the medium", () => {
    expect(resetLabel("video")).toBe("Mark as unwatched");
    expect(resetLabel("audio")).toBe("Mark as unplayed");
    expect(labels(build("video").set, [{ id: "1", file: "file-1" }])).toEqual(["Mark as unwatched"]);
    expect(labels(build("audio").set, [{ id: "1", file: "file-1" }])).toEqual(["Mark as unplayed"]);
  });

  it("is hidden while the rows have no progress, and not defined when the page never read progress", () => {
    const { set } = build("video");
    expect(labels(set, [{ id: "2", file: "file-2" }])).toEqual([]);
    expect(labels(set, [{ id: "2", file: null }])).toEqual([]);
    expect(resetProgressAction("video", undefined, vi.fn())).toEqual([]);
  });

  it("offers a selection when any row has progress, and resets only the files that do", async () => {
    const { set, run } = build("video");
    const selection = [{ id: "1", file: "file-1" }, { id: "2", file: "file-2" }, { id: "3", file: "file-3" }];
    expect(labels(set, selection, "bulk")).toEqual(["Mark as unwatched"]);
    const [action] = resolveActions(set, selection, "bulk").flatMap((group) => group.actions);
    await action!.definition.run!(selection, {} as never);
    expect(run).toHaveBeenCalledWith(["file-1", "file-3"]);
  });

  it("stays visible but disabled, with its reason, until the page has loaded what it would reset", () => {
    const { set } = build("audio", { ...reset, disabledReason: () => "Loading all tracks" });
    const [action] = resolveActions(set, [{ id: "1", file: "file-1" }], "menu").flatMap((group) => group.actions);
    expect(action).toMatchObject({ label: "Mark as unplayed", disabledReason: "Loading all tracks" });
  });
});

describe("useResetProgress", () => {
  const ids = Array.from({ length: 205 }, (_, index) => `file-${String(index + 1).padStart(3, "0")}`);

  it("deletes only this person's rows for the files, in batches of 100", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useResetProgress(), { wrapper });
    let done: Promise<void>;
    act(() => { done = result.current([...ids, ids[0]!]); });
    for (const [index, size] of [100, 100, 5].entries()) {
      await waitFor(() => expect(requests).toHaveLength(index + 1));
      const { operation } = requests[index]!;
      expect(operation.operationName).toBe("EntityPlaybackProgressDeleteMany");
      const files = ids.slice(index * 100, index * 100 + size);
      expect(operation.variables).toEqual({ where: { userId: { eq: "user-a" }, mediaFileId: { inList: files } } });
      expect(operation.variables.where).toEqual(resetWhere("user-a", files));
      await act(async () => requests[index]!.resolve(deleted()));
    }
    await expect(done!).resolves.toBeUndefined();
  });

  it("refuses without a signed-in user and does nothing for no files", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useResetProgress(), { wrapper });
    await expect(result.current([])).resolves.toBeUndefined();
    expect(requests).toHaveLength(0);
    userId = undefined;
    const signedOut = renderHook(() => useResetProgress(), { wrapper });
    await expect(signedOut.result.current(["file-1"])).rejects.toThrow("Sign in");
    expect(requests).toHaveLength(0);
  });

  it("rejects an unsuccessful payload with the server's reason, so the action reports one failure", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useResetProgress(), { wrapper });
    let outcome: Promise<void>;
    act(() => { outcome = result.current(["file-1"]); });
    const failed = expect(outcome!).rejects.toThrow("Access denied");
    await waitFor(() => expect(requests).toHaveLength(1));
    await act(async () => requests[0]!.resolve(deleted(false, "Access denied")));
    await failed;
  });

  it("brings the progress on screen back to unplayed without a reload", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => ({ reset: useResetProgress(), progress: useMediaProgress(["file-1", "file-2"], "user-a") }), { wrapper });
    await waitFor(() => expect(requests).toHaveLength(1));
    await act(async () => requests[0]!.resolve(rows(progressRow("file-1"), progressRow("file-2", { isWatched: true }))));
    await waitFor(() => expect([...result.current.progress.progressByFile.keys()]).toEqual(["file-1", "file-2"]));

    let done: Promise<void>;
    act(() => { done = result.current.reset(["file-1", "file-2"]); });
    await waitFor(() => expect(requests).toHaveLength(2));
    expect(requests[1]!.operation.operationName).toBe("EntityPlaybackProgressDeleteMany");
    await act(async () => requests[1]!.resolve(deleted()));
    // The watched list is read again as soon as the delete lands.
    await waitFor(() => expect(requests).toHaveLength(3));
    expect(requests[2]!.operation.operationName).toBe("PlaybackProgressForFiles");
    await act(async () => requests[2]!.resolve(rows()));
    await done!;
    await waitFor(() => expect(result.current.progress.progressByFile.size).toBe(0));
  });
});
