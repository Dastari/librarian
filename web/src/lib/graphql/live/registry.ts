import type { LiveTopic } from "./documents";

/** Active-view scheduling only. Events invalidate authorized snapshots; they are
 * never treated as authority to insert a row into a filtered/sorted connection. */
export interface LiveEvent { topic: LiveTopic; id?: string }
export interface LiveDependency { topic: LiveTopic; ids?: readonly string[] }
export interface LiveRegistration {
  key: string;
  /** Apollo watches share cache writes; local snapshots require every owner callback. */
  shared?: boolean;
  dependencies: readonly LiveDependency[];
  refresh: () => unknown | Promise<unknown>;
}
export type LiveSource = (topic: LiveTopic, event: (event: LiveEvent) => void, interrupted: (retry?: boolean) => void) => () => void;
interface Entry {
  consumers: Map<symbol, LiveRegistration>;
  dirty: boolean;
  busy: boolean;
  due: number;
  lastStarted: number;
  failures: number;
}

export function stableKey(value: unknown): string {
  return JSON.stringify(value, (_key, item: unknown) => item && typeof item === "object" && !Array.isArray(item)
    ? Object.fromEntries(Object.entries(item).sort(([a], [b]) => a.localeCompare(b))) : item) ?? "null";
}

export class LiveRegistry {
  private entries = new Map<string, Entry>();
  private sources = new Map<LiveTopic, { stop: () => void; retry?: ReturnType<typeof setTimeout>; failed?: boolean; terminal?: boolean }>();
  private timer?: ReturnType<typeof setTimeout>;
  private sourceAttempts = new Map<LiveTopic, number>();
  private registrationSequence = 0;
  private epoch = 0;
  private running = 0;
  private enabled = false;
  private connected = false;

  constructor(private source: LiveSource, private options = { coalesceMs: 150, minGapMs: 1000, concurrency: 4, maxRetries: 4 }) {}

  /** Principal/client changes discard all prior callbacks and pending work. */
  reset(enabled: boolean): void {
    this.epoch++;
    this.enabled = enabled;
    this.connected = false;
    if (this.timer) clearTimeout(this.timer);
    this.timer = undefined;
    for (const source of this.sources.values()) { source.stop(); if (source.retry) clearTimeout(source.retry); }
    this.sources.clear();
    this.sourceAttempts.clear();
    this.entries.clear();
    this.running = 0;
  }

  register(registration: LiveRegistration): () => void {
    const token = Symbol();
    const key = registration.shared === false ? `${registration.key}:owner:${++this.registrationSequence}` : registration.key;
    let entry = this.entries.get(key);
    if (!entry) {
      entry = { consumers: new Map(), dirty: false, busy: false, due: 0, lastStarted: -Infinity, failures: 0 };
      this.entries.set(key, entry);
    }
    entry.consumers.set(token, registration);
    this.reconcileSources();
    const epoch = this.epoch;
    return () => {
      if (epoch !== this.epoch) return;
      entry.consumers.delete(token);
      if (!entry.consumers.size) this.entries.delete(key);
      this.reconcileSources();
    };
  }

  invalidate(event?: LiveEvent): void {
    if (!this.enabled) return;
    for (const entry of this.entries.values()) {
      const matches = !event || [...entry.consumers.values()].some((consumer) => consumer.dependencies.some((dependency) =>
        dependency.topic === event.topic && (!dependency.ids || !event.id || dependency.ids.includes(event.id))));
      if (!matches) continue;
      if (!entry.dirty || entry.failures > this.options.maxRetries) entry.due = Math.max(Date.now() + this.options.coalesceMs, entry.lastStarted + this.options.minGapMs);
      entry.dirty = true;
      entry.failures = 0;
    }
    this.schedule();
  }

  /** A healthy socket is not a replay log. Re-read once after any connection gap. */
  transport(connected: boolean): void {
    if (connected && !this.connected) {
      for (const [topic, source] of this.sources) if (source.failed && !source.terminal) {
        source.stop(); if (source.retry) clearTimeout(source.retry);
        this.sources.delete(topic); this.sourceAttempts.delete(topic);
      }
      this.reconcileSources();
      this.invalidate();
    }
    this.connected = connected;
  }

  private reconcileSources(): void {
    const topics = new Set(this.enabled ? [...this.entries.values()].flatMap((entry) => [...entry.consumers.values()].flatMap((consumer) => consumer.dependencies.map((dependency) => dependency.topic))) : []);
    for (const [topic, source] of this.sources) if (!topics.has(topic)) {
      source.stop(); if (source.retry) clearTimeout(source.retry); this.sources.delete(topic); this.sourceAttempts.delete(topic);
    }
    for (const topic of topics) if (!this.sources.has(topic)) this.startSource(topic);
  }

  private startSource(topic: LiveTopic): void {
    const epoch = this.epoch;
    const holder: { stop: () => void; retry?: ReturnType<typeof setTimeout>; failed?: boolean; terminal?: boolean } = { stop: () => {} };
    this.sources.set(topic, holder);
    const interrupted = (retry = true) => {
      if (epoch !== this.epoch || this.sources.get(topic) !== holder || holder.retry) return;
      // Completion/lag and errors both lose continuity. Resnapshot and recreate
      // the listener; this timer retries the stream, never polls domain reads.
      holder.failed = true; holder.terminal = !retry;
      const attempt = (this.sourceAttempts.get(topic) ?? 0) + 1;
      this.sourceAttempts.set(topic, attempt);
      if (attempt === 1) this.invalidate({ topic });
      if (!retry || attempt > this.options.maxRetries) return;
      holder.retry = setTimeout(() => {
        if (this.sources.get(topic) !== holder) return;
        holder.stop(); this.sources.delete(topic); this.reconcileSources();
      }, Math.min(30_000, 1500 * 2 ** (attempt - 1)));
    };
    holder.stop = this.source(topic, (event) => {
      if (epoch === this.epoch && this.sources.get(topic) === holder) {
        holder.failed = false;
        this.sourceAttempts.delete(topic);
        this.invalidate(event);
      }
    }, interrupted);
  }

  private schedule(): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = undefined;
    if (!this.enabled || this.running >= this.options.concurrency) return;
    const pending = [...this.entries.values()].filter((entry) => entry.dirty && !entry.busy && entry.failures <= this.options.maxRetries);
    if (!pending.length) return;
    const due = Math.min(...pending.map((entry) => entry.due));
    this.timer = setTimeout(() => { this.timer = undefined; this.drain(); }, Math.max(0, due - Date.now()));
  }

  private drain(): void {
    const epoch = this.epoch;
    for (const [key, entry] of this.entries) {
      if (this.running >= this.options.concurrency) break;
      if (!entry.dirty || entry.busy || entry.due > Date.now() || entry.failures > this.options.maxRetries) continue;
      // Identical regular Apollo queries share the same normalized result. One
      // callback refreshes every mounted consumer, including all three shell bars.
      const consumer = [...entry.consumers.values()].at(-1);
      if (!consumer) continue;
      entry.dirty = false; entry.busy = true; entry.lastStarted = Date.now(); this.running++;
      void Promise.resolve().then(() => {
        if (epoch !== this.epoch || this.entries.get(key) !== entry) return;
        return [...entry.consumers.values()].at(-1)?.refresh();
      }).then(() => { entry.failures = 0; }, () => {
        entry.failures++;
        entry.dirty = true;
        entry.due = Date.now() + Math.min(30_000, 1000 * 2 ** entry.failures);
      }).finally(() => {
        if (epoch !== this.epoch) return;
        this.running--; entry.busy = false;
        if (this.entries.get(key) === entry && entry.dirty) entry.due = Math.max(entry.due, entry.lastStarted + this.options.minGapMs);
        this.schedule();
      });
    }
    this.schedule();
  }
}
