import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useWantMissing, wantMissingActions } from "../wantMissing";
import { mediaHookHarness } from "./mediaHookHarness";

describe("wanting a title's missing chapters or tracks", () => {
  it("wants only the chapters with no file, across every selected book", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useWantMissing(), { wrapper });
    let done: Promise<number>;
    act(() => { done = result.current("chapters", ["book-a", "book-b"], true); });
    await waitFor(() => expect(requests).toHaveLength(1));
    const { operation, resolve } = requests[0]!;
    expect(operation.operationName).toBe("EntityChapterUpdateMany");
    expect(operation.variables).toEqual({ where: { audiobookId: { inList: ["book-a", "book-b"] }, mediaFileId: { isNull: true } }, input: { wanted: true } });
    resolve({ updateChapters: { __typename: "UpdateChaptersResult", success: true, error: null, affectedCount: 7 } });
    await expect(done!).resolves.toBe(7);
  });

  it("stops wanting an album's missing tracks, leaving tracks with files alone", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useWantMissing(), { wrapper });
    let done: Promise<number>;
    act(() => { done = result.current("tracks", ["album-a"], false); });
    await waitFor(() => expect(requests).toHaveLength(1));
    const { operation, resolve } = requests[0]!;
    expect(operation.operationName).toBe("EntityTrackUpdateMany");
    expect(operation.variables).toEqual({ where: { albumId: { inList: ["album-a"] }, mediaFileId: { isNull: true } }, input: { wanted: false } });
    resolve({ updateTracks: { __typename: "UpdateTracksResult", success: false, error: "denied", affectedCount: 0 } });
    await expect(done!).rejects.toThrow();
  });

  it("offers Want and Stop wanting for any selection, then reloads the page", async () => {
    const wantMissing = vi.fn(async () => 3);
    const onChanged = vi.fn();
    const [want, unwant] = wantMissingActions<{ id: string }>("tracks", wantMissing, onChanged);
    expect(want!.label).toBe("Want missing tracks");
    expect(unwant!.label).toBe("Stop wanting missing tracks");
    expect(want!.bulk).toBe(true);
    await want!.run([{ id: "a" }, { id: "b" }]);
    expect(wantMissing).toHaveBeenCalledWith("tracks", ["a", "b"], true);
    await unwant!.run([{ id: "a" }]);
    expect(wantMissing).toHaveBeenLastCalledWith("tracks", ["a"], false);
    expect(onChanged).toHaveBeenCalledTimes(2);
  });

  it("offers Stop wanting only where something is wanted", () => {
    const [want, unwant] = wantMissingActions<{ id: string; wantedMissing?: { pageInfo: { totalCount?: number | null } } | null }>("chapters", vi.fn(async () => 0));
    const wanted = (n: number) => ({ id: `r${n}`, wantedMissing: { pageInfo: { totalCount: n } } });
    expect(unwant!.visible?.([wanted(0)])).toBe(false);
    expect(unwant!.visible?.([wanted(0), wanted(2)])).toBe(true);
    expect(unwant!.visible?.([wanted(4)])).toBe(true);
    // A row that never read the count keeps the action, so a surface without it is not stripped.
    expect(unwant!.visible?.([{ id: "unknown" }])).toBe(true);
    expect(want!.visible).toBeUndefined();
  });
});
