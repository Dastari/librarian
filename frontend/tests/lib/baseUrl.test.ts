import { afterEach, describe, expect, it, vi } from "vitest";

afterEach(() => {
  vi.unstubAllEnvs();
  vi.resetModules();
});

describe("browser API origin", () => {
  it.each([undefined, "", "  "])(
    "uses the live origin when the API override is %j",
    async (value) => {
      vi.stubEnv("VITE_API_URL", value);
      vi.resetModules();
      const { API_BASE_URL, graphqlWebSocketUrl } =
        await import("../../src/lib/api/baseUrl");
      const { buildApiUrl } = await import("../../src/lib/api/authFetch");

      expect(API_BASE_URL).toBe("");
      expect(
        new URL(buildApiUrl("/api/healthz"), "https://librarian.dastari.net")
          .href,
      ).toBe("https://librarian.dastari.net/api/healthz");
      expect(
        graphqlWebSocketUrl(undefined, "https://librarian.dastari.net"),
      ).toBe("wss://librarian.dastari.net/graphql/ws");
    },
  );

  it("keeps local browser requests on Vite's proxied origin", async () => {
    const { graphqlWebSocketUrl } = await import("../../src/lib/api/baseUrl");
    expect(graphqlWebSocketUrl("", "http://localhost:3000")).toBe(
      "ws://localhost:3000/graphql/ws",
    );
  });

  it("respects an explicit backend override and normalizes its trailing slash", async () => {
    vi.stubEnv("VITE_API_URL", "https://api.example.test/");
    vi.resetModules();
    const { API_BASE_URL, graphqlWebSocketUrl } =
      await import("../../src/lib/api/baseUrl");
    expect(API_BASE_URL).toBe("https://api.example.test");
    expect(
      graphqlWebSocketUrl(undefined, "https://librarian.dastari.net"),
    ).toBe("wss://api.example.test/graphql/ws");
  });
});
