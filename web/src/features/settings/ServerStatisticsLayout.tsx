import { Outlet } from "@tanstack/react-router";
import { IconChartLine, IconHistory, IconPlay, IconUsers } from "glass-ux/icons";

import { SegmentTabs, type TabItem } from "@/components/ui";
import { ENTITY_KINDS } from "@/lib/entities/kinds";

export const SERVER_STATISTICS_TABS: TabItem[] = [
  { key: "now-playing", label: "Now playing", href: "/settings/statistics", icon: IconPlay },
  { key: "overview", label: "Statistics", href: "/settings/statistics/overview", icon: IconChartLine },
  { key: "people", label: "People", href: "/settings/statistics/people", icon: IconUsers },
  { key: "history", label: "History", href: "/settings/statistics/history", icon: IconHistory },
  { key: "devices", label: "Devices", href: "/settings/statistics/devices", icon: ENTITY_KINDS.device.icon },
];

/** Settings paths whose table owns its scrolling: the settings pane is bounded for them. */
export const SERVER_STATISTICS_TABLE_PATHS = ["/settings/statistics/people", "/settings/statistics/history", "/settings/statistics/devices"];

/**
 * Everyone's playback, for administrators (design.md Q150): what is playing now with Stop,
 * everyone's statistics, a breakdown per person, the full play history (who played what, when,
 * where, how and for how long) and every device. Your own live in your profile.
 */
export function ServerStatisticsLayout() {
  return (
    <div data-print-flow className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
      <div className="shrink-0" data-nav-section>
        <SegmentTabs ariaLabel="Server statistics views" items={SERVER_STATISTICS_TABS} />
      </div>
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <Outlet />
      </div>
    </div>
  );
}
