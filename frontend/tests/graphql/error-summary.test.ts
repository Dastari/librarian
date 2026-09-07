import { describe, expect, it } from "vitest";

import { summarizeGraphQLErrors } from "../../src/lib/graphql/errors";

describe("GraphQL error notifications", () => {
  it("collapses schema drift into one actionable error", () => {
    expect(
      summarizeGraphQLErrors("CastDevices", [
        'Unknown field "enabled" on type "CastDevice".',
        'Unknown field "playbackSupported" on type "CastDevice".',
        'Unknown field "discoveryOrigin" on type "CastDevice".',
      ]),
    ).toBe(
      'Frontend/backend GraphQL schema mismatch in CastDevices. Unknown field "enabled" on type "CastDevice". Unknown field "playbackSupported" on type "CastDevice". Unknown field "discoveryOrigin" on type "CastDevice". Rebuild and restart the backend, then reload the frontend.',
    );
  });

  it("deduplicates repeated errors without rewriting a single message", () => {
    expect(
      summarizeGraphQLErrors("Login", [
        "Login service unavailable",
        "Login service unavailable",
      ]),
    ).toBe("Login service unavailable");
  });
});
