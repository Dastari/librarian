import type { LiveTopic } from "./documents";

/** Active-view scheduling only. Events invalidate authorized snapshots; they are
 * never treated as authority to insert a row into a filtered/sorted connection. */
export interface LiveEvent { topic: LiveTopic; id?: string; propagated?: boolean }
/** `ids` narrows the dependency to those records; `direct` ignores events propagated up from child records. */
export interface LiveDependency { topic: LiveTopic; ids?: readonly string[]; direct?: boolean }
export interface LiveRegistration {
  key: string;
  /** Apollo watches share cache writes; local snapshots require every owner callback. */
  shared?: boolean;
  dependencies: readonly LiveDependency[];
  refresh: () => unknown | Promise<unknown>;
}
/** `keys` is set only when every consumer of the topic names its records, so the stream can be keyed. */
export type LiveSource = (topic: LiveTopic, event: (event: LiveEvent) => void, interrupted: (retry?: boolean) => void, keys?: readonly string[]) => () => void;
interface Source { stop: () => void; retry?: ReturnType<typeof setTimeout>; failed?: boolean; terminal?: boolean; retiring?: boolean; keys?: readonly string[] }
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

/** Generated `Changed` subscriptions reject longer `keys` lists. */
export const MAX_SUBSCRIPTION_KEYS = 500;
const RESUBSCRIBE_JITTER_MS = 1000;
const KEY_SWAP_DEBOUNCE_MS = 200;
const KEY_SWAP_OVERLAP_MS = 2000;
const sameKeys = (a?: readonly string[], b?: readonly string[]) => a === b || (a !== undefined && b !== undefined && a.length === b.length && a.every((key, index) => key === b[index]));

export class LiveRegistry {
  private entries = new Map<string, Entry>();
  private sources = new Map<LiveTopic, Source>();
  private retiring = new Set<Source>();
  private timer?: ReturnType<typeof setTimeout>;
  private swapTimer?: ReturnType<typeof setTimeout>;
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
    if (this.swapTimer) clearTimeout(this.swapTimer);
    this.swapTimer = undefined;
    for (const source of [...this.sources.values(), ...this.retiring]) { source.stop(); if (source.retry) clearTimeout(source.retry); }
    this.sources.clear();
    this.retiring.clear();
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
        dependency.topic === event.topic && !(dependency.direct && event.propagated) && (!dependency.ids || !event.id || dependency.ids.includes(event.id))));
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

  /** Per topic: the ids to key the stream on, or undefined when any consumer needs the whole topic. */
  private wanted(): Map<LiveTopic, readonly string[] | undefined> {
    const wanted = new Map<LiveTopic, Set<string> | undefined>();
    if (this.enabled) for (const entry of this.entries.values()) for (const consumer of entry.consumers.values()) for (const dependency of consumer.dependencies) {
      const ids = wanted.has(dependency.topic) ? wanted.get(dependency.topic) : new Set<string>();
      if (!ids) continue;
      if (dependency.ids) { for (const id of dependency.ids) ids.add(id); wanted.set(dependency.topic, ids); } else wanted.set(dependency.topic, undefined);
    }
    return new Map([...wanted].map(([topic, ids]) => [topic, ids && ids.size <= MAX_SUBSCRIPTION_KEYS ? [...ids].sort() : undefined]));
  }

  private reconcileSources(): void {
    const wanted = this.wanted();
    for (const [topic, source] of this.sources) if (!wanted.has(topic)) {
      source.stop(); if (source.retry) clearTimeout(source.retry); this.sources.delete(topic); this.sourceAttempts.delete(topic);
    }
    let changed = false;
    for (const [topic, keys] of wanted) {
      const source = this.sources.get(topic);
      if (!source) this.startSource(topic, keys);
      else if (!sameKeys(source.keys, keys)) changed = true;
    }
    if (changed && !this.swapTimer) this.swapTimer = setTimeout(() => { this.swapTimer = undefined; this.swapKeys(); }, KEY_SWAP_DEBOUNCE_MS);
  }

  /** The graphql-ws arguments of a live subscription cannot change, so open the new one before
   * closing the old. The old stream keeps delivering until the new one is surely admitted, and
   * records added to the set are re-read because nothing is replayed to the new subscription. */
  private swapKeys(): void {
    for (const [topic, keys] of this.wanted()) {
      const old = this.sources.get(topic);
      if (!old || sameKeys(old.keys, keys)) continue;
      if (old.retry) clearTimeout(old.retry);
      const added = keys === undefined ? [] : keys.filter((key) => !old.keys?.includes(key));
      this.sources.delete(topic);
      this.startSource(topic, keys);
      old.retiring = true;
      this.retiring.add(old);
      old.retry = setTimeout(() => { this.retiring.delete(old); old.stop(); }, KEY_SWAP_OVERLAP_MS);
      for (const id of added) this.invalidate({ topic, id });
    }
  }

  private startSource(topic: LiveTopic, keys?: readonly string[]): void {
    const epoch = this.epoch;
    const holder: Source = { stop: () => {}, keys };
    this.sources.set(topic, holder);
    const interrupted = (retry = true) => {
      if (epoch !== this.epoch || this.sources.get(topic) !== holder || holder.retry || holder.terminal) return;
      // Completion/lag and errors both lose continuity. Subscribe again first and
      // re-read afterwards, so a write between the two is delivered or fetched.
      holder.failed = true; holder.terminal = !retry;
      const attempt = (this.sourceAttempts.get(topic) ?? 0) + 1;
      this.sourceAttempts.set(topic, attempt);
      if (!retry) { if (attempt === 1) this.invalidate({ topic }); return; }
      if (attempt > this.options.maxRetries) return;
      const backoff = attempt === 1 ? 0 : Math.min(30_000, 1500 * 2 ** (attempt - 1));
      holder.retry = setTimeout(() => {
        if (this.sources.get(topic) !== holder) return;
        holder.stop(); this.sources.delete(topic); this.reconcileSources();
        this.invalidate({ topic });
      }, backoff + Math.random() * RESUBSCRIBE_JITTER_MS);
    };
    holder.stop = this.source(topic, (event) => {
      if (epoch !== this.epoch) return;
      if (this.sources.get(topic) === holder) {
        holder.failed = false;
        this.sourceAttempts.delete(topic);
        this.invalidate(event);
      } else if (holder.retiring) this.invalidate(event);
    }, interrupted, keys);
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
