import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { formatScrubDelta, SCRUB_COMMIT_MS, Scrubber, scrubStep } from "../scrub";

describe("scrub steps", () => {
  it("starts at 10 s, then 30 s, then a minute or two for long items", () => {
    expect([0, 1, 2].map((streak) => scrubStep(streak, 1800))).toEqual([10, 10, 10]);
    expect([3, 7].map((streak) => scrubStep(streak, 1800))).toEqual([30, 30]);
    expect(scrubStep(8, 1800)).toBe(60);
    expect(scrubStep(8, 7200)).toBe(120);
  });

  it("writes the delta with a sign", () => {
    expect(formatScrubDelta(90)).toBe("+1:30");
    expect(formatScrubDelta(-20)).toBe("−0:20");
    expect(formatScrubDelta(3720)).toBe("+1:02:00");
  });
});

describe("Scrubber", () => {
  let now = 0;
  let position = 600;
  const commit = vi.fn();
  const onChange = vi.fn();
  const make = (duration = 7200) => new Scrubber({ position: () => position, duration: () => duration, commit, onChange, now: () => now });

  beforeEach(() => {
    vi.useFakeTimers();
    now = 0;
    position = 600;
    commit.mockReset();
    onChange.mockReset();
  });
  afterEach(() => vi.useRealTimers());

  it("moves a target, not the playhead, and seeks once after a pause", () => {
    const scrubber = make();
    scrubber.step(1);
    now += 200;
    scrubber.step(1);
    expect(scrubber.current).toEqual({ origin: 600, target: 620, duration: 7200 });
    expect(commit).not.toHaveBeenCalled();
    vi.advanceTimersByTime(SCRUB_COMMIT_MS - 1);
    expect(commit).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(commit).toHaveBeenCalledTimes(1);
    expect(commit).toHaveBeenCalledWith(620);
    expect(scrubber.active).toBe(false);
    expect(onChange).toHaveBeenLastCalledWith(null);
  });

  it("accelerates while presses keep coming, and a lone tap is 10 s", () => {
    const scrubber = make();
    const targets: number[] = [];
    for (let press = 0; press < 10; press += 1) {
      scrubber.step(1, press > 0);
      targets.push(scrubber.current!.target);
      now += 200;
    }
    // 10, 10, 10, then 30 × 5, then 120 (over an hour) × 2.
    expect(targets).toEqual([610, 620, 630, 660, 690, 720, 750, 780, 900, 1020]);
    scrubber.cancel();
    now += 5000;
    scrubber.step(1);
    expect(scrubber.current!.target).toBe(610);
  });

  it("throttles a held key's auto-repeat", () => {
    const scrubber = make();
    scrubber.step(1);
    now += 30;
    scrubber.step(1, true);
    now += 30;
    scrubber.step(1, true);
    expect(scrubber.current!.target).toBe(610);
    now += 150;
    scrubber.step(1, true);
    expect(scrubber.current!.target).toBe(620);
  });

  it("commits at once on OK, and cancel leaves playback where it was", () => {
    const scrubber = make();
    scrubber.step(-1);
    scrubber.commit();
    expect(commit).toHaveBeenCalledWith(590);
    commit.mockReset();
    scrubber.step(1);
    scrubber.cancel();
    vi.advanceTimersByTime(5000);
    expect(commit).not.toHaveBeenCalled();
  });

  it("stays inside the item and does nothing before it has a length", () => {
    position = 5;
    const scrubber = make(60);
    scrubber.step(-1);
    expect(scrubber.current!.target).toBe(0);
    scrubber.cancel();
    expect(make(0).step(1)).toBe(false);
  });
});
