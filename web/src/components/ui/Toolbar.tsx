import { IconLayoutGrid, IconList } from "@tabler/icons-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

import { GlassSegmented } from "./GlassControls";

export type ViewMode = "card" | "table";

interface ViewToggleProps {
  value: ViewMode;
  onChange: (mode: ViewMode) => void;
  className?: string;
}

export function ViewToggle({ value, onChange, className }: ViewToggleProps) {
  return (
    <GlassSegmented<ViewMode>
      ariaLabel="View"
      size="sm"
      value={value}
      onChange={onChange}
      className={className}
      items={[
        { key: "card", label: null, ariaLabel: "Grid view", icon: <IconLayoutGrid size={16} /> },
        { key: "table", label: null, ariaLabel: "List view", icon: <IconList size={16} /> },
      ]}
    />
  );
}

interface ToolbarProps {
  children: ReactNode;
  className?: string;
  trailing?: ReactNode;
}

/** A single row of controls under a page header. Wraps on narrow screens. */
export function Toolbar({ children, trailing, className }: ToolbarProps) {
  return (
    <div className={cn("flex flex-wrap items-center gap-2", className)}>
      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2">{children}</div>
      {trailing ? <div className="flex shrink-0 items-center gap-2">{trailing}</div> : null}
    </div>
  );
}
