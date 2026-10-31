import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

/**
 * A settings list without a frame: a heading row (title, one line of help, actions) above a
 * table that sits on the page, like every other entity table (docs/entity-tables.md).
 */
export function SettingsSection({ title, description, actions, children, className }: { title: string; description?: ReactNode; actions?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <section className={cn("flex min-w-0 flex-col gap-3", className)}>
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-title-sm text-foreground">{title}</h2>
          {description ? <p className="text-label-sm text-muted">{description}</p> : null}
        </div>
        {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
      </div>
      {children}
    </section>
  );
}
