import type { ReactNode } from "react";

import { HeroBanner } from "@/components/ui";

interface DetailShellProps {
  hero: Parameters<typeof HeroBanner>[0];
  children: ReactNode;
}

/** Hero + content column used by every media detail page. */
export function DetailShell({ hero, children }: DetailShellProps) {
  return (
    <div className="flex flex-col gap-10 pb-12">
      <HeroBanner {...hero} />
      <div className="page-gutter flex flex-col gap-10">{children}</div>
    </div>
  );
}
