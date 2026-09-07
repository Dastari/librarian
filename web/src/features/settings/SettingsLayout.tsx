import { Outlet } from "@tanstack/react-router";
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
  const items = SECTIONS.filter((section) => !section.adminOnly || isAdmin);
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-6 page-gutter pt-8">
      <PageHeader title="Settings" />
      <div className="grid min-h-0 flex-1 gap-6 lg:grid-cols-[14rem_minmax(0,1fr)]">
        <SideTabs ariaLabel="Settings sections" items={items} className="lg:sticky lg:top-0 lg:self-start" />
        <div className="scrollbar-thin -mx-1 min-h-0 overflow-y-auto px-1 pb-8">
          <Outlet />
        </div>
      </div>
    </div>
  );
}

export const SETTINGS_SECTIONS = SECTIONS;
