import { Badge, type BadgeTone } from "glass-ux/badge";
import { ToneDot } from "glass-ux/tone-dot";

import type { StatusMeta, StatusTone } from "@/lib/status";
import { cn } from "@/lib/utils";

const TONE: Record<StatusTone, BadgeTone> = {
  success: "success", warning: "warning", accent: "info", danger: "danger", default: "neutral",
};

interface StatusChipProps {
  status: StatusMeta;
  size?: "sm" | "md";
  className?: string;
  minimal?: boolean;
}

export function StatusChip({ status, size = "sm", className, minimal }: StatusChipProps) {
  if (minimal) {
    return <span className={cn("inline-flex items-center gap-1.5 type-label", className)}><ToneDot tone={TONE[status.tone]} />{status.label}</span>;
  }
  return <Badge tone={TONE[status.tone]} size={size} className={className}>{status.label}</Badge>;
}
