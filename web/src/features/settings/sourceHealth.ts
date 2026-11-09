import { formatRelative } from "@/lib/format";

/** What the health fields of a source or feed say, in words a person reads. */
export interface HealthInput {
  enabled: boolean;
  /** `ok`, `failing` or `auth_expired`; absent on rows never checked. */
  healthStatus?: string | null;
  /** `auth`, `unreachable`, `rate_limited` or `other`. */
  healthKind?: string | null;
  lastError?: string | null;
  lastSuccessAt?: string | null;
  lastCheckedAt?: string | null;
  /** Consecutive failures: `errorCount` on a source, `consecutiveFailures` on a feed. */
  failures?: number | null;
}

export interface HealthState {
  label: string;
  tone: "success" | "danger" | "warning" | "neutral";
  /** The identity cell's status dot. */
  status: "online" | "error" | "offline";
  /** The reason, and when it last worked, for the row's second line. */
  detail: string;
}

const KIND_LABEL: Record<string, string> = {
  auth: "Sign-in expired",
  unreachable: "Unreachable",
  rate_limited: "Rate limited",
  other: "Failing",
};

/** One short line saying when it last worked. */
export function lastWorked(lastSuccessAt: string | null | undefined, now = Date.now()): string {
  if (!lastSuccessAt) return "never worked yet";
  const time = Date.parse(lastSuccessAt);
  return Number.isNaN(time) ? "never worked yet" : `last worked ${formatRelative(lastSuccessAt, now)}`;
}

/**
 * The state a Sources row shows: off, working, not checked yet, or failing with the reason and when
 * it last worked. A failure is red, except being rate limited, which the source recovers from by
 * waiting.
 */
export function healthState(input: HealthInput, now = Date.now()): HealthState {
  if (!input.enabled) return { label: "Disabled", tone: "neutral", status: "offline", detail: "Switched off" };
  const failing = input.healthStatus === "failing" || input.healthStatus === "auth_expired" || (!input.healthStatus && Boolean(input.lastError));
  if (failing) {
    const kind = input.healthStatus === "auth_expired" ? "auth" : input.healthKind ?? "other";
    const times = input.failures && input.failures > 1 ? ` ×${input.failures}` : "";
    const reason = input.lastError ?? (kind === "auth" ? "Its login has expired" : "It is failing");
    return {
      label: `${KIND_LABEL[kind] ?? "Failing"}${times}`,
      tone: kind === "rate_limited" ? "warning" : "danger",
      status: "error",
      detail: `${reason} · ${lastWorked(input.lastSuccessAt, now)}`,
    };
  }
  if (input.healthStatus === "ok") {
    return { label: "Working", tone: "success", status: "online", detail: input.lastCheckedAt ? `Checked ${formatRelative(input.lastCheckedAt, now)}` : "Working" };
  }
  return { label: "Not checked yet", tone: "neutral", status: "online", detail: "Checked within the hour" };
}
