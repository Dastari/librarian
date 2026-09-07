import type { ReactNode } from "react";

import { isPresent } from "@/lib/utils";

/** Renders inline items separated by middle dots, skipping empty values. */
export function MetaLine({ items, className }: { items: Array<ReactNode | null | undefined | false>; className?: string }) {
  const present = items.filter((item) => isPresent(item) && item !== false && item !== "");
  return (
    <span className={className}>
      {present.map((item, index) => (
        <span key={index}>
          {index > 0 ? <span className="mx-1.5 text-muted/70">·</span> : null}
          {item}
        </span>
      ))}
    </span>
  );
}
