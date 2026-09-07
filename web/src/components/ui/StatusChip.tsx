import { Chip } from "@heroui/react";

import type { StatusMeta, StatusTone } from "@/lib/status";
import { cn } from "@/lib/utils";

const CHIP_COLOR: Record<StatusTone, "success" | "warning" | "accent" | "danger" | "default"> = {
  success: "success",
  warning: "warning",
  accent: "accent",
  danger: "danger",
  default: "default",
};

/** Informational chips are blue, distinct from the gold accent used for primary actions. */
const TONE_CLASS: Partial<Record<StatusTone, string>> = {
  accent: "!bg-info-soft !text-info",
};

interface StatusChipProps {
  status: StatusMeta;
  size?: "sm" | "md";
  className?: string;
  /** Show only the dot and label without the chip background (for dense tables). */
  minimal?: boolean;
}

export function StatusChip({ status, size = "sm", className, minimal }: StatusChipProps) {
  if (minimal) {
    return (
      <span className={cn("inline-flex items-center gap-1.5 text-label text-foreground/85", className)}>
        <span className={cn("size-1.5 rounded-full", status.dot)} />
        {status.label}
      </span>
    );
  }
  return (
    <Chip color={CHIP_COLOR[status.tone]} size={size} variant="soft" className={cn("whitespace-nowrap", TONE_CLASS[status.tone], className)}>
      {status.label}
    </Chip>
  );
}
