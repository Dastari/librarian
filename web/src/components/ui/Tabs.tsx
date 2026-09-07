import { Link, useRouterState } from "@tanstack/react-router";
import type { Icon as TablerIcon } from "@tabler/icons-react";
import { useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";

import { cn } from "@/lib/utils";

export interface TabItem {
  key: string;
  label: ReactNode;
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
  size?: "sm" | "md";
}

function isActiveHref(href: string, pathname: string): boolean {
  return pathname === href || pathname.startsWith(`${href}/`);
}

/**
 * Horizontal tabs with a sliding indicator. With `href` on items this is a navigation strip
 * whose active state follows the URL; otherwise it is controlled through `selected`/`onSelect`.
 * Arrow keys move between tabs (WAI-ARIA tabs pattern) and the strip scrolls on phones.
 */
export function SegmentTabs({ items, selected, onSelect, className, ariaLabel, size = "md" }: SegmentTabsProps) {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const isNav = items.some((item) => item.href);
  const activeKey = isNav
    ? [...items].sort((a, b) => (b.href?.length ?? 0) - (a.href?.length ?? 0)).find((item) => item.href && isActiveHref(item.href, pathname))?.key
    : selected;

  const listRef = useRef<HTMLDivElement>(null);
  const [indicator, setIndicator] = useState<{ left: number; width: number } | null>(null);

  useLayoutEffect(() => {
    const list = listRef.current;
    if (!list) return;
    const measure = () => {
      const active = list.querySelector<HTMLElement>("[data-active='true']");
      if (!active) {
        setIndicator(null);
        return;
      }
      setIndicator({ left: active.offsetLeft, width: active.offsetWidth });
      active.scrollIntoView({ inline: "nearest", block: "nearest" });
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(list);
    return () => observer.disconnect();
  }, [activeKey, items]);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    const enabled = items.filter((item) => !item.disabled);
    const current = enabled.findIndex((item) => item.key === (document.activeElement as HTMLElement | null)?.dataset.tabKey);
    const next = enabled[(current + (event.key === "ArrowRight" ? 1 : -1) + enabled.length) % enabled.length];
    if (!next) return;
    event.preventDefault();
    listRef.current?.querySelector<HTMLElement>(`[data-tab-key='${next.key}']`)?.focus();
    if (!isNav) onSelect?.(next.key);
  };

  return (
    <div className={cn("scrollbar-none -mx-1 overflow-x-auto px-1", className)}>
      <div ref={listRef} role="tablist" aria-label={ariaLabel} onKeyDown={onKeyDown} data-spatial-ignore className="relative flex w-max min-w-full gap-1 border-b border-separator">
        {indicator ? <span aria-hidden className="absolute bottom-0 h-0.5 rounded-full bg-brand transition-[left,width] duration-base ease-fluid" style={{ left: indicator.left, width: indicator.width }} /> : null}
        {items.map((item) => {
          const active = item.key === activeKey;
          const Icon = item.icon;
          const content = (
            <>
              {Icon ? <Icon size={size === "sm" ? 16 : 18} stroke={1.75} className={cn(active ? "text-brand" : "text-muted")} /> : null}
              <span>{item.label}</span>
              {typeof item.count === "number" ? <span className={cn("text-numeric rounded-md px-1.5 py-0.5 text-label-sm", active ? "bg-brand-soft text-foreground" : "bg-surface-tertiary text-muted")}>{item.count}</span> : null}
            </>
          );
          const classes = cn(
            "nav-focus inline-flex shrink-0 items-center gap-2 whitespace-nowrap rounded-t-lg border-b-2 border-transparent px-3 transition-colors duration-fast",
            size === "sm" ? "h-9 text-label" : "h-11 text-body-sm font-medium",
            active ? "text-foreground" : "text-muted hover:text-foreground",
            item.disabled && "pointer-events-none opacity-40",
          );
          if (item.href) {
            return (
              <Link key={item.key} to={item.href as never} role="tab" aria-selected={active} tabIndex={active ? 0 : -1} data-tab-key={item.key} data-active={active} data-focusable className={classes}>
                {content}
              </Link>
            );
          }
          return (
            <button key={item.key} type="button" role="tab" aria-selected={active} tabIndex={active ? 0 : -1} data-tab-key={item.key} data-active={active} data-focusable disabled={item.disabled} onClick={() => onSelect?.(item.key)} className={classes}>
              {content}
            </button>
          );
        })}
      </div>
    </div>
  );
}

interface SideTabsProps {
  items: TabItem[];
  className?: string;
  ariaLabel: string;
}

/**
 * Vertical navigation for settings-style pages. Collapses to a horizontal strip below `lg`.
 * Every item is a router link; the active state follows the URL.
 */
export function SideTabs({ items, className, ariaLabel }: SideTabsProps) {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  return (
    <nav aria-label={ariaLabel} className={cn("scrollbar-none -mx-1 flex gap-1 overflow-x-auto px-1 lg:mx-0 lg:flex-col lg:overflow-visible lg:px-0", className)}>
      {items.map((item) => {
        const Icon = item.icon;
        const active = item.href ? isActiveHref(item.href, pathname) : false;
        return (
          <Link
            key={item.key}
            to={item.href as never}
            data-focusable
            aria-current={active ? "page" : undefined}
            className={cn(
              "nav-focus flex shrink-0 items-center gap-2.5 rounded-lg px-3 py-2 text-body-sm transition-colors duration-fast",
              active ? "bg-brand-soft text-foreground" : "text-muted hover:bg-surface-hover hover:text-foreground",
            )}
          >
            {Icon ? <Icon size={18} stroke={1.75} className={cn(active ? "text-brand" : "text-muted")} /> : null}
            <span className="whitespace-nowrap">{item.label}</span>
            {typeof item.count === "number" ? <span className="text-numeric ml-auto text-label-sm text-muted">{item.count}</span> : null}
          </Link>
        );
      })}
    </nav>
  );
}
