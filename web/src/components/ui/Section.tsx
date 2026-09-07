import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface SectionProps {
  title: ReactNode;
  /** Trailing element: a "See all" link, a count, or a small control. */
  trailing?: ReactNode;
  children: ReactNode;
  className?: string;
  /** Children run edge to edge (poster rows); the heading keeps the page gutter. */
  bleed?: boolean;
  id?: string;
}

export function Section({ title, trailing, children, className, bleed, id }: SectionProps) {
  return (
    <section id={id} className={cn("flex flex-col gap-3", className)} aria-label={typeof title === "string" ? title : undefined}>
      <div className={cn("flex items-baseline justify-between gap-4", bleed && "page-gutter")}>
        <h2 className="text-title-lg text-foreground">{title}</h2>
        {trailing ? <div className="text-label text-muted">{trailing}</div> : null}
      </div>
      <div>{children}</div>
    </section>
  );
}
