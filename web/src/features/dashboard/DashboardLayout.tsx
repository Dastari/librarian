import { Outlet, useRouterState } from "@tanstack/react-router";
import { IconChartLine, IconHistory, IconPlay } from "glass-ux/icons";

import { PageScroller } from "@/components/shell/PageScroller";
import { PageHeader, SegmentTabs, type TabItem } from "@/components/ui";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { cn } from "@/lib/utils";

export const DASHBOARD_TABS: TabItem[] = [
  { key: "now-playing", label: "Now playing", href: "/dashboard", icon: IconPlay },
  { key: "statistics", label: "Statistics", href: "/dashboard/statistics", icon: IconChartLine },
  { key: "history", label: "History", href: "/dashboard/history", icon: IconHistory },
  { key: "devices", label: "Devices", href: "/dashboard/devices", icon: ENTITY_KINDS.device.icon },
];

/** The tables own their scrolling; Now playing and Statistics scroll as a page. */
const TABLE_ROUTES = new Set(["/_app/dashboard/history", "/_app/dashboard/devices"]);

/**
 * Playback across the server, as in Plex's dashboard: what is playing now, statistics, the play
 * history and the devices that sign in. The tabs stay put; only the view under them scrolls.
 */
export function DashboardLayout() {
  const table = useRouterState({ select: (state) => state.matches.some((match) => TABLE_ROUTES.has(match.routeId)) });
  return (
    <div data-print-flow className="flex min-h-0 min-w-0 flex-1 flex-col">
      <PageHeader className="sr-only" titleVisibility="accessible" title="Dashboard" />
      <div className="page-gutter shrink-0 pt-2 pb-3" data-nav-section>
        <SegmentTabs ariaLabel="Dashboard views" items={DASHBOARD_TABS} />
      </div>
      <PageScroller
        data-print-flow
        scrolling={!table}
        className={cn("page-gutter flex min-h-0 flex-1 flex-col", table ? "fa-focus-room pb-[calc(var(--space-3)+var(--focus-reach))]" : "pb-[calc(var(--space-8)+var(--focus-reach))]")}
      >
        <Outlet />
      </PageScroller>
    </div>
  );
}
