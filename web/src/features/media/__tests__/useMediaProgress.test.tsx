import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { PlaybackProgressForFilesDocument, type PlaybackProgressFieldsFragment } from "@/graphql/generated/graphql";

import { useMediaProgress } from "../useMediaProgress";
import { mediaHookHarness } from "./mediaHookHarness";

const progress = (mediaFileId: string, userId = "user-a", isWatched = false): PlaybackProgressFieldsFragment => ({
  id: `${userId}-${mediaFileId}`, userId, mediaFileId, isWatched,
  currentPosition: isWatched ? 100 : 20, duration: 100, progressPercent: isWatched ? 100 : 20,
  watchedAt: isWatched ? "2026-01-01" : null, createdAt: "2026-01-01", updatedAt: "2026-01-01",
});
const response = (...rows: PlaybackProgressFieldsFragment[]) => ({ playbackProgresses: { __typename: "PlaybackProgressConnection", edges: rows.map((node) => ({ __typename: "PlaybackProgressEdge", node: { __typename: "PlaybackProgress", ...node } })) } });
const ids = Array.from({ length: 205 }, (_, i) => `file-${String(i + 1).padStart(3, "0")}`);

describe("useMediaProgress", () => {
  it("queries disjoint batches of at most 100 files and finds watched state beyond the first batch", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useMediaProgress([...ids, ids[0]!], "user-a"), { wrapper });
    await waitFor(() => expect(requests).toHaveLength(3));
    const batches = requests.map(({ operation }) => operation.variables.mediaFileIds as string[]);
    expect(batches.map((batch) => batch.length)).toEqual([100, 100, 5]);
    expect(batches.flat()).toEqual(ids);
    expect(new Set(batches.flat()).size).toBe(ids.length);
    for (const { operation } of requests) {
      expect(operation.operationName).toBe("PlaybackProgressForFiles");
      expect(operation.variables.userId).toBe("user-a");
    }
    await act(async () => requests[1]!.resolve(response(progress(ids[100]!, "user-a", true))));
    expect(result.current.progressByFile.get(ids[100]!)?.isWatched).toBe(true);
    expect(result.current.loading).toBe(true);
    await act(async () => requests[0]!.resolve(response()));
    expect(result.current.loading).toBe(true);
    await act(async () => requests[2]!.resolve(response()));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.progressByFile.get(ids[100]!)?.isWatched).toBe(true);
  });

  it("clears the previous user's progress immediately and ignores their late responses", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result, rerender } = renderHook(({ userId }) => useMediaProgress(["file-1"], userId), { wrapper, initialProps: { userId: "user-a" } });
    await act(async () => requests[0]!.resolve(response(progress("file-1", "user-a", true))));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.progressByFile.get("file-1")?.userId).toBe("user-a");
    // Apollo aborts an in-flight refetch when the owning query is disposed.
    act(() => { void result.current.refetch().catch(() => {}); });
    await waitFor(() => expect(requests).toHaveLength(2));
    rerender({ userId: "user-b" });
    expect(result.current.progressByFile.size).toBe(0);
    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(requests).toHaveLength(3));
    await act(async () => requests[2]!.resolve(response(progress("file-1", "user-b"))));
    await act(async () => requests[1]!.resolve(response(progress("file-1", "user-a", true))));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.progressByFile.get("file-1")?.userId).toBe("user-b");
    expect(result.current.progressByFile.get("file-1")?.isWatched).toBe(false);
  });

  it("clears old file data during an ID change and ignores the old in-flight batch", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result, rerender } = renderHook(({ files }) => useMediaProgress(files, "user-a"), { wrapper, initialProps: { files: ["old-file"] } });
    rerender({ files: ["new-file"] });
    expect(result.current.progressByFile.size).toBe(0);
    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(requests).toHaveLength(2));
    await act(async () => requests[1]!.resolve(response(progress("new-file"))));
    await act(async () => requests[0]!.resolve(response(progress("old-file", "user-a", true))));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect([...result.current.progressByFile.keys()]).toEqual(["new-file"]);
  });

  it("reacts to Apollo cache writes without another network request", async () => {
    const { client, requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useMediaProgress(["file-1"], "user-a"), { wrapper });
    await act(async () => requests[0]!.resolve(response(progress("file-1"))));
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => {
      client.cache.writeQuery({ query: PlaybackProgressForFilesDocument, variables: { userId: "user-a", mediaFileIds: ["file-1"] }, data: response(progress("file-1", "user-a", true)) });
    });
    await waitFor(() => expect(result.current.progressByFile.get("file-1")?.isWatched).toBe(true));
    expect(requests).toHaveLength(1);
  });

  it("reports a failed batch and retries all batches before declaring loading finished", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const files = ids.slice(0, 101);
    const { result } = renderHook(() => useMediaProgress(files, "user-a"), { wrapper });
    await act(async () => requests[0]!.resolve(response(progress(files[0]!))));
    await act(async () => requests[1]!.reject(new Error("Progress unavailable")));
    await waitFor(() => expect(result.current.error?.message).toBe("Progress unavailable"));
    act(() => { void result.current.refetch(); });
    await waitFor(() => expect(requests).toHaveLength(4));
    expect(result.current.loading).toBe(true);
    await act(async () => requests[2]!.resolve(response()));
    expect(result.current.loading).toBe(true);
    await act(async () => requests[3]!.resolve(response(progress(files[100]!, "user-a", true))));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBeUndefined();
    expect(result.current.progressByFile.get(files[100]!)?.isWatched).toBe(true);
  });

  it("does not fetch without files or a signed-in user and clears data on sign-out", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result, rerender } = renderHook(({ files, userId }: { files: string[]; userId: string | undefined }) => useMediaProgress(files, userId), { wrapper, initialProps: { files: [] as string[], userId: "user-a" as string | undefined } });
    expect(requests).toHaveLength(0);
    expect(result.current.loading).toBe(false);
    rerender({ files: ["file-1"], userId: undefined });
    expect(requests).toHaveLength(0);
    rerender({ files: ["file-1"], userId: "user-a" });
    await act(async () => requests[0]!.resolve(response(progress("file-1"))));
    await waitFor(() => expect(result.current.progressByFile.size).toBe(1));
    rerender({ files: ["file-1"], userId: undefined });
    expect(result.current.progressByFile.size).toBe(0);
    expect(result.current.loading).toBe(false);
  });
});
