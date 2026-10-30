import { EmptyState as GlassEmptyState } from "glass-ux/empty-state";
import type { ComponentType, ReactNode } from "react";

import { cn } from "@/lib/utils";

interface EmptyStateProps {
  icon: ComponentType<{ className?: string }>;
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  className?: string;
  compact?: boolean;
}

export function EmptyState({ icon: Icon, title, description, action, className, compact }: EmptyStateProps) {
  return (
    <div className={cn("w-full min-w-0", compact ? "h-40" : "h-64", className)}>
      <GlassEmptyState icon={<Icon className="fa-icon" />} action={action}>
        {title}
        {description ? <span className="block type-body text-muted">{description}</span> : null}
      </GlassEmptyState>
    </div>
  );
}
