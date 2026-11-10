import { IconArrowUpCircle } from "@tabler/icons-react";
import { Outlet, useRouterState } from "@tanstack/react-router";
import { IconCast, IconCloudDownload, IconDatabase, IconFileText, IconFolder, IconGlobe, IconInfo, IconLayers, IconPlug, IconSliders, IconTag, IconUsers } from "glass-ux/icons";

import { PageScroller } from "@/components/shell/PageScroller";
import { PageHeader, SideTabs, type TabItem } from "@/components/ui";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { cn } from "@/lib/utils";
import { useIsAdmin } from "@/lib/auth/useSession";

const SECTIONS: Array<TabItem & { adminOnly?: boolean }> = [
  { key: "general", label: "General", href: "/settings/general", icon: IconSliders },
  { key: "libraries", label: "Libraries", href: "/settings/libraries", icon: IconLayers, adminOnly: true },
  { key: "network-shares", label: "Network shares", href: "/settings/network-shares", icon: ENTITY_KINDS.networkServer.icon, adminOnly: true },
  { key: "metadata", label: "Metadata", href: "/settings/metadata", icon: IconGlobe, adminOnly: true },
  { key: "sources", label: "Sources", href: "/settings/sources", icon: IconPlug, adminOnly: true },
  { key: "quality", label: "Quality", href: "/settings/quality", icon: IconTag, adminOnly: true },
  { key: "organization", label: "Organization", href: "/settings/organization", icon: IconFolder, adminOnly: true },
  { key: "downloads", label: "Downloads", href: "/settings/downloads", icon: IconCloudDownload, adminOnly: true },
  { key: "casting", label: "Casting", href: "/settings/casting", icon: IconCast },
  { key: "users", label: "Users", href: "/settings/users", icon: IconUsers, adminOnly: true },
  { key: "backup", label: "Backup", href: "/settings/backup", icon: IconDatabase, adminOnly: true },
  { key: "logs", label: "Logs", href: "/settings/logs", icon: IconFileText, adminOnly: true },
  { key: "updates", label: "Updates", href: "/settings/updates", icon: IconArrowUpCircle, adminOnly: true },
  { key: "about", label: "About", href: "/settings/about", icon: IconInfo },
];

export function SettingsLayout() {
  const isAdmin = useIsAdmin();
  const tablePane = useRouterState({ select: (state) => state.location.pathname === "/settings/logs" });
  const items = SECTIONS.filter((section) => !section.adminOnly || isAdmin);
  return (
    <div data-print-flow className="flex min-h-0 flex-1 flex-col page-gutter pt-2">
      {/* The breadcrumb names the page; the heading stays for assistive technology and print. */}
      <PageHeader className="sr-only" titleVisibility="accessible" title="Settings" />
      <SideTabs ariaLabel="Settings sections" items={items} className="min-h-0 flex-1">
        <PageScroller data-print-flow scrolling={!tablePane} className={cn("fa-focus-room h-full pb-[calc(var(--space-8)+var(--focus-reach))]", tablePane && "flex flex-col")}>
          <Outlet />
        </PageScroller>
      </SideTabs>
    </div>
  );
}

export const SETTINGS_SECTIONS = SECTIONS;
