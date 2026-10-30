import { Button } from "glass-ux/button";
import { IconSettings } from "glass-ux/icons";

import { StatusChip } from "@/components/ui";
import type { StatusMeta } from "@/lib/status";

/** Download status is passive for viewers; admins can open acquisition settings. */
export function AcquisitionChip({ status, onPress, className }: { status: StatusMeta; onPress?: () => void; className?: string }) {
  if (!onPress) return <StatusChip status={status} className={className} />;
  return <Button size="sm" variant="ghost" icon={<IconSettings />} onClick={onPress} aria-label={`Download settings — ${status.label}`} className={className}>{status.label}</Button>;
}
