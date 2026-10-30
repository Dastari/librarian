import type { ReactNode } from "react";
import type { Icon as TablerIcon } from "@tabler/icons-react";
import { RouteTabs, Tabs, type RouteTabItem, type TabItem as GlassTabItem } from "glass-ux/tabs";
import { RouteVerticalTabs } from "glass-ux/vertical-tabs";

import { cn } from "@/lib/utils";

export interface TabItem {
  key: string;
  label: string;
  icon?: TablerIcon;
  count?: number | null;
  /** Resolved href. When set the tab navigates and the active state follows the URL. */
  href?: string;
  disabled?: boolean;
}

interface SegmentTabsProps {
  items: TabItem[];
  selected?: string;
  onSelect?: (key: string) => void;
  className?: string;
  ariaLabel: string;
  /** Retained for callers; Glass tabs use the shared appearance density. */
  size?: "sm" | "md";
}

function toTabItem(item: TabItem): GlassTabItem {
  const Icon = item.icon;
  return {
    value: item.key,
    label: item.label,
    icon: Icon ? <Icon /> : undefined,
    count: item.count ?? undefined,
    disabled: item.disabled,
  };
}

function toRouteItems(items: TabItem[]): RouteTabItem[] {
  return items.filter((item) => item.href).map((item) => ({
    ...toTabItem(item),
    link: { to: item.href as never },
  }));
}

/** Compatibility adapter: routed sections are links; local choices use Base UI tabs. */
export function SegmentTabs({ items, selected, onSelect, className, ariaLabel }: SegmentTabsProps) {
  if (items.some((item) => item.href)) {
    return (
      <div className={cn("min-w-0 shrink-0", className)}>
        <RouteTabs aria-label={ariaLabel} items={toRouteItems(items)} />
      </div>
    );
  }
  return (
    <Tabs.Root value={selected} onValueChange={onSelect} className={cn("min-w-0 shrink-0", className)}>
      <Tabs.List aria-label={ariaLabel} items={items.map(toTabItem)} />
    </Tabs.Root>
  );
}

interface SideTabsProps {
  children?: ReactNode;
  items: TabItem[];
  className?: string;
  ariaLabel: string;
}

/** Routed settings navigation; the library owns focus and container-aware collapse. */
export function SideTabs({ items, className, ariaLabel, children }: SideTabsProps) {
  return (
    <div className={cn("min-w-0", className)}>
      <RouteVerticalTabs aria-label={ariaLabel} items={toRouteItems(items)}>{children}</RouteVerticalTabs>
    </div>
  );
}
