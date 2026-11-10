import { useNavigate } from "@tanstack/react-router";
import { usePaletteCommands, type PaletteCommand } from "glass-ux/command-palette";
import { IconChartLine, IconLock, IconUser } from "glass-ux/icons";
import { useMemo } from "react";

import { useOpenProfile } from "@/features/users/profileDialog";
import { useIsAdmin } from "@/lib/auth/useSession";
import { PRIMARY_NAV } from "./nav";

/** Navigation commands exist only in the authenticated shell. */
export function ShellCommands() {
  const navigate = useNavigate();
  const openProfile = useOpenProfile();
  const isAdmin = useIsAdmin();
  const commands = useMemo<PaletteCommand[]>(() => [
    ...PRIMARY_NAV.filter((item) => !item.adminOnly || isAdmin).map((item) => ({
      id: `navigate.${item.key}`,
      label: item.label,
      group: "Navigation",
      icon: <item.icon />,
      run: () => navigate({ to: item.href as never }),
    })),
    // Your profile is a dialog over the page you are on (design.md Q147).
    { id: "profile.open", label: "Your profile", group: "Account", icon: <IconUser />, run: () => openProfile("profile") },
    { id: "profile.statistics", label: "Your statistics", group: "Account", icon: <IconChartLine />, run: () => openProfile("statistics") },
    { id: "profile.security", label: "Password and two-factor sign-in", group: "Account", icon: <IconLock />, run: () => openProfile("security") },
    ...(isAdmin ? [{ id: "navigate.server-statistics", label: "Server statistics", group: "Navigation", icon: <IconChartLine />, run: () => navigate({ to: "/settings/statistics" }) }] : []),
  ], [isAdmin, navigate, openProfile]);
  usePaletteCommands(commands);
  return null;
}
