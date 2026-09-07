// @vitest-environment jsdom
import { beforeEach, afterEach, expect, it, vi } from "vitest";
const state = vi.hoisted(() => ({ expiresAt: 900 }));
vi.mock("../../src/lib/auth", () => ({ getSession: () => state }));
import { startSessionRenewal } from "../../src/lib/sessionRenewal";
let stop: (() => void) | undefined;
beforeEach(() => { vi.useFakeTimers(); vi.setSystemTime(0); state.expiresAt = 900; });
afterEach(() => { stop?.(); vi.useRealTimers(); });
it("keeps renewing throughout an hour of playback, before cookies expire", async () => {
  const refresh = vi.fn(async () => {
    expect(Date.now() / 1000).toBeLessThan(state.expiresAt);
    state.expiresAt = Date.now() / 1000 + 900;
    return true;
  });
  stop = startSessionRenewal(refresh);
  await vi.advanceTimersByTimeAsync(60 * 60 * 1000);
  expect(refresh).toHaveBeenCalledTimes(4);
  expect(state.expiresAt).toBeGreaterThan(Date.now() / 1000);
});
it("renews immediately on wake and reconnection, retrying a temporary failure", async () => {
  const refresh = vi.fn().mockResolvedValueOnce(false).mockImplementation(async () => {
    state.expiresAt = Date.now() / 1000 + 900;
    return true;
  });
  stop = startSessionRenewal(refresh);
  vi.setSystemTime(29 * 86400 * 1000);
  window.dispatchEvent(new Event("focus"));
  await Promise.resolve();
  window.dispatchEvent(new Event("online"));
  await Promise.resolve();
  expect(refresh).toHaveBeenCalledTimes(2);
  expect(state.expiresAt).toBeGreaterThan(Date.now() / 1000);
  stop();
  state.expiresAt = 0;
  window.dispatchEvent(new Event("focus"));
  await vi.advanceTimersByTimeAsync(60000);
  expect(refresh).toHaveBeenCalledTimes(2);
});
