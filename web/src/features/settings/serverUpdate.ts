import type { ServerUpdateStatusFieldsFragment } from "@/graphql/generated/graphql";

export type ServerUpdate = ServerUpdateStatusFieldsFragment;
export type UpgradeOutcome = ServerUpdate["history"][number];

const BUILD_KINDS: Record<string, string> = {
  release: "Release build",
  debug: "Debug build",
  docker: "Docker image",
  dev: "Release build in dev mode",
  source: "Built from source",
};

/** `buildKind` as words. */
export function buildKindLabel(kind: string): string {
  return BUILD_KINDS[kind] ?? kind;
}

const STAGES: Record<string, string> = {
  downloading: "Downloading",
  verifying: "Verifying",
  backing_up: "Backing up",
  installing: "Installing",
  restarting: "Restarting",
  failed: "Failed",
};

export function stageLabel(stage: string): string {
  return STAGES[stage] ?? stage;
}

/** An upgrade is running (not finished, not failed): the page polls while it is. */
export function upgradeRunning(status: Pick<ServerUpdate, "progress"> | null | undefined): boolean {
  return Boolean(status?.progress && !status.progress.failed);
}

const OUTCOMES: Record<string, { label: string; tone: "success" | "danger" | "warning" }> = {
  upgraded: { label: "Upgraded", tone: "success" },
  rolled_back: { label: "Rolled back", tone: "warning" },
  failed: { label: "Failed", tone: "danger" },
};

export function outcomeLabel(outcome: string): { label: string; tone: "success" | "danger" | "warning" | "neutral" } {
  return OUTCOMES[outcome] ?? { label: outcome, tone: "neutral" };
}

/** The one-line answer to "am I up to date?". */
export function summary(status: Pick<ServerUpdate, "currentVersion" | "updateAvailable" | "latest" | "lastCheckedAt" | "channel">): string {
  if (!status.lastCheckedAt) return "Not checked yet";
  if (!status.latest) return `No ${status.channel === "dev" ? "" : "stable "}releases found`;
  if (status.updateAvailable) return `Librarian ${status.latest.version} is available`;
  return `Up to date (${status.currentVersion})`;
}

/** The release notes from a `server_update` notification's `actionData`. */
export function serverUpdateDetails(actionData: string | null | undefined): { version?: string; url?: string; canUpgrade: boolean } | null {
  if (!actionData) return null;
  try {
    const data = JSON.parse(actionData) as Record<string, unknown>;
    const text = (value: unknown) => (typeof value === "string" && value ? value : undefined);
    const url = text(data.url);
    // Only GitHub release pages are offered as links.
    return { version: text(data.version), url: url && /^https:\/\/github\.com\//.test(url) ? url : undefined, canUpgrade: data.canUpgrade === true };
  } catch {
    return null;
  }
}
