// @vitest-environment node
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LiveRegistry, type LiveEvent, type LiveSource } from "../registry";
import type { LiveTopic } from "../documents";

function fixture() {
  const events = new Map<LiveTopic, (event: LiveEvent) => void>();
  const interrupted = new Map<LiveTopic, () => void>();
  const stopped = vi.fn();
  const source: LiveSource = vi.fn((topic, emit, interrupt) => {
    events.set(topic, emit); interrupted.set(topic, interrupt);
    return () => { stopped(topic); events.delete(topic); };
  });
  const registry = new LiveRegistry(source);
  registry.reset(true);
  return { registry, source, events, interrupted, stopped };
}
const deferred = () => {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => { resolve = done; });
  return { promise, resolve };
};
beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe("active live registry", () => {
  it("shares streams and one task across three shell consumers", async () => {
    const { registry, source, events, stopped } = fixture();
    const callbacks = [vi.fn(), vi.fn(), vi.fn()];
    const dispose = callbacks.map((refresh) => registry.register({ key: "count", dependencies: [{ topic: "TorrentProgress" }], refresh }));
    expect(source).toHaveBeenCalledTimes(1);
    for (let i = 0; i < 500; i++) events.get("TorrentProgress")!({ topic: "TorrentProgress", id: "one" });
    await vi.advanceTimersByTimeAsync(200);
    expect(callbacks.reduce((sum, callback) => sum + callback.mock.calls.length, 0)).toBe(1);
    dispose[0]!(); dispose[1]!(); expect(stopped).not.toHaveBeenCalled();
    dispose[2]!(); expect(stopped).toHaveBeenCalledTimes(1);
    registry.reset(false);
  });

  it("shares the member capability listener only while active and fences the old principal", async () => {
    const { registry, source, events, stopped } = fixture();
    const first = vi.fn(); const second = vi.fn();
    const registration = { key: "HasEnabledSources", dependencies: [{ topic: "EnabledSources" as const }] };
    const disposeFirst = registry.register({ ...registration, refresh: first });
    const disposeSecond = registry.register({ ...registration, refresh: second });
    expect(source).toHaveBeenCalledTimes(1);
    const oldEvent = events.get("EnabledSources")!;
    disposeFirst();
    expect(stopped).not.toHaveBeenCalled();
    oldEvent({ topic: "EnabledSources" });
    await vi.advanceTimersByTimeAsync(150);
    expect(second).toHaveBeenCalledOnce();
    disposeSecond();
    expect(stopped).toHaveBeenCalledOnce();
    registry.reset(true);
    const current = vi.fn();
    registry.register({ ...registration, refresh: current });
    oldEvent({ topic: "EnabledSources" });
    await vi.advanceTimersByTimeAsync(1000);
    expect(current).not.toHaveBeenCalled();
    events.get("EnabledSources")!({ topic: "EnabledSources" });
    await vi.advanceTimersByTimeAsync(150);
    expect(current).toHaveBeenCalledOnce();
    registry.reset(false);
  });

  it("refreshes every independent local snapshot even when its document and variables match", async () => {
    const { registry, source, events } = fixture();
    const first = vi.fn(); const second = vi.fn();
    registry.register({ key: "same-document", shared: false, dependencies: [{ topic: "Movie" }], refresh: first });
    registry.register({ key: "same-document", shared: false, dependencies: [{ topic: "Movie" }], refresh: second });
    expect(source).toHaveBeenCalledTimes(1);
    events.get("Movie")!({ topic: "Movie" });
    await vi.advanceTimersByTimeAsync(150);
    expect(first).toHaveBeenCalledOnce(); expect(second).toHaveBeenCalledOnce();
    registry.reset(false);
  });

  it("retains exactly one dirty follow-up when events arrive during a busy refresh", async () => {
    const { registry, events } = fixture();
    const pending = deferred();
    const refresh = vi.fn().mockReturnValueOnce(pending.promise).mockResolvedValue(undefined);
    registry.register({ key: "complete", dependencies: [{ topic: "Episode" }], refresh });
    events.get("Episode")!({ topic: "Episode" });
    await vi.advanceTimersByTimeAsync(150);
    for (let i = 0; i < 500; i++) events.get("Episode")!({ topic: "Episode" });
    await vi.advanceTimersByTimeAsync(100);
    expect(refresh).toHaveBeenCalledTimes(1);
    pending.resolve();
    await vi.advanceTimersByTimeAsync(1000);
    expect(refresh).toHaveBeenCalledTimes(2);
    registry.reset(false);
  });

  it("scopes explicit IDs while empty list membership still sees CREATE", async () => {
    const { registry, events } = fixture();
    const one = vi.fn(); const two = vi.fn(); const empty = vi.fn();
    registry.register({ key: "one", dependencies: [{ topic: "Movie", ids: ["one"] }], refresh: one });
    registry.register({ key: "two", dependencies: [{ topic: "Movie", ids: ["two"] }], refresh: two });
    registry.register({ key: "empty-filtered-list", dependencies: [{ topic: "Movie" }], refresh: empty });
    events.get("Movie")!({ topic: "Movie", id: "two" });
    await vi.advanceTimersByTimeAsync(150);
    expect(one).not.toHaveBeenCalled(); expect(two).toHaveBeenCalledOnce(); expect(empty).toHaveBeenCalledOnce();
    registry.reset(false);
  });

  it("resnapshots once on reconnect and subscription completion/lag", async () => {
    const { registry, interrupted } = fixture();
    const refresh = vi.fn();
    registry.register({ key: "snapshot", dependencies: [{ topic: "Track" }], refresh });
    registry.transport(true); registry.transport(true);
    await vi.advanceTimersByTimeAsync(150);
    expect(refresh).toHaveBeenCalledTimes(1);
    registry.transport(false); registry.transport(true);
    await vi.advanceTimersByTimeAsync(1000);
    expect(refresh).toHaveBeenCalledTimes(2);
    interrupted.get("Track")!();
    await vi.advanceTimersByTimeAsync(1000);
    expect(refresh).toHaveBeenCalledTimes(3);
    registry.reset(false);
  });

  it("discards old principal callbacks, timers and late source events", async () => {
    const { registry, events } = fixture();
    const old = vi.fn();
    registry.register({ key: "old", dependencies: [{ topic: "Movie" }], refresh: old });
    const oldEvent = events.get("Movie")!;
    oldEvent({ topic: "Movie" });
    registry.reset(true);
    const current = vi.fn();
    registry.register({ key: "new", dependencies: [{ topic: "Movie" }], refresh: current });
    oldEvent({ topic: "Movie" });
    await vi.advanceTimersByTimeAsync(1000);
    expect(old).not.toHaveBeenCalled(); expect(current).not.toHaveBeenCalled();
    events.get("Movie")!({ topic: "Movie" });
    await vi.advanceTimersByTimeAsync(150);
    expect(current).toHaveBeenCalledOnce();
    registry.reset(false);
  });

  it("retries failed snapshots with bounded backoff and caps simultaneous work", async () => {
    const { registry, events } = fixture();
    let active = 0; let maximum = 0;
    const wait = deferred();
    for (let i = 0; i < 20; i++) registry.register({ key: `view-${i}`, dependencies: [{ topic: "Movie" }], refresh: async () => { active++; maximum = Math.max(maximum, active); await wait.promise; active--; } });
    events.get("Movie")!({ topic: "Movie" });
    await vi.advanceTimersByTimeAsync(150);
    expect(maximum).toBe(4);
    wait.resolve(); await vi.advanceTimersByTimeAsync(5000);
    expect(maximum).toBe(4);
    registry.reset(true);
    const failing = vi.fn().mockRejectedValue(new Error("offline"));
    registry.register({ key: "retry", dependencies: [{ topic: "Movie" }], refresh: failing });
    events.get("Movie")!({ topic: "Movie" });
    await vi.advanceTimersByTimeAsync(120_000);
    expect(failing).toHaveBeenCalledTimes(5);
    registry.reset(false);
  });
});
