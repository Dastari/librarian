import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export interface KeyValueItem {
  label: string;
  value: ReactNode;
  mono?: boolean;
}

/** Two-column definition list for technical details (codecs, paths, ids). */
export function KeyValueList({ items, className, columns = 1 }: { items: KeyValueItem[]; className?: string; columns?: 1 | 2 }) {
  const visible = items.filter((item) => item.value !== null && item.value !== undefined && item.value !== "");
  if (visible.length === 0) return null;
  return (
    <dl className={cn("grid gap-x-8 gap-y-3", columns === 2 && "sm:grid-cols-2", className)}>
      {visible.map((item) => (
        <div key={item.label} className="flex flex-col gap-0.5 border-b border-separator pb-2.5 last:border-b-0 sm:flex-row sm:items-baseline sm:justify-between sm:gap-4">
          <dt className="shrink-0 text-label text-muted">{item.label}</dt>
          <dd className={cn("min-w-0 text-body-sm text-foreground sm:text-right", item.mono && "font-mono text-label break-all")}>{item.value}</dd>
        </div>
      ))}
    </dl>
  );
}
