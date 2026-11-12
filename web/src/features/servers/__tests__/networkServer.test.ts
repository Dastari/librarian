import { describe, expect, it } from "vitest";

import { serverFor } from "../StorageLocationField";
import { networkServerState, serverAddress, signInLabel } from "../NetworkServerIdentity";
import { testSummary } from "../useNetworkServerActions";

const server = (extra: Record<string, unknown> = {}) => ({
  id: "s1", name: "TrueNAS", host: "10.0.1.190", port: null as number | null, username: "toby", hasPassword: true, knownShares: ["media"],
  status: null as string | null, lastError: null, lastCheckedAt: null, createdAt: "", updatedAt: "", address: "smb://10.0.1.190", __typename: "NetworkServer" as const, ...extra,
});

describe("a saved file server", () => {
  it("says whether it answered the last time it was tried", () => {
    expect(networkServerState(server()).label).toBe("Not tested yet");
    expect(networkServerState(server({ status: "ok" })).tone).toBe("success");
    expect(networkServerState(server({ status: "failing" })).tone).toBe("danger");
  });

  it("names its address and who it signs in as", () => {
    expect(serverAddress(server())).toBe("10.0.1.190");
    expect(serverAddress(server({ port: 4455 }))).toBe("10.0.1.190:4455");
    expect(signInLabel(server())).toBe("Signs in as toby");
    expect(signInLabel(server({ username: null }))).toBe("Guest sign-in");
  });

  it("is found from the host of a location, port included", () => {
    const servers = [server(), server({ id: "s2", host: "nas", port: 4455 })];
    expect(serverFor(servers, "10.0.1.190")?.id).toBe("s1");
    expect(serverFor(servers, "10.0.1.190:445")?.id).toBe("s1");
    expect(serverFor(servers, "NAS:4455")?.id).toBe("s2");
    expect(serverFor(servers, "nas")).toBeNull();
  });

  it("sums up a test in one line", () => {
    expect(testSummary({ success: true, shares: [{ listed: true }, { listed: false }] })).toBe("Signed in. 1 share offered.");
    expect(testSummary({ success: true, shares: [], sharesError: "nas signed you in but would not list its shares" })).toMatch(/would not list/);
    expect(testSummary({ success: false, error: "The server refused the sign-in for 'toby'", shares: [] })).toMatch(/refused/);
  });
});
