import { Metric } from "glass-ux/metric";
import { Tooltip } from "glass-ux/tooltip";
import type { ComponentProps, ReactNode } from "react";

/** One value with an icon naming it, on a line of them. */
export function Stat({ icon, label, children }: { icon: ReactNode; label: string; children: ReactNode }) {
  return (
    <div className="inline-flex min-w-0 items-center gap-1.5">
      <Tooltip content={label}><span className="inline-flex shrink-0 text-muted" role="img" aria-label={label}>{icon}</span></Tooltip>
      {children}
    </div>
  );
}

/** Metric hides its own label at this width, leaving the value and a small trend beside the icon. */
export function LiveStat({ icon, width = "w-36", ...metric }: { icon: ReactNode; label: string; width?: string } & ComponentProps<typeof Metric>) {
  return <Stat icon={icon} label={metric.label}><div className={`h-6 min-w-0 max-sm:w-[5.25rem] ${width}`}><Metric better="none" range="Last two minutes" sparklineFill {...metric} /></div></Stat>;
}
