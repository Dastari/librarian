import { beforeEach, describe, expect, it, vi } from "vitest";

const authMock = vi.hoisted(() => ({
  accessToken: null as string | null,
}));

vi.mock("../../src/lib/auth", () => ({
  getAccessToken: () => authMock.accessToken,
}));

import { authFetch, buildApiUrl } from "../../src/lib/api/authFetch";

describe("authFetch", () => {
  beforeEach(() => {
    authMock.accessToken = null;
    vi.stubGlobal(
      "fetch",
      vi.fn(() => Promise.resolve(new Response("ok", { status: 200 }))),
    );
  });

  it("builds API URLs from relative paths without changing absolute URLs", () => {
    expect(buildApiUrl("/api/health", "http://localhost:3001")).toBe(
      "http://localhost:3001/api/health",
    );
    expect(buildApiUrl("api/health", "http://localhost:3001")).toBe(
      "http://localhost:3001/api/health",
    );
    expect(buildApiUrl("https://example.test/api/health", "http://localhost:3001")).toBe(
      "https://example.test/api/health",
    );
  });

  it("adds bearer auth and include credentials by default", async () => {
    authMock.accessToken = "access-token";

    await authFetch("/api/artwork/1", {
      baseUrl: "http://backend.test",
      method: "POST",
      headers: { "X-Request-Id": "request-1" },
    });

    expect(fetch).toHaveBeenCalledOnce();
    const [url, init] = vi.mocked(fetch).mock.calls[0];
    const headers = (init as RequestInit).headers as Headers;

    expect(url).toBe("http://backend.test/api/artwork/1");
    expect(init).toMatchObject({ credentials: "include", method: "POST" });
    expect(headers.get("Authorization")).toBe("Bearer access-token");
    expect(headers.get("X-Request-Id")).toBe("request-1");
  });

  it("does not override explicit Authorization or add auth when disabled", async () => {
    authMock.accessToken = "access-token";

    await authFetch("/api/media", {
      baseUrl: "http://backend.test",
      headers: { Authorization: "Bearer explicit-token" },
    });
    await authFetch("/api/media", {
      baseUrl: "http://backend.test",
      includeAuth: false,
    });

    const firstHeaders = vi.mocked(fetch).mock.calls[0][1]?.headers as Headers;
    const secondHeaders = vi.mocked(fetch).mock.calls[1][1]?.headers as Headers;

    expect(firstHeaders.get("Authorization")).toBe("Bearer explicit-token");
    expect(secondHeaders.has("Authorization")).toBe(false);
  });
});

