import { BrandMark as GlassBrandMark } from "glass-ux/brand";
import type { CSSProperties } from "react";

import "@/lib/glass/brand";
import { cn } from "@/lib/utils";

/** Preserve the application's mark API while Glass UX owns its rendering and palette. */
export function BrandMark({ className, size = 32, style, title }: { className?: string; size?: number | string; style?: CSSProperties; title?: string }) {
  return <GlassBrandMark mark="product" label={title} width={size} height={size} className={cn("shrink-0", className)} style={style} />;
}
