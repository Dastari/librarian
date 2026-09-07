import { beforeEach, afterEach, expect, it, vi } from "vitest";
import type { AuthSession } from "../../src/lib/auth";
const state = vi.hoisted(() => ({
  setTokens: vi.fn(), clearTokens: vi.fn(), expired: true,
  session: null as AuthSession | null,
}));
vi.mock("../../src/lib/auth", () => ({
  getSession: () => state.session,
  isTokenExpired: () => state.expired,
  setTokens: state.setTokens,
  clearTokens: state.clearTokens,
}));
import { refreshSession, ensureFreshSession } from "../../src/lib/refreshSession";
const user = { id: "reader", username: "reader", role: "member" };
const success = () => Response.json({ data: { refreshToken: { success: true, tokens: { expiresIn: 900 } } } });
beforeEach(() => {
  vi.clearAllMocks();
  state.expired = true;
  state.session = { user, expiresAt: 1 };
  vi.stubGlobal("fetch", vi.fn());
});
afterEach(() => vi.unstubAllGlobals());
it("shares cookie rotation across simultaneous callers without clearing playback's cache", async () => {
  let complete!: (value: Response) => void;
  vi.mocked(fetch).mockReturnValue(new Promise(resolve => { complete = resolve; }));
  const first = refreshSession();
  const second = refreshSession();
  expect(fetch).toHaveBeenCalledOnce();
  complete(success());
  const sessions = await Promise.all([first, second]);
  expect(sessions[0]).toEqual(sessions[1]);
  expect(sessions[0]?.user.id).toBe("reader");
  expect(state.setTokens).toHaveBeenCalledExactlyOnceWith(sessions[0], { refresh: true });
  expect(vi.mocked(fetch).mock.calls[0][1]).toMatchObject({ credentials: "include" });
});
it("uses another tab's renewed session after acquiring the refresh lock", async () => {
  vi.stubGlobal("navigator", { locks: {
    request: async (_name: string, callback: () => unknown) => {
      state.session = { user, expiresAt: Math.floor(Date.now() / 1000) + 900 };
      return callback();
    },
  } });
  expect(await refreshSession()).toEqual(state.session);
  expect(fetch).not.toHaveBeenCalled();
});
it("restores the user from HttpOnly cookies when display metadata is missing", async () => {
  state.session = null;
  vi.mocked(fetch).mockResolvedValueOnce(success()).mockResolvedValueOnce(Response.json({ data: { me: user } }));
  expect((await refreshSession())?.user).toMatchObject(user);
  expect(fetch).toHaveBeenCalledTimes(2);
});
it.each([503, 200])("preserves sessions on temporary HTTP/GraphQL failures (%s) and permits retry", async status => {
  vi.mocked(fetch).mockResolvedValueOnce(Response.json({ errors: [{ message: "Unavailable" }] }, { status }));
  await expect(ensureFreshSession()).rejects.toThrow();
  expect(state.clearTokens).not.toHaveBeenCalled();
  vi.mocked(fetch).mockResolvedValueOnce(success());
  await ensureFreshSession();
  expect(state.setTokens).toHaveBeenCalledOnce();
});
it("clears only an explicitly expired or revoked session", async () => {
  vi.mocked(fetch).mockResolvedValueOnce(Response.json({ data: { refreshToken: { success: false, tokens: null } } }));
  await expect(ensureFreshSession()).rejects.toThrow("Session expired");
  expect(state.clearTokens).toHaveBeenCalledOnce();
});
it("does not rotate a fresh session before every protected request", async () => {
  state.expired = false;
  await ensureFreshSession();
  expect(fetch).not.toHaveBeenCalled();
});
