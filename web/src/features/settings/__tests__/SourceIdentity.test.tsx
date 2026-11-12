import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { healthState } from "../sourceHealth";
import { SourceIdentity, sourceIdentityState } from "../SourceIdentity";
import { feedItemActions } from "../FeedItemsDialog";

describe("sourceIdentityState", () => {
  it("carries the health label and tone, and marks a disabled source as paused", () => {
    const failing = healthState({ enabled: true, healthStatus: "failing", healthKind: "unreachable", lastError: "HTTP 503", failures: 3 });
    expect(sourceIdentityState(failing, true)).toEqual({ id: "unreachable", label: "Unreachable ×3", tone: "danger", paused: false });
    const off = healthState({ enabled: false });
    expect(sourceIdentityState(off, false)).toMatchObject({ id: "disabled", tone: "neutral", paused: true });
  });
});

describe("SourceIdentity", () => {
  it("says why a failing source fails on its second line, and what it is otherwise", () => {
    const failing = healthState({ enabled: true, healthStatus: "failing", healthKind: "unreachable", lastError: "HTTP 503", failures: 1 }, Date.parse("2026-11-08T12:00:00Z"));
    const { rerender } = render(<SourceIdentity kind="feed" name="TV feed" health={failing} enabled facts="every 30 min" />);
    expect(screen.getByText(/HTTP 503 · never worked yet/)).toBeInTheDocument();
    expect(screen.queryByText("every 30 min")).not.toBeInTheDocument();
    rerender(<SourceIdentity kind="feed" name="TV feed" health={healthState({ enabled: true, healthStatus: "ok" })} enabled facts="every 30 min" />);
    expect(screen.getByText("every 30 min")).toBeInTheDocument();
  });
});

describe("feedItemActions", () => {
  it("offers copying titles for one or several listed releases", () => {
    const [copy] = feedItemActions.actions;
    const one = [{ title: "A" }] as never[];
    expect(typeof copy!.label === "function" ? copy!.label(one) : copy!.label).toBe("Copy release title");
    expect(typeof copy!.label === "function" ? copy!.label([...one, ...one]) : copy!.label).toBe("Copy release titles");
    expect(copy!.bulk).toBe(true);
  });
});
