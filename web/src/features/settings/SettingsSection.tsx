import { useId, type ReactNode } from "react";

import { cn } from "@/lib/utils";

interface SettingsSectionProps {
  title: string;
  description?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  /**
   * `full` (the default) is a list of records: a table on the page canvas, edge to edge.
   * `readable` is a group of setting rows, held to `--settings-measure` so lines stay readable
   * on wide screens. A form that wraps several groups sets the measure once, on itself.
   */
  width?: "full" | "readable";
}

/**
 * One group on a settings page, without a frame. A table gets a heading row (title, one line of
 * help, actions) above it; a group of setting rows gets the overline group label of R15.8.
 * Groups are separated by space (`SettingsPage`), never by a divider or a card.
 */
export function SettingsSection({ title, description, actions, children, className, width = "full" }: SettingsSectionProps) {
  const headingId = useId();
  const rows = width === "readable";
  return (
    <section className={cn("flex min-w-0 flex-col", rows ? "settings-measure gap-2" : "gap-3", className)}>
      <div className="flex flex-wrap items-end justify-between gap-x-3 gap-y-2">
        <div className="min-w-0">
          <h2 id={headingId} className={rows ? "text-overline text-muted" : "text-title-sm text-foreground"}>{title}</h2>
          {description ? <p className={cn("text-label-sm text-muted", rows && "mt-1 max-w-[var(--form-measure)]")}>{description}</p> : null}
        </div>
        {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
      </div>
      {rows ? <div role="group" aria-labelledby={headingId} className="flex min-w-0 flex-col gap-2">{children}</div> : children}
    </section>
  );
}

/** The vertical rhythm of a settings page: its groups and tables are `--gap-section` plus apart. */
export function SettingsPage({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("flex min-w-0 flex-col gap-8", className)}>{children}</div>;
}
