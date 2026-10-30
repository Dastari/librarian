import { EmptyState } from "glass-ux/empty-state";
import { IconRefresh, IconWarning } from "glass-ux/icons";
import type { ReactNode } from "react";

import { errorMessage } from "@/lib/graphql/errors";
import { cn } from "@/lib/utils";

import { Button } from "./Button";

interface ErrorStateProps {
  error: unknown;
  title?: ReactNode;
  onRetry?: () => void;
  className?: string;
  compact?: boolean;
}

export function ErrorState({ error, title = "Couldn't load this", onRetry, className, compact }: ErrorStateProps) {
  return (
    <div className={cn("w-full min-w-0", compact ? "h-40" : "h-64", className)}>
      <EmptyState tone="danger" icon={<IconWarning />} action={onRetry ? <Button size="sm" variant="secondary" onPress={onRetry} icon={<IconRefresh />}>Try again</Button> : undefined}>
        {title}
        <span className="block type-body text-muted">{errorMessage(error)}</span>
      </EmptyState>
    </div>
  );
}
