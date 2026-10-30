import type { Icon as TablerIcon } from "@tabler/icons-react";
import { DetailTile } from "glass-ux/detail-tile";
import { ToneDot } from "glass-ux/tone-dot";
import type { ReactNode } from "react";

interface StatTileProps {
  label: string;
  value: ReactNode;
  hint?: ReactNode;
  icon?: TablerIcon;
  tone?: "default" | "brand" | "accent" | "success" | "warning" | "danger";
  className?: string;
}

export function StatTile({ label, value, hint, icon: Icon, tone = "default", className }: StatTileProps) {
  const stateTone = tone === "success" || tone === "warning" || tone === "danger" ? tone : undefined;
  return (
    <DetailTile className={className} label={label} value={stateTone ? <span className="inline-flex items-center gap-2"><ToneDot tone={stateTone} />{value}</span> : value} detail={hint} icon={Icon ? <Icon className="fa-icon" /> : undefined} numeric />
  );
}
