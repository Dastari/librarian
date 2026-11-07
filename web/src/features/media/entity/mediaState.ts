import type { EntityState } from "glass-ux/entity";

import type { StatusMeta } from "@/lib/status";

const BUSY = /^(Playing|Downloading|Processing)/;

/** A title's one status as an identity's state: its words, its colour, and whether it's moving. */
export function mediaEntityState(status: StatusMeta): EntityState {
  return {
    id: status.label.toLowerCase().split(" ")[0]!,
    label: status.label,
    tone: status.tone === "default" ? "neutral" : status.tone === "accent" ? "info" : status.tone,
    activity: BUSY.test(status.label) || undefined,
    paused: status.label === "Paused" || undefined,
  };
}
