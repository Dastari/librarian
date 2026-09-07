import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface PanelProps {
  children: ReactNode;
  className?: string;
  title?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  /** Remove inner padding (for tables and lists that manage their own). */
  flush?: boolean;
  tone?: "default" | "secondary" | "glass";
}

/** Flat surface card used for settings groups, detail metadata and dashboard tiles. */
export function Panel({ children, className, title, description, actions, flush, tone = "default" }: PanelProps) {
  return (
    <section
      className={cn(
        "flex flex-col overflow-hidden rounded-card",
        tone === "secondary" ? "glass-surface glass-opaque" : "glass-surface",
        className,
      )}
    >
      {title || actions ? (
        <header className={cn("flex items-start justify-between gap-4 px-4 pt-4", flush ? "pb-3" : "pb-1")}>
          <div className="min-w-0">
            {title ? <h3 className="text-title-md text-foreground">{title}</h3> : null}
            {description ? <p className="mt-0.5 text-body-sm text-muted">{description}</p> : null}
          </div>
          {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
        </header>
      ) : null}
      <div className={cn(flush ? "" : "p-4", title && !flush && "pt-3")}>{children}</div>
    </section>
  );
}
