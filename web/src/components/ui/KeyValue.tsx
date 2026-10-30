import { FactList } from "glass-ux/fact-list";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export interface KeyValueItem {
  label: string;
  value: ReactNode;
  mono?: boolean;
}

export function KeyValueList({ items, className, columns = 1 }: { items: KeyValueItem[]; className?: string; columns?: 1 | 2 }) {
  const visible = items.filter((item) => item.value !== null && item.value !== undefined && item.value !== "");
  if (visible.length === 0) return null;
  return <FactList className={cn(columns === 2 && "@min-[36rem]:grid @min-[36rem]:grid-cols-2", className)} facts={visible.map(({ label, value, mono }) => ({ label, value: mono ? <span className="font-mono break-all">{value}</span> : value }))} />;
}
