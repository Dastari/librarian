import type { Icon as TablerIcon } from "@tabler/icons-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface EmptyStateProps {
  icon: TablerIcon;
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  className?: string;
  compact?: boolean;
}

export function EmptyState({ icon: Icon, title, description, action, className, compact }: EmptyStateProps) {
  return (
    <div
      className={cn(
        "glass-surface flex flex-col items-center justify-center rounded-card text-center",
        compact ? "gap-2 px-4 py-8" : "gap-3 px-6 py-16",
        className,
      )}
    >
      <div className="grid size-14 place-items-center rounded-2xl bg-surface-secondary text-muted">
        <Icon size={compact ? 24 : 30} stroke={1.5} />
      </div>
      <div className="max-w-sm">
        <p className="text-title-md text-foreground">{title}</p>
        {description ? <p className="mt-1 text-body-sm text-muted">{description}</p> : null}
      </div>
      {action ? <div className="mt-2">{action}</div> : null}
    </div>
  );
}
