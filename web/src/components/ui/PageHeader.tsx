import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface PageHeaderProps {
  title: ReactNode;
  /** Navigation already names this route; retain its heading for assistive technology. */
  titleVisibility?: "visible" | "accessible";
  /** Small label above the title (library name, section). */
  eyebrow?: ReactNode;
  /** One short line under the title. Use sparingly. */
  meta?: ReactNode;
  actions?: ReactNode;
  className?: string;
  size?: "md" | "lg";
}

/** The single page heading. Title left, actions right, wraps on narrow screens. */
export function PageHeader({ title, eyebrow, meta, actions, className, size = "md", titleVisibility = "visible" }: PageHeaderProps) {
  return (
    <header className={cn("flex min-w-0 shrink-0 flex-wrap items-end justify-between gap-x-6 gap-y-3 [@media(max-height:35rem)]:[&_h1]:text-title-sm [@media(max-height:35rem)]:[&_.text-overline]:hidden", className)}>
      <div className="min-w-0">
        {eyebrow ? <div className="text-overline mb-1.5 text-muted">{eyebrow}</div> : null}
        <h1 className={titleVisibility === "accessible" ? "sr-only route-print-title" : cn("break-words text-foreground", size === "lg" ? "text-display-lg" : "text-display-md")}>{title}</h1>
        {meta ? <div className={cn("text-body-sm text-muted", titleVisibility === "visible" && "mt-1.5")}>{meta}</div> : null}
      </div>
      {actions ? <div data-print="hide" className="flex min-w-0 max-w-full flex-wrap items-center gap-2">{actions}</div> : null}
    </header>
  );
}
