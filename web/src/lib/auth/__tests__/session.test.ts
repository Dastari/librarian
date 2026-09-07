import { CombinedGraphQLErrors } from "@apollo/client";
import { GraphQLError } from "graphql";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { LogoutDocument, RefreshSessionDocument } from "@/graphql/generated/graphql";

import type { SessionUser } from "../session";

const user = { id: "u1", username: "toby", email: "toby@example.com", role: "admin" } as unknown as SessionUser;

interface FakeClient {
  query: ReturnType<typeof vi.fn>;
  mutate: ReturnType<typeof vi.fn>;
  clearStore: ReturnType<typeof vi.fn>;
}

/** `vi.resetModules()` gives each test a fresh copy of every module, so match on the operation name rather than document identity. */
const operationName = (document: unknown): string =>
  (document as { definitions: Array<{ name?: { value: string } }> }).definitions[0]?.name?.value ?? "";

interface FakeClientOptions {
  refresh?: () => unknown;
  me?: () => unknown;
  logout?: () => unknown;
}

/** Fresh module instance per test: the session store is a singleton. */
async function freshSession(options: FakeClientOptions = {}) {
  vi.resetModules();
  const { session } = await import("../session");
  const client: FakeClient = {
    query: vi.fn(async ({ query }: { query: unknown }) => {
      expect(operationName(query)).toBe("CurrentUser");
      return (options.me ?? (() => ({ data: { me: user } })))();
    }),
    mutate: vi.fn(async ({ mutation }: { mutation: unknown }) => {
      const name = operationName(mutation);
      if (name === "RefreshSession") return (options.refresh ?? (() => ({ data: { refreshToken: { success: true, tokens: { expiresIn: 900 } } } })))();
      if (name === "Logout") return (options.logout ?? (() => ({ data: { logout: { success: true } } })))();
      throw new Error(`unexpected mutation ${name}`);
    }),
    clearStore: vi.fn(async () => []),
  };
  session.attach(client as never);
  return { session, client };
}

const unauthorized = () => {
  throw new CombinedGraphQLErrors({ errors: [new GraphQLError("nope", { extensions: { code: "UNAUTHORIZED" } })] });
};

beforeEach(() => {
  vi.stubGlobal("navigator", Object.assign(Object.create(Object.getPrototypeOf(navigator)), navigator, { locks: undefined }));
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

describe("session boot", () => {
  it("rebuilds an authenticated session from the cookies", async () => {
    const { session, client } = await freshSession();
    expect(session.getSnapshot().status).toBe("booting");
    await session.boot();
    const state = session.getSnapshot();
    expect(state.status).toBe("authenticated");
    expect(state.user).toEqual(user);
    expect(state.expiresAt).toBeGreaterThan(Date.now());
    expect(client.mutate).toHaveBeenCalledTimes(1);
    expect(client.query).toHaveBeenCalledTimes(1);
  });

  it("settles as anonymous when the server has no session", async () => {
    const { session, client } = await freshSession({ refresh: () => ({ data: { refreshToken: { success: false, tokens: null } } }) });
    await session.boot();
    expect(session.getSnapshot()).toEqual({ status: "anonymous", user: null, expiresAt: null });
    expect(client.query).not.toHaveBeenCalled();
  });

  it("boots once however many callers await it", async () => {
    const { session, client } = await freshSession();
    await Promise.all([session.boot(), session.ready(), session.boot()]);
    expect(client.mutate).toHaveBeenCalledTimes(1);
  });

  it("stays anonymous when the user query fails during boot", async () => {
    const { session } = await freshSession({
      me: () => {
        throw new Error("network");
      },
    });
    await session.boot();
    expect(session.getSnapshot().status).toBe("anonymous");
  });

  it("notifies subscribers on every state change", async () => {
    const { session } = await freshSession();
    const listener = vi.fn();
    const unsubscribe = session.subscribe(listener);
    await session.boot();
    expect(listener).toHaveBeenCalled();
    unsubscribe();
    const before = listener.mock.calls.length;
    session.establish(user, 900);
    expect(listener.mock.calls.length).toBe(before);
  });
});

describe("session renewal", () => {
  it("skips the network while the credential is comfortably fresh", async () => {
    const { session, client } = await freshSession();
    session.establish(user, 900);
    client.mutate.mockClear();
    await expect(session.ensureFresh()).resolves.toBe(true);
    expect(client.mutate).not.toHaveBeenCalled();
  });

  it("renews when the credential is near expiry", async () => {
    const { session, client } = await freshSession();
    session.establish(user, 60);
    client.mutate.mockClear();
    await expect(session.ensureFresh()).resolves.toBe(true);
    expect(client.mutate).toHaveBeenCalledWith(expect.objectContaining({ mutation: RefreshSessionDocument }));
    expect(session.getSnapshot().expiresAt).toBeGreaterThan(Date.now() + 800_000);
  });

  it("collapses concurrent renewals into one request", async () => {
    const { session, client } = await freshSession();
    await Promise.all([session.refresh(), session.refresh(), session.refresh()]);
    expect(client.mutate).toHaveBeenCalledTimes(1);
  });

  it("renews early, before the access window closes", async () => {
    vi.useFakeTimers();
    const { session, client } = await freshSession();
    session.establish(user, 900);
    client.mutate.mockClear();
    // The timer is set for 90s before expiry.
    await vi.advanceTimersByTimeAsync(809_000);
    expect(client.mutate).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(2_000);
    expect(client.mutate).toHaveBeenCalledTimes(1);
  });

  it("keeps the session and retries later when the network is down", async () => {
    vi.useFakeTimers();
    const { session, client } = await freshSession({
      refresh: () => {
        throw new Error("Failed to fetch");
      },
    });
    session.establish(user, 900);
    client.mutate.mockClear();
    await expect(session.refresh()).resolves.toBe(true);
    expect(session.getSnapshot().status).toBe("authenticated");
    await vi.advanceTimersByTimeAsync(31_000);
    expect(client.mutate).toHaveBeenCalledTimes(2);
  });

  it("ends the session when the server says the refresh cookie is gone", async () => {
    const { session, client } = await freshSession({ refresh: unauthorized });
    const changes: boolean[] = [];
    session.onChange((authenticated) => changes.push(authenticated));
    session.establish(user, 900);
    await expect(session.refresh()).resolves.toBe(false);
    expect(session.getSnapshot()).toEqual({ status: "anonymous", user: null, expiresAt: null });
    expect(changes).toEqual([true, false]);
    expect(client.clearStore).toHaveBeenCalled();
  });

  it("renews on focus only when a session is live", async () => {
    const { session, client } = await freshSession();
    window.dispatchEvent(new Event("focus"));
    expect(client.mutate).not.toHaveBeenCalled();
    session.establish(user, 30);
    client.mutate.mockClear();
    window.dispatchEvent(new Event("focus"));
    await vi.waitFor(() => expect(client.mutate).toHaveBeenCalled());
  });

  it("serialises renewal across tabs with the Web Locks API", async () => {
    const request = vi.fn(async (_name: string, task: () => Promise<unknown>) => task());
    vi.stubGlobal("navigator", Object.assign(Object.create(Object.getPrototypeOf(navigator)), navigator, { locks: { request } }));
    const { session } = await freshSession();
    await session.refresh();
    expect(request).toHaveBeenCalledWith("librarian.session.refresh", expect.any(Function));
  });
});

describe("session logout", () => {
  it("tells the server, clears the cache and goes anonymous", async () => {
    const { session, client } = await freshSession();
    const changes: boolean[] = [];
    session.onChange((authenticated) => changes.push(authenticated));
    session.establish(user, 900);
    await session.logout();
    expect(client.mutate).toHaveBeenCalledWith({ mutation: LogoutDocument });
    expect(session.getSnapshot()).toEqual({ status: "anonymous", user: null, expiresAt: null });
    expect(changes).toEqual([true, false]);
    expect(client.clearStore).toHaveBeenCalled();
  });

  it("still ends the local session when the logout call fails", async () => {
    const { session } = await freshSession({
      logout: () => {
        throw new Error("Failed to fetch");
      },
    });
    session.establish(user, 900);
    await session.logout();
    expect(session.getSnapshot().status).toBe("anonymous");
  });

  it("stops renewing after logout", async () => {
    vi.useFakeTimers();
    const { session, client } = await freshSession();
    session.establish(user, 900);
    await session.logout();
    client.mutate.mockClear();
    await vi.advanceTimersByTimeAsync(900_000);
    expect(client.mutate).not.toHaveBeenCalled();
  });
});
