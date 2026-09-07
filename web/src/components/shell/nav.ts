import { IconBell, IconDownload, IconHome, IconListSearch, IconSearch, IconSettings, IconStack2, type Icon as TablerIcon } from "@tabler/icons-react";

export interface NavItem {
  key: string;
  label: string;
  href: string;
  icon: TablerIcon;
  /** Show on the phone tab bar. */
  mobile?: boolean;
  /** Only admins see it. */
  adminOnly?: boolean;
  badge?: "downloads" | "notifications";
}

/** Primary destinations. Order matters: it is the order on the rail and the tab bar. */
export const PRIMARY_NAV: NavItem[] = [
  { key: "home", label: "Home", href: "/", icon: IconHome, mobile: true },
  { key: "libraries", label: "Libraries", href: "/libraries", icon: IconStack2, mobile: true },
  { key: "search", label: "Search", href: "/search", icon: IconSearch, mobile: true },
  { key: "wanted", label: "Wanted", href: "/wanted", icon: IconListSearch, mobile: true },
  { key: "downloads", label: "Downloads", href: "/downloads", icon: IconDownload, mobile: true, badge: "downloads" },
  { key: "activity", label: "Activity", href: "/activity", icon: IconBell, badge: "notifications" },
  { key: "settings", label: "Settings", href: "/settings", icon: IconSettings, mobile: true },
];

export function isNavActive(href: string, pathname: string): boolean {
  if (href === "/") return pathname === "/";
  return pathname === href || pathname.startsWith(`${href}/`);
}
