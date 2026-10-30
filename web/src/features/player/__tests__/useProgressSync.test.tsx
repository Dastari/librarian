import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  EntityPlaybackProgressCreateDocument, EntityPlaybackProgressUpdateDocument,
  EntityPlaybackSessionCreateDocument, EntityPlaybackSessionUpdateDocument,
  PlaybackProgressForFileDocument,
  type PlaybackProgressFieldsFragment,
} from "@/graphql/generated/graphql";

import type { PlayItem } from "../store";
import { useProgressSync } from "../useProgressSync";

type Mutation = { mutation: unknown; variables: { id?: string; input: Record<string, unknown> } };
type Read = { query: unknown; variables: { userId: string; mediaFileId: string } };
const runtime = vi.hoisted(() => ({
  userId: "user-a" as string | null,
  epoch: "authority-a",
  sessionEpoch: 0,
  rows: new Map<string, PlaybackProgressFieldsFragment[]>(),
  loading: new Set<string>(),
  refresh: undefined as (() => Promise<void>) | undefined,
  changed: undefined as (() => void) | undefined,
  client: { mutate: vi.fn(), query: vi.fn() },
}));
vi.mock("@apollo/client/react", () => ({ useApolloClient: () => runtime.client }));
vi.mock("@/lib/auth/session", () => ({ session: { getEpoch: () => runtime.sessionEpoch, onChange: (listener: () => void) => { runtime.changed = listener; } } }));
vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: runtime.userId ? { id: runtime.userId } : null }) }));
vi.mock("@/lib/graphql/live/hooks", () => ({
  useLiveEpoch: () => runtime.epoch,
  useLiveInvalidation: (_document: unknown, _variables: unknown, refresh: () => Promise<void>) => { runtime.refresh = refresh; },
  useLiveQuery: (_document: unknown, options?: { variables: { userId: string; mediaFileId: string } }) => {
    if (!options) return { data: { appSettings: { edges: [{ node: { value: "15" } }] } } };
    const key = `${options.variables.userId}:${options.variables.mediaFileId}`;
    return {
      variables: options.variables,
      loading: runtime.loading.has(key),
      data: runtime.loading.has(key) ? undefined : { playbackProgresses: { edges: (runtime.rows.get(key) ?? []).map((node) => ({ node })) } },
    };
  },
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((settle) => { resolve = settle; });
  return { promise, resolve };
}
const item = (id: string): PlayItem => ({ mediaFileId: id, title: id, entity: { kind: "track", id: `track-${id}` } });
let sessionNumber = 0;
function response({ mutation, variables: { input, id } }: Mutation) {
  if (mutation === EntityPlaybackProgressCreateDocument) {
    const row = { ...input, id: `progress-${input.userId}-${input.mediaFileId}`, createdAt: "", updatedAt: "" } as PlaybackProgressFieldsFragment;
    runtime.rows.set(`${input.userId}:${input.mediaFileId}`, [row]);
    return { data: { createPlaybackProgress: { success: true, playbackProgress: { id: row.id } } } };
  }
  if (mutation === EntityPlaybackProgressUpdateDocument) {
    for (const [key, rows] of runtime.rows) runtime.rows.set(key, rows.map((row) => row.id === id ? { ...row, ...input } : row));
    return { data: { updatePlaybackProgress: { success: true, playbackProgress: { id } } } };
  }
  if (mutation === EntityPlaybackSessionCreateDocument) return { data: { createPlaybackSession: { success: true, playbackSession: { id: `session-${input.userId}-${input.mediaFileId}-${++sessionNumber}` } } } };
  return { data: { updatePlaybackSession: { success: true, playbackSession: { id } } } };
}
const calls = (document: unknown): Mutation[] => runtime.client.mutate.mock.calls.map(([request]) => request as Mutation).filter((request) => request.mutation === document);
const settle = async () => { await act(async () => { await Promise.resolve(); }); };
beforeEach(() => {
  runtime.userId = "user-a";
  runtime.epoch = "authority-a";
  runtime.sessionEpoch = 0;
  runtime.rows.clear();
  runtime.loading.clear();
  runtime.refresh = undefined;
  runtime.changed?.();
  sessionNumber = 0;
  runtime.client.mutate.mockReset().mockImplementation(async (request: Mutation) => response(request));
  runtime.client.query.mockReset().mockImplementation(async ({ variables }: Read) => {
    const key = `${variables.userId}:${variables.mediaFileId}`;
    if (runtime.loading.has(key)) throw new Error("Read unavailable");
    return { data: { playbackProgresses: { edges: (runtime.rows.get(key) ?? []).map((node) => ({ node: { ...node } })) } } };
  });
});
afterEach(async () => { cleanup(); await settle(); vi.restoreAllMocks(); });

describe("useProgressSync serialization", () => {
  it.each(["checkpoint", "session"])("shares the file lane across actual A → B → A unmount/remounts with delayed %s creation", async (delay) => {
    const pending = deferred<ReturnType<typeof response>>();
    const delayedDocument = delay === "checkpoint" ? EntityPlaybackProgressCreateDocument : EntityPlaybackSessionCreateDocument;
    runtime.client.mutate.mockImplementation((request: Mutation) => request.mutation === delayedDocument && request.variables.input.mediaFileId === "a"
      ? pending.promise : Promise.resolve(response(request)));
    const first = renderHook(() => useProgressSync(item("a")));
    act(() => first.result.current.report(40, 300, true));
    await settle();
    const oldRequest = calls(delayedDocument)[0]!;
    first.unmount();
    const middle = renderHook(() => useProgressSync(item("b")));
    act(() => middle.result.current.report(5, 100, true));
    await settle();
    middle.unmount();
    const last = renderHook(() => useProgressSync(item("a")));
    await settle();
    expect(last.result.current.loading).toBe(true);
    // The new snapshot must wait for the old checkpoint/session and final write.
    expect(runtime.client.query.mock.calls.filter(([request]) => (request as Read).variables.mediaFileId === "a")).toHaveLength(1);
    await act(async () => pending.resolve(response(oldRequest)));
    await settle();
    expect(last.result.current.loading).toBe(false);
    expect(last.result.current.resumePosition).toBe(40);
    expect(runtime.client.query.mock.lastCall?.[0]).toMatchObject({ query: PlaybackProgressForFileDocument, fetchPolicy: "no-cache", context: { queryDeduplication: false } });
    // Only the old creation is delayed; the new activation gets its own session.
    runtime.client.mutate.mockImplementation(async (request: Mutation) => response(request));
    act(() => last.result.current.report(70, 300, true));
    await act(async () => last.result.current.flush(true));
    expect(calls(EntityPlaybackProgressCreateDocument).filter(({ variables }) => variables.input.mediaFileId === "a")).toHaveLength(1);
    const sessions = calls(EntityPlaybackSessionCreateDocument).filter(({ variables }) => variables.input.mediaFileId === "a");
    expect(sessions.map(({ variables }) => variables.input.currentPosition)).toEqual([40, 70]);
    const stops = calls(EntityPlaybackSessionUpdateDocument).filter(({ variables }) => variables.id?.startsWith("session-user-a-a-") && variables.input.isPlaying === false);
    expect(stops).toHaveLength(1);
    expect(stops[0]?.variables.input.currentPosition).toBe(40);
    expect(calls(EntityPlaybackProgressUpdateDocument).at(-1)?.variables).toMatchObject({ id: "progress-user-a-a", input: { currentPosition: 70 } });
  });

  it("allows an older same-file mount to close its session without overwriting the newer mount's checkpoint", async () => {
    const old = renderHook(() => useProgressSync(item("same")));
    act(() => old.result.current.report(40, 300, true));
    await settle();
    const current = renderHook(() => useProgressSync(item("same")));
    await settle();
    act(() => current.result.current.report(90, 300, true));
    await act(async () => current.result.current.flush(true));
    const before = calls(EntityPlaybackProgressUpdateDocument).length;
    old.unmount();
    await settle();
    expect(calls(EntityPlaybackProgressUpdateDocument)).toHaveLength(before);
    expect(runtime.rows.get("user-a:same")?.[0]?.currentPosition).toBe(90);
    expect(calls(EntityPlaybackSessionUpdateDocument).at(-1)?.variables).toMatchObject({ id: "session-user-a-same-1", input: { currentPosition: 40, isPlaying: false } });
    expect(calls(EntityPlaybackSessionUpdateDocument).filter(({ variables }) => variables.id === "session-user-a-same-2").every(({ variables }) => variables.input.isPlaying === true)).toBe(true);
  });

  it("does not expose or reuse an earlier same-file mount's delayed read", async () => {
    const stale = deferred<{ data: { playbackProgresses: { edges: { node: PlaybackProgressFieldsFragment }[] } } }>();
    const row: PlaybackProgressFieldsFragment = { id: "old-row", userId: "user-a", mediaFileId: "same", currentPosition: 10, duration: 300, progressPercent: 3, isWatched: false, watchedAt: null, createdAt: "", updatedAt: "" };
    runtime.client.query.mockReturnValueOnce(stale.promise);
    const old = renderHook(() => useProgressSync(item("same")));
    await settle();
    old.unmount();
    runtime.rows.set("user-a:same", [{ ...row, id: "current-row", currentPosition: 80 }]);
    const current = renderHook(() => useProgressSync(item("same")));
    await settle();
    expect(runtime.client.query).toHaveBeenCalledTimes(1);
    expect(current.result.current.resumePosition).toBeUndefined();
    expect(current.result.current.loading).toBe(true);
    await act(async () => stale.resolve({ data: { playbackProgresses: { edges: [{ node: row }] } } }));
    await settle();
    expect(runtime.client.query).toHaveBeenCalledTimes(2);
    expect(current.result.current.resumePosition).toBe(80);
    act(() => current.result.current.report(85, 300, true));
    await act(async () => current.result.current.flush(true));
    expect(calls(EntityPlaybackProgressCreateDocument)).toHaveLength(0);
    expect(calls(EntityPlaybackProgressUpdateDocument).every(({ variables }) => variables.id === "current-row")).toBe(true);
  });

  it("forgets settled idle lanes and reads a replacement checkpoint ID on a later mount", async () => {
    const old = renderHook(() => useProgressSync(item("file")));
    act(() => old.result.current.report(40, 300, true));
    await settle();
    old.unmount();
    await settle();
    // A server-side delete between visits must permit a fresh checkpoint create.
    runtime.rows.clear();
    const current = renderHook(() => useProgressSync(item("file")));
    await settle();
    expect(current.result.current.resumePosition).toBeUndefined();
    act(() => current.result.current.report(7, 300, true));
    await act(async () => current.result.current.flush(true));
    expect(calls(EntityPlaybackProgressCreateDocument).map(({ variables }) => variables.input.currentPosition)).toEqual([40, 7]);
    expect(calls(EntityPlaybackSessionCreateDocument)).toHaveLength(2);
  });

  it("replaces authority across mounts without waiting on or reusing old private work", async () => {
    const pending = deferred<ReturnType<typeof response>>();
    runtime.client.mutate.mockImplementation((request: Mutation) => request.mutation === EntityPlaybackProgressCreateDocument && request.variables.input.currentPosition === 40
      ? pending.promise : Promise.resolve(response(request)));
    const old = renderHook(() => useProgressSync(item("file")));
    act(() => old.result.current.report(40, 300, true));
    await settle();
    const oldRequest = calls(EntityPlaybackProgressCreateDocument)[0]!;
    old.unmount();
    runtime.sessionEpoch += 1;
    runtime.epoch = "replacement-authority";
    runtime.changed?.();
    const current = renderHook(() => useProgressSync(item("file")));
    await settle();
    expect(current.result.current.loading).toBe(false);
    act(() => current.result.current.report(8, 300, true));
    await act(async () => current.result.current.flush(true));
    await act(async () => pending.resolve(response(oldRequest)));
    await settle();
    expect(calls(EntityPlaybackSessionCreateDocument).map(({ variables }) => variables.input.currentPosition)).toEqual([8]);
    expect(calls(EntityPlaybackSessionUpdateDocument).every(({ variables }) => variables.input.isPlaying === true)).toBe(true);
    expect(calls(EntityPlaybackProgressUpdateDocument).every(({ variables }) => variables.input.currentPosition === 8)).toBe(true);
  });

  it("opens a new local session for a same-file activation key and preserves it through metadata updates", async () => {
    const { result, rerender } = renderHook(({ activationKey, selected }) => useProgressSync(selected, activationKey), { initialProps: { activationKey: 1, selected: item("file") } });
    act(() => result.current.report(40, 300, true));
    await settle();
    rerender({ activationKey: 2, selected: item("file") });
    await settle();
    act(() => result.current.report(12, 300, true));
    await act(async () => result.current.flush(true));
    rerender({ activationKey: 2, selected: { ...item("file"), title: "Renamed" } });
    await act(async () => result.current.flush(true));
    expect(calls(EntityPlaybackProgressCreateDocument)).toHaveLength(1);
    expect(calls(EntityPlaybackSessionCreateDocument).map(({ variables }) => variables.input.currentPosition)).toEqual([40, 12]);
    expect(calls(EntityPlaybackSessionUpdateDocument).filter(({ variables }) => variables.input.isPlaying === false)).toHaveLength(1);
  });

  it.each(["checkpoint", "session"])("isolates delayed old %s creation, its stop, and the new file's IDs/position", async (delay) => {
    const old = deferred<ReturnType<typeof response>>();
    const delayedDocument = delay === "checkpoint" ? EntityPlaybackProgressCreateDocument : EntityPlaybackSessionCreateDocument;
    runtime.client.mutate.mockImplementation((request: Mutation) => request.mutation === delayedDocument && request.variables.input.mediaFileId === "old"
      ? old.promise : Promise.resolve(response(request)));
    const { result, rerender } = renderHook(({ selected }) => useProgressSync(selected), { initialProps: { selected: item("old") } });
    act(() => result.current.report(45, 300, true));
    await settle();
    rerender({ selected: item("new") });
    act(() => result.current.report(8, 180, true));
    await settle();
    const oldRequest = calls(delayedDocument).find((request) => request.variables.input.mediaFileId === "old")!;
    await act(async () => old.resolve(response(oldRequest)));
    await settle();
    act(() => result.current.report(12, 180, true));
    await act(async () => result.current.flush(true));
    expect(calls(EntityPlaybackProgressCreateDocument).map(({ variables }) => [variables.input.mediaFileId, variables.input.currentPosition])).toEqual([["old", 45], ["new", 8]]);
    const oldStop = calls(EntityPlaybackSessionUpdateDocument).find(({ variables }) => variables.id?.includes("-old-") && variables.input.isPlaying === false);
    expect(oldStop?.variables.input.currentPosition).toBe(45);
    expect(calls(EntityPlaybackSessionUpdateDocument).some(({ variables }) => variables.id?.includes("-new-") && variables.input.isPlaying === false)).toBe(false);
    expect(calls(EntityPlaybackProgressUpdateDocument).at(-1)?.variables).toMatchObject({ id: "progress-user-a-new", input: { currentPosition: 12, duration: 180 } });
    expect(calls(EntityPlaybackSessionUpdateDocument).at(-1)?.variables).toMatchObject({ id: expect.stringContaining("-new-"), input: { currentPosition: 12, isPlaying: true } });
  });

  it("serializes concurrent forced flushes behind a single checkpoint create", async () => {
    const create = deferred<ReturnType<typeof response>>();
    runtime.client.mutate.mockImplementation((request: Mutation) => request.mutation === EntityPlaybackProgressCreateDocument ? create.promise : Promise.resolve(response(request)));
    const { result } = renderHook(() => useProgressSync(item("one")));
    act(() => result.current.report(10, 100, true));
    await settle();
    let first!: Promise<void>;
    let second!: Promise<void>;
    act(() => {
      result.current.report(20, 100, false);
      first = result.current.flush(true);
      second = result.current.flush(true);
    });
    expect(calls(EntityPlaybackProgressCreateDocument)).toHaveLength(1);
    await act(async () => {
      create.resolve(response(calls(EntityPlaybackProgressCreateDocument)[0]!));
      await Promise.all([first, second]);
    });
    expect(calls(EntityPlaybackProgressCreateDocument)).toHaveLength(1);
    expect(calls(EntityPlaybackProgressUpdateDocument)).toHaveLength(2);
    expect(calls(EntityPlaybackProgressUpdateDocument).every(({ variables }) => variables.id === "progress-user-a-one")).toBe(true);
    expect(calls(EntityPlaybackSessionCreateDocument)).toHaveLength(1);
  });

  it("reuses the pending checkpoint lane through A → B → A, while closing only A's old session", async () => {
    const create = deferred<ReturnType<typeof response>>();
    runtime.client.mutate.mockImplementation((request: Mutation) => request.mutation === EntityPlaybackProgressCreateDocument && request.variables.input.mediaFileId === "a"
      ? create.promise : Promise.resolve(response(request)));
    const { result, rerender } = renderHook(({ selected }) => useProgressSync(selected), { initialProps: { selected: item("a") } });
    act(() => result.current.report(40, 300, true));
    await settle();
    rerender({ selected: item("b") });
    act(() => result.current.report(5, 100, true));
    rerender({ selected: item("a") });
    act(() => result.current.report(70, 300, true));
    await act(async () => create.resolve(response(calls(EntityPlaybackProgressCreateDocument).find(({ variables }) => variables.input.mediaFileId === "a")!)));
    await act(async () => result.current.flush(true));
    expect(calls(EntityPlaybackProgressCreateDocument).filter(({ variables }) => variables.input.mediaFileId === "a")).toHaveLength(1);
    const aSessions = calls(EntityPlaybackSessionCreateDocument).filter(({ variables }) => variables.input.mediaFileId === "a");
    expect(aSessions.map(({ variables }) => variables.input.currentPosition)).toEqual([40, 70]);
    const aStops = calls(EntityPlaybackSessionUpdateDocument).filter(({ variables }) => variables.id?.startsWith("session-user-a-a-") && variables.input.isPlaying === false);
    expect(aStops).toHaveLength(1);
    expect(aStops[0]?.variables.input.currentPosition).toBe(40);
    expect(calls(EntityPlaybackSessionUpdateDocument).at(-1)?.variables.input).toMatchObject({ currentPosition: 70, isPlaying: true });
  });

  it("discards queued old-principal writes after a delayed mutation and never reuses their IDs", async () => {
    const create = deferred<ReturnType<typeof response>>();
    runtime.client.mutate.mockImplementation((request: Mutation) => request.mutation === EntityPlaybackProgressCreateDocument && request.variables.input.userId === "user-a"
      ? create.promise : Promise.resolve(response(request)));
    const { result, rerender } = renderHook(() => useProgressSync(item("same")));
    act(() => result.current.report(80, 300, true));
    await settle();
    runtime.userId = "user-b";
    runtime.epoch = "authority-b";
    rerender();
    act(() => result.current.report(4, 300, true));
    await settle();
    await act(async () => create.resolve(response(calls(EntityPlaybackProgressCreateDocument)[0]!)));
    await act(async () => result.current.flush(true));
    expect(calls(EntityPlaybackSessionCreateDocument).map(({ variables }) => variables.input.userId)).toEqual(["user-b"]);
    expect(calls(EntityPlaybackProgressUpdateDocument).every(({ variables }) => variables.id === "progress-user-b-same")).toBe(true);
    expect(calls(EntityPlaybackSessionUpdateDocument).every(({ variables }) => variables.id?.startsWith("session-user-b-"))).toBe(true);
  });

  it("drops unmount cleanup after authority changes even without a new hook render", async () => {
    const create = deferred<ReturnType<typeof response>>();
    runtime.client.mutate.mockImplementation((request: Mutation) => request.mutation === EntityPlaybackSessionCreateDocument
      ? create.promise : Promise.resolve(response(request)));
    const { result, unmount } = renderHook(() => useProgressSync(item("old")));
    act(() => result.current.report(60, 300, true));
    await settle();
    expect(calls(EntityPlaybackSessionCreateDocument)).toHaveLength(1);
    runtime.sessionEpoch += 1;
    unmount();
    await act(async () => create.resolve(response(calls(EntityPlaybackSessionCreateDocument)[0]!)));
    await settle();
    expect(calls(EntityPlaybackProgressUpdateDocument)).toHaveLength(0);
    expect(calls(EntityPlaybackSessionUpdateDocument)).toHaveLength(0);
  });

  it("does not use stale query positions or IDs for a different file/principal", async () => {
    const stale: PlaybackProgressFieldsFragment = {
      id: "stale-progress", userId: "other-user", mediaFileId: "other-file", currentPosition: 250,
      duration: 300, progressPercent: 83, isWatched: false, watchedAt: null, createdAt: "", updatedAt: "",
    };
    runtime.rows.set("user-a:new", [stale]);
    const { result, rerender } = renderHook(() => useProgressSync(item("new")));
    expect(result.current.resumePosition).toBeUndefined();
    expect(result.current.loading).toBe(true);
    act(() => result.current.report(10, 100, false));
    await act(async () => result.current.flush(true));
    expect(runtime.client.mutate).not.toHaveBeenCalled();
    runtime.rows.set("user-a:new", []);
    rerender();
    await act(async () => runtime.refresh?.());
    expect(result.current.loading).toBe(false);
    await act(async () => result.current.flush(true));
    expect(calls(EntityPlaybackProgressCreateDocument)[0]?.variables.input).toMatchObject({ userId: "user-a", mediaFileId: "new", currentPosition: 10 });
  });

  it("checks payload success before retaining IDs and retries a failed create", async () => {
    let rejectProgress = true;
    let rejectSession = true;
    runtime.client.mutate.mockImplementation(async (request: Mutation) => {
      if (request.mutation === EntityPlaybackProgressCreateDocument && rejectProgress) {
        rejectProgress = false;
        return { data: { createPlaybackProgress: { success: false, error: "Denied", playbackProgress: { id: "invalid-progress" } } } };
      }
      if (request.mutation === EntityPlaybackSessionCreateDocument && rejectSession) {
        rejectSession = false;
        return { data: { createPlaybackSession: { success: false, error: "Denied", playbackSession: { id: "invalid-session" } } } };
      }
      return response(request);
    });
    const { result } = renderHook(() => useProgressSync(item("file")));
    act(() => result.current.report(10, 100, true));
    await settle();
    await act(async () => result.current.flush());
    expect(calls(EntityPlaybackProgressCreateDocument)).toHaveLength(2);
    expect(calls(EntityPlaybackSessionCreateDocument)).toHaveLength(2);
    await act(async () => result.current.flush(true));
    expect(calls(EntityPlaybackProgressUpdateDocument)[0]?.variables.id).toBe("progress-user-a-file");
    expect(calls(EntityPlaybackSessionUpdateDocument)[0]?.variables.id).not.toBe("invalid-session");
  });

  it("uses explicit book identity for chapter sessions even when the detail href differs", async () => {
    const chapter: PlayItem = { ...item("chapter-file"), entity: { kind: "chapter", id: "chapter-id" }, audiobookId: "book-id", href: "/audiobooks/not-the-identity" };
    const { result, rerender } = renderHook(({ selected }) => useProgressSync(selected), { initialProps: { selected: chapter } });
    act(() => result.current.report(20, 100, true));
    await settle();
    expect(calls(EntityPlaybackSessionCreateDocument)[0]?.variables.input).toMatchObject({ audiobookId: "book-id", mediaFileId: "chapter-file", contentType: "chapter" });
    rerender({ selected: { ...chapter, title: "New metadata" } });
    await act(async () => result.current.flush(true));
    expect(calls(EntityPlaybackSessionCreateDocument)).toHaveLength(1);
    expect(calls(EntityPlaybackSessionUpdateDocument).some(({ variables }) => variables.input.isPlaying === false)).toBe(false);
  });
});
