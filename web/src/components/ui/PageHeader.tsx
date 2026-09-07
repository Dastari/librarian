import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface PageHeaderProps {
  title: ReactNode;
  /** Small label above the title (library name, section). */
  eyebrow?: ReactNode;
  /** One short line under the title. Use sparingly. */
  meta?: ReactNode;
  actions?: ReactNode;
  className?: string;
  size?: "md" | "lg";
}

/** The single page heading. Title left, actions right, wraps on narrow screens. */
export function PageHeader({ title, eyebrow, meta, actions, className, size = "md" }: PageHeaderProps) {
  return (
    <header className={cn("flex flex-wrap items-end justify-between gap-x-6 gap-y-3", className)}>
      <div className="min-w-0">
        {eyebrow ? <div className="text-overline mb-1.5 text-muted">{eyebrow}</div> : null}
        <h1 className={cn("truncate text-foreground", size === "lg" ? "text-display-lg" : "text-display-md")}>{title}</h1>
        {meta ? <div className="mt-1.5 text-body-sm text-muted">{meta}</div> : null}
      </div>
      {actions ? <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div> : null}
    </header>
  );
}
