import { IconGrid, IconList } from "glass-ux/icons";
import { Segmented } from "glass-ux/segmented";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export type ViewMode = "card" | "table";

interface ViewToggleProps {
  value: ViewMode;
  onChange: (mode: ViewMode) => void;
  className?: string;
}

export function ViewToggle({ value, onChange, className }: ViewToggleProps) {
  return (
    <Segmented<ViewMode>
      aria-label="View"
      iconOnly
      size="sm"
      value={value}
      onValueChange={onChange}
      className={className}
      options={[
        { value: "card", label: "Grid view", icon: <IconGrid /> },
        { value: "table", label: "List view", icon: <IconList /> },
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
