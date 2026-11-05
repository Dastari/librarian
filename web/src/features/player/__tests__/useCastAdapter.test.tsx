import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useCastAdapter } from "../useCastAdapter";

const mocks = vi.hoisted(() => ({
  rows: [] as Array<Record<string, unknown>>,
  stop: vi.fn(async () => ({ data: { castStop: { success: true, error: null } } })),
  seek: vi.fn(async () => ({ data: { castSeek: { success: true, error: null } } })),
}));

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "me" } }) }));
vi.mock("@/features/settings/useSettingsConnection", () => ({
  useSettingsConnection: (_document: unknown, _variables: unknown, select: (data: Record<string, unknown>) => unknown) => {
    const kind = select({ castSessions: "sessions", castDevices: "devices" });
    return { rows: kind === "sessions" ? mocks.rows : [{ id: "tv", name: "Lounge TV" }], refetch: vi.fn() };
  },
}));
vi.mock("@apollo/client/react", () => ({
  useMutation: (document: { definitions: Array<{ name?: { value: string } }> }) => {
    const name = document.definitions[0]?.name?.value ?? "";
    return [name === "CastStop" ? mocks.stop : name === "CastSeek" ? mocks.seek : vi.fn(async () => ({ data: {} }))];
  },
}));

const session = (overrides = {}) => ({ id: "s1", userId: "me", deviceId: "tv", mediaFileId: "file-1", endedAt: null, playerState: "PLAYING", currentPosition: 42, duration: 100, volume: 0.5, isMuted: false, ...overrides });

describe("useCastAdapter", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.rows = [];
  });

  it("is available with no session, and connecting opens the picker", () => {
    const { result } = renderHook(() => useCastAdapter("file-1"));
    expect(result.current.adapter.getSession().status).toBe("available");
    act(() => void result.current.adapter.connect());
    expect(result.current.pickerOpen).toBe(true);
  });

  it("reads the receiver's state from this file's live session", () => {
    mocks.rows = [session()];
    const { result } = renderHook(() => useCastAdapter("file-1"));
    expect(result.current.adapter.getSession()).toEqual({ status: "connected", deviceName: "Lounge TV", currentTime: 42, duration: 100, paused: false, volume: 0.5, muted: false });
  });

  it("ignores a session for another file or another person", () => {
    mocks.rows = [session({ mediaFileId: "file-2" }), session({ id: "s2", userId: "them" })];
    const { result } = renderHook(() => useCastAdapter("file-1"));
    expect(result.current.adapter.getSession().status).toBe("available");
  });

  it("keeps one session object until something changes, and tells listeners when it does", () => {
    mocks.rows = [session()];
    const { result, rerender } = renderHook(() => useCastAdapter("file-1"));
    const first = result.current.adapter.getSession();
    const listener = vi.fn();
    result.current.adapter.subscribe(listener);
    rerender();
    expect(result.current.adapter.getSession()).toBe(first);
    expect(listener).not.toHaveBeenCalled();
    mocks.rows = [session({ currentPosition: 50 })];
    rerender();
    expect(result.current.adapter.getSession().currentTime).toBe(50);
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("sends commands to the session and stops it on disconnect", async () => {
    mocks.rows = [session()];
    const { result } = renderHook(() => useCastAdapter("file-1"));
    await result.current.adapter.seek(30);
    expect(mocks.seek).toHaveBeenCalledWith({ variables: { sessionId: "s1", position: 30 } });
    await result.current.adapter.disconnect();
    expect(mocks.stop).toHaveBeenCalledWith({ variables: { sessionId: "s1" } });
  });
});
