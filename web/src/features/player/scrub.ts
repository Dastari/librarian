/**
 * Scrubbing with a remote or keyboard (design.md Q125): Left/Right (or Rewind/Fast-forward) move a
 * target, not the playhead, accelerating while presses keep coming; the player seeks once, on OK or
 * after a short pause. Back puts the target back where it started. Over HLS that is one seek, not
 * one per key press.
 */

/** Seeks this long after the last press. */
export const SCRUB_COMMIT_MS = 700;
/** Presses closer together than this (or held-key repeats) build up speed. */
export const SCRUB_STREAK_MS = 500;
/** Held-key repeats move the target at most this often. */
export const SCRUB_REPEAT_MS = 150;

/**
 * Seconds per press after `streak` presses in a row: 10 s at first, then 30 s, then a minute (two
 * for anything over an hour). A single tap is always 10 s.
 */
export function scrubStep(streak: number, duration: number): number {
  if (streak < 3) return 10;
  if (streak < 8) return 30;
  return duration >= 3600 ? 120 : 60;
}

/** "+1:30", "−0:20", "+1:02:00". */
export function formatScrubDelta(seconds: number): string {
  const sign = seconds < 0 ? "−" : "+";
  const total = Math.round(Math.abs(seconds));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return h > 0 ? `${sign}${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${sign}${m}:${String(s).padStart(2, "0")}`;
}

export interface ScrubState {
  /** Where playback was when scrubbing began. */
  origin: number;
  /** Where it will seek to. */
  target: number;
  duration: number;
}

export interface ScrubberOptions {
  position: () => number;
  duration: () => number;
  /** Seek, once. */
  commit: (target: number) => void;
  /** The scrub as it changes; `null` when it ends either way. */
  onChange?: (state: ScrubState | null) => void;
  now?: () => number;
  commitDelay?: number;
}

export class Scrubber {
  private state: ScrubState | null = null;
  private streak = 0;
  private lastPress = Number.NEGATIVE_INFINITY;
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor(private readonly options: ScrubberOptions) {}

  get active(): boolean {
    return this.state !== null;
  }

  get current(): ScrubState | null {
    return this.state;
  }

  /**
   * One press towards the end (1) or the start (-1). `repeat` is a held key's auto-repeat, which is
   * throttled. Returns false when there is nothing to scrub (no length yet).
   */
  step(direction: 1 | -1, repeat = false): boolean {
    const now = (this.options.now ?? Date.now)();
    const duration = this.options.duration();
    if (!(duration > 0)) return false;
    if (repeat && now - this.lastPress < SCRUB_REPEAT_MS) {
      this.schedule();
      return true;
    }
    this.streak = this.state && (repeat || now - this.lastPress <= SCRUB_STREAK_MS) ? this.streak + 1 : 0;
    this.lastPress = now;
    const origin = this.state?.origin ?? this.options.position();
    const from = this.state?.target ?? origin;
    const target = Math.min(duration, Math.max(0, from + direction * scrubStep(this.streak, duration)));
    this.state = { origin, target, duration };
    this.options.onChange?.(this.state);
    this.schedule();
    return true;
  }

  /** Seek to the target now (OK, or the pause after the last press). */
  commit(): void {
    const state = this.state;
    this.end();
    if (state && Math.abs(state.target - state.origin) >= 0.5) this.options.commit(state.target);
  }

  /** Back: forget the target; playback never moved. */
  cancel(): void {
    this.end();
  }

  dispose(): void {
    clearTimeout(this.timer);
    this.state = null;
  }

  private schedule(): void {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => this.commit(), this.options.commitDelay ?? SCRUB_COMMIT_MS);
  }

  private end(): void {
    clearTimeout(this.timer);
    const was = this.state;
    this.state = null;
    this.streak = 0;
    this.lastPress = Number.NEGATIVE_INFINITY;
    if (was) this.options.onChange?.(null);
  }
}
