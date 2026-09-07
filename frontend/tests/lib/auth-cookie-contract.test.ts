import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const frontendRoot = resolve(__dirname, "../..");

function source(path: string): string {
  return readFileSync(resolve(frontendRoot, path), "utf8");
}

describe("browser cookie-auth contract", () => {
  it("does not expose credentials through auth documents or browser state", () => {
    const documents = source("src/lib/graphql/documents/auth.graphql");
    const auth = source("src/lib/auth.ts");

    expect(documents).not.toContain("accessToken");
    expect(documents).not.toContain("refreshToken:");
    expect(documents).toContain("mutation RefreshToken {");
    expect(documents).toContain("mutation Logout {");
    expect(auth).not.toContain("getAccessToken");
    expect(auth).not.toContain("getRefreshToken");
    expect(auth).not.toContain("librarian_access_token");
    expect(auth).not.toContain("librarian_refresh_token");
  });

  it("uses cookies rather than auth headers, websocket params, or media query tokens", () => {
    const client = source("src/lib/graphql/client.ts");
    const video = source("src/components/VideoPlayer.tsx");
    const authFetch = source("src/lib/api/authFetch.ts");

    expect(client).toContain('credentials: "include"');
    expect(client).not.toContain("setContext");
    expect(client).not.toContain("connectionParams");
    expect(client).not.toContain("authorization: token");
    expect(authFetch).not.toContain("Authorization");
    expect(video).not.toContain("?token=");
    expect(video).not.toContain("getAccessToken");
  });
});
