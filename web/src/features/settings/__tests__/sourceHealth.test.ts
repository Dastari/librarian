import { describe, expect, it } from "vitest";

import { healthState, lastWorked } from "../sourceHealth";

const NOW = Date.parse("2026-11-08T12:00:00Z");

describe("healthState", () => {
  it("shows a switched-off source as disabled whatever its last error", () => {
    expect(healthState({ enabled: false, healthStatus: "failing", lastError: "boom" }, NOW)).toMatchObject({ label: "Disabled", tone: "neutral", status: "offline" });
  });

  it("shows a working source as working with when it was checked", () => {
    const state = healthState({ enabled: true, healthStatus: "ok", lastCheckedAt: "2026-11-08T11:30:00Z" }, NOW);
    expect(state).toMatchObject({ label: "Working", tone: "success", status: "online" });
    expect(state.detail).toMatch(/Checked 30 minutes ago/);
  });

  it("shows an expired sign-in in red with the reason and when it last worked", () => {
    const state = healthState({ enabled: true, healthStatus: "auth_expired", healthKind: "auth", lastError: "IPTorrents can't sign in: CAPTCHA", lastSuccessAt: "2026-11-07T12:00:00Z", failures: 3 }, NOW);
    expect(state).toMatchObject({ label: "Sign-in expired ×3", tone: "danger", status: "error" });
    expect(state.detail).toBe("IPTorrents can't sign in: CAPTCHA · last worked yesterday");
  });

  it("names the other failures, and treats being rate limited as a wait, not a fault", () => {
    expect(healthState({ enabled: true, healthStatus: "failing", healthKind: "unreachable", lastError: "timed out" }, NOW).label).toBe("Unreachable");
    expect(healthState({ enabled: true, healthStatus: "failing", healthKind: "rate_limited" }, NOW)).toMatchObject({ label: "Rate limited", tone: "warning", status: "error" });
    expect(healthState({ enabled: true, healthStatus: "failing", healthKind: "other" }, NOW).label).toBe("Failing");
  });

  it("still reads a source that predates the status field by its last error", () => {
    expect(healthState({ enabled: true, healthStatus: null, lastError: "HTTP 503", failures: 2 }, NOW)).toMatchObject({ label: "Failing ×2", status: "error" });
    expect(healthState({ enabled: true, healthStatus: null, lastError: null }, NOW)).toMatchObject({ label: "Not checked yet", status: "online" });
  });
});

describe("lastWorked", () => {
  it("says never when it has not", () => {
    expect(lastWorked(null, NOW)).toBe("never worked yet");
    expect(lastWorked("not a time", NOW)).toBe("never worked yet");
  });
});
