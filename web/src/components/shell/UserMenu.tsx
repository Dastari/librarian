import { useNavigate } from "@tanstack/react-router";
import { IconBell, IconDisplays, IconLogOut, IconPalette, IconSettings } from "glass-ux/icons";
import { Menu } from "glass-ux/menu";
import { AccountButton } from "glass-ux/shell";

import { session } from "@/lib/auth/session";
import { useSession } from "@/lib/auth/useSession";
import { useInputMode } from "@/lib/input-mode";
import { THEMES, useTheme, type ThemePreference } from "@/lib/theme";

import { useShellCounts } from "./useShellCounts";

/** The signed-in account: its avatar carries the unread count, so there is no separate bell. */
export function UserMenu() {
  const { user } = useSession();
  const { preference, setPreference } = useTheme();
  const { forced, setForced } = useInputMode();
  const counts = useShellCounts();
  const navigate = useNavigate();
  if (!user) return null;

  return (
    <Menu.Root>
      <Menu.Trigger render={<AccountButton name={user.displayName ?? user.username} unread={counts.notifications} className="nav-focus" data-focusable />} />
      <Menu.Popup align="end" aria-label="Account menu">
        <Menu.Group>
          <Menu.GroupLabel>{user.displayName ?? user.username}</Menu.GroupLabel>
          <Menu.Item icon={<IconBell />} onClick={() => void navigate({ to: "/activity" })}>{counts.notifications > 0 ? `Activity (${counts.notifications})` : "Activity"}</Menu.Item>
          <Menu.Item icon={<IconSettings />} onClick={() => void navigate({ to: "/settings" })}>Settings</Menu.Item>
        </Menu.Group>
        <Menu.Submenu icon={<IconPalette />} label="Theme">
          <Menu.RadioGroup value={preference} onValueChange={(value) => setPreference(value as ThemePreference)}>
            {THEMES.map((theme) => <Menu.RadioItem key={theme.id} value={theme.id}>{theme.label}</Menu.RadioItem>)}
            <Menu.RadioItem value="system">Match system</Menu.RadioItem>
          </Menu.RadioGroup>
        </Menu.Submenu>
        <Menu.CheckboxItem checked={forced === "tv"} onCheckedChange={(checked) => setForced(checked ? "tv" : null)}><span className="flex items-center gap-2"><IconDisplays size="sm" />TV mode</span></Menu.CheckboxItem>
        <Menu.Separator />
        <Menu.Item tone="danger" icon={<IconLogOut />} onClick={() => void session.logout().then(() => navigate({ to: "/login" }))}>Sign out</Menu.Item>
      </Menu.Popup>
    </Menu.Root>
  );
}
