// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { getSession, isTokenExpired } from "../../src/lib/auth";
afterEach(() => {
  document.cookie = "librarian_user=; Max-Age=0; Path=/";
  vi.useRealTimers();
});
it("retains user metadata for refresh when the access expiry cookie has disappeared", () => {
  const user = { id: "one", username: "reader", role: "member" };
  document.cookie = `librarian_user=${encodeURIComponent(JSON.stringify(user))}; Path=/`;
  expect(getSession()).toEqual({ user, expiresAt: 0 });
  expect(isTokenExpired()).toBe(true);
});

vi.mock("../../src/lib/graphql/client", () => ({ resetApolloCache: vi.fn(), restartWebSocket: vi.fn() }));
import { setTokens } from "../../src/lib/auth";
import { resetApolloCache, restartWebSocket } from "../../src/lib/graphql/client";
it("retains display metadata for 30 days and preserves cached content during refresh", async () => {
  const writes = vi.spyOn(document, "cookie", "set");
  const now = Date.now();
  setTokens({ expiresAt: Math.floor(now / 1000) + 900, user: { id: "one", username: "reader", role: "member" } }, { refresh: true });
  for (const [cookie] of writes.mock.calls) {
    const expiry = /expires=([^;]+)/.exec(cookie)?.[1];
    expect(expiry).toBeDefined();
    expect(Math.abs(Date.parse(expiry!) - now - 30 * 86400 * 1000)).toBeLessThan(1000);
  }
  expect(writes).toHaveBeenCalledTimes(2);
  await vi.waitFor(() => expect(restartWebSocket).toHaveBeenCalled());
  expect(resetApolloCache).not.toHaveBeenCalled();
  writes.mockRestore();
});
