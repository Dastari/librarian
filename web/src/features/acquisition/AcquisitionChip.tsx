import { StatusChip } from "@/components/ui";
import type { StatusMeta } from "@/lib/status";
import { cn } from "@/lib/utils";

/** The download-mode chip in a detail header. Pressing it opens the acquisition dialog. */
export function AcquisitionChip({ status, onPress, className }: { status: StatusMeta; onPress: () => void; className?: string }) {
  return (
    <button
      type="button"
      data-focusable
      onClick={onPress}
      aria-label={`Download settings — ${status.label}`}
      className={cn("nav-focus rounded-pill transition-opacity duration-fast hover:opacity-80", className)}
    >
      <StatusChip status={status} />
    </button>
  );
}
