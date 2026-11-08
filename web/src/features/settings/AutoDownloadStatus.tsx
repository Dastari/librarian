import { RelativeTime } from "glass-ux/data-table";

import { parseTimestamp } from "@/lib/format";

/** One pass as the scheduler records it in `auto_download.status`. */
interface PassRecord {
  at?: string;
  candidates?: number;
  searched?: number;
  grabbed?: number;
  backoffSkipped?: number;
  capped?: number;
  feeds?: number;
  failed?: number;
  newItems?: number;
  matched?: number;
  errors?: string[];
}

const PASSES: Array<{ key: string; label: string }> = [
  { key: "sweep", label: "Last sweep" },
  { key: "airTime", label: "Last air-time check" },
  { key: "rss", label: "Last RSS poll" },
  { key: "manual", label: "Last run by hand" },
];

/** Reads the stored status, tolerating a missing or damaged value. */
export function parseStatus(raw: string | undefined): Record<string, PassRecord> {
  if (!raw) return {};
  try {
    const value = JSON.parse(raw) as unknown;
    return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, PassRecord>) : {};
  } catch {
    return {};
  }
}

function describe(kind: string, pass: PassRecord): string {
  if (kind === "rss") {
    const failed = pass.failed ? `, ${pass.failed} failed` : "";
    return `${pass.feeds ?? 0} feed(s)${failed}, ${pass.newItems ?? 0} new item(s), ${pass.matched ?? 0} matched, ${pass.grabbed ?? 0} grabbed`;
  }
  const parts = [`searched ${pass.searched ?? 0}`, `grabbed ${pass.grabbed ?? 0}`];
  if (pass.backoffSkipped) parts.push(`${pass.backoffSkipped} waiting to be searched again`);
  if (pass.capped) parts.push(`${pass.capped} waiting for a free slot`);
  if (pass.errors?.length) parts.push(`${pass.errors.length} error(s)`);
  return parts.join(", ");
}

/** When automatic downloading last did something, so "On" can be told from "On but never ran". */
export function AutoDownloadStatus({ settings }: { settings: Map<string, string> }) {
  const status = parseStatus(settings.get("auto_download.status"));
  const shown = PASSES.filter(({ key }) => status[key]?.at);
  if (!shown.length) return <p className="text-label-sm text-muted">Nothing has run yet.</p>;
  return (
    <ul className="flex flex-col gap-1 text-label-sm text-muted" aria-label="Recent automatic downloads">
      {shown.map(({ key, label }) => (
        <li key={key}>
          {label} <RelativeTime value={parseTimestamp(status[key]!.at)} />: {describe(key, status[key]!)}
        </li>
      ))}
    </ul>
  );
}
