
import { IconAlertTriangle, IconRefresh } from "@tabler/icons-react";
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
    <div
      role="alert"
      className={cn(
        "flex flex-col items-center justify-center gap-3 rounded-card border border-danger/30 bg-danger/5 text-center",
        compact ? "px-4 py-6" : "px-6 py-12",
        className,
      )}
    >
      <IconAlertTriangle size={compact ? 22 : 28} className="text-danger" stroke={1.75} />
      <div className="max-w-md">
        <p className="text-title-md text-foreground">{title}</p>
        <p className="mt-1 text-body-sm text-muted">{errorMessage(error)}</p>
      </div>
      {onRetry ? (
        <Button size="sm" variant="secondary" onPress={onRetry}>
          <IconRefresh size={16} />
          Try again
        </Button>
      ) : null}
    </div>
  );
}
