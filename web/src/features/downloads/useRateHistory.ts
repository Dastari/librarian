import { useEffect, useRef, useState } from "react";

/** Samples kept per series: two minutes at one sample every two seconds. */
const SAMPLES = 60;
const INTERVAL_MS = 2000;

export interface RateSample {
  down: number;
  up: number;
}

export interface RateHistory {
  /** Per torrent (by info hash), oldest first. */
  byHash: ReadonlyMap<string, { down: readonly number[]; up: readonly number[] }>;
  total: { down: readonly number[]; up: readonly number[] };
}

const EMPTY: RateHistory = { byHash: new Map(), total: { down: [], up: [] } };

const push = (series: readonly number[] | undefined, value: number) => {
  const next = [...(series ?? []), value];
  return next.length > SAMPLES ? next.slice(next.length - SAMPLES) : next;
};

/**
 * Recent transfer rates for the sparklines, sampled from the live client state while the page
 * is open. The server keeps no rate history, so the series start when the page opens; reading
 * `current` on a timer (not on every progress event) keeps the samples evenly spaced.
 */
export function useRateHistory(current: ReadonlyMap<string, RateSample>): RateHistory {
  const latest = useRef(current);
  latest.current = current;
  const [history, setHistory] = useState<RateHistory>(EMPTY);

  useEffect(() => {
    const sample = () => {
      setHistory((previous) => {
        const byHash = new Map<string, { down: readonly number[]; up: readonly number[] }>();
        let down = 0;
        let up = 0;
        for (const [hash, rate] of latest.current) {
          const series = previous.byHash.get(hash);
          byHash.set(hash, { down: push(series?.down, rate.down), up: push(series?.up, rate.up) });
          down += rate.down;
          up += rate.up;
        }
        return { byHash, total: { down: push(previous.total.down, down), up: push(previous.total.up, up) } };
      });
    };
    sample();
    const timer = window.setInterval(sample, INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, []);

  return history;
}
