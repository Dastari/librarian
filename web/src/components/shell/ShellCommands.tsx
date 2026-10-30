import { useNavigate } from "@tanstack/react-router";
import { usePaletteCommands, type PaletteCommand } from "glass-ux/command-palette";
import { useMemo } from "react";

import { useIsAdmin } from "@/lib/auth/useSession";
import { PRIMARY_NAV } from "./nav";

/** Navigation commands exist only in the authenticated shell. */
export function ShellCommands() {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const commands = useMemo<PaletteCommand[]>(() => PRIMARY_NAV.filter((item) => !item.adminOnly || isAdmin).map((item) => ({
    id: `navigate.${item.key}`,
    label: item.label,
    group: "Navigation",
    icon: <item.icon />,
    run: () => navigate({ to: item.href as never }),
  })), [isAdmin, navigate]);
  usePaletteCommands(commands);
  return null;
}
