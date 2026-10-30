import { Outlet, useRouterState } from "@tanstack/react-router";
import {
  IconAdjustments,
  IconCast,
  IconCloudDownload,
  IconDatabase,
  IconFileText,
  IconFolders,
  IconInfoCircle,
  IconPlug,
  IconStack2,
  IconTags,
  IconUsers,
  IconWorld,
} from "@tabler/icons-react";

import { PageHeader, SideTabs, type TabItem } from "@/components/ui";
import { cn } from "@/lib/utils";
import { useIsAdmin } from "@/lib/auth/useSession";

const SECTIONS: Array<TabItem & { adminOnly?: boolean }> = [
  { key: "general", label: "General", href: "/settings/general", icon: IconAdjustments },
  { key: "libraries", label: "Libraries", href: "/settings/libraries", icon: IconStack2, adminOnly: true },
  { key: "metadata", label: "Metadata", href: "/settings/metadata", icon: IconWorld, adminOnly: true },
  { key: "sources", label: "Sources", href: "/settings/sources", icon: IconPlug, adminOnly: true },
  { key: "quality", label: "Quality", href: "/settings/quality", icon: IconTags, adminOnly: true },
  { key: "organization", label: "Organization", href: "/settings/organization", icon: IconFolders, adminOnly: true },
  { key: "downloads", label: "Downloads", href: "/settings/downloads", icon: IconCloudDownload, adminOnly: true },
  { key: "casting", label: "Casting", href: "/settings/casting", icon: IconCast },
  { key: "users", label: "Users", href: "/settings/users", icon: IconUsers, adminOnly: true },
  { key: "backup", label: "Backup", href: "/settings/backup", icon: IconDatabase, adminOnly: true },
  { key: "logs", label: "Logs", href: "/settings/logs", icon: IconFileText, adminOnly: true },
  { key: "about", label: "About", href: "/settings/about", icon: IconInfoCircle },
];

export function SettingsLayout() {
  const isAdmin = useIsAdmin();
  const tablePane = useRouterState({ select: (state) => state.location.pathname === "/settings/logs" });
  const items = SECTIONS.filter((section) => !section.adminOnly || isAdmin);
  return (
    <div data-print-flow className="flex min-h-0 flex-1 flex-col gap-6 page-gutter pt-8">
      <PageHeader title="Settings" />
      <SideTabs ariaLabel="Settings sections" items={items} className="min-h-0 flex-1">
        <div data-print-flow className={cn("fa-focus-room scrollbar-thin h-full min-h-0 pb-[calc(var(--space-8)+var(--focus-reach))]", tablePane ? "flex flex-col overflow-hidden" : "overflow-y-auto")}>
          <Outlet />
        </div>
      </SideTabs>
    </div>
  );
}

export const SETTINGS_SECTIONS = SECTIONS;
