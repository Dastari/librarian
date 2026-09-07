import type { Icon as TablerIcon } from "@tabler/icons-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface StatTileProps {
  label: string;
  value: ReactNode;
  hint?: ReactNode;
  icon?: TablerIcon;
  tone?: "default" | "brand" | "accent" | "success" | "warning" | "danger";
  className?: string;
}

const TONE: Record<NonNullable<StatTileProps["tone"]>, string> = {
  default: "text-muted",
  brand: "text-brand",
  accent: "text-accent",
  success: "text-success",
  warning: "text-warning",
  danger: "text-danger",
};

export function StatTile({ label, value, hint, icon: Icon, tone = "default", className }: StatTileProps) {
  return (
    <div className={cn("glass-surface flex items-start gap-3 rounded-card p-4", className)}>
      {Icon ? (
        <span className={cn("grid size-10 shrink-0 place-items-center rounded-xl bg-surface-secondary", TONE[tone])}>
          <Icon size={20} stroke={1.75} />
        </span>
      ) : null}
      <div className="min-w-0">
        <p className="text-label text-muted">{label}</p>
        <p className="text-numeric text-title-lg text-foreground">{value}</p>
        {hint ? <p className="mt-0.5 text-label-sm text-muted">{hint}</p> : null}
      </div>
    </div>
  );
}
