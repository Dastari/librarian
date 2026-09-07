import { beforeEach, describe, expect, it, vi } from "vitest";

import { authFetch, buildApiUrl } from "../../src/lib/api/authFetch";

describe("authFetch", () => {
  beforeEach(() => {
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

  it("includes HttpOnly cookies without synthesizing an auth header", async () => {
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
    expect(headers.has("Authorization")).toBe(false);
    expect(headers.get("X-Request-Id")).toBe("request-1");
  });

  it("preserves explicitly supplied headers", async () => {
    await authFetch("/api/media", {
      baseUrl: "http://backend.test",
      headers: { Authorization: "Bearer explicit-token" },
    });

    const firstHeaders = vi.mocked(fetch).mock.calls[0][1]?.headers as Headers;

    expect(firstHeaders.get("Authorization")).toBe("Bearer explicit-token");
  });
});
