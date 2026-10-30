import { Avatar } from "glass-ux/avatar";
import { Menu } from "glass-ux/menu";
import { useNavigate } from "@tanstack/react-router";
import { IconDeviceTv, IconLogout, IconPalette, IconSettings } from "@tabler/icons-react";

import { session } from "@/lib/auth/session";
import { useSession } from "@/lib/auth/useSession";
import { useInputMode } from "@/lib/input-mode";
import { THEMES, useTheme, type ThemePreference } from "@/lib/theme";

export function UserMenu() {
  const { user } = useSession();
  const { preference, setPreference } = useTheme();
  const { forced, setForced } = useInputMode();
  const navigate = useNavigate();
  if (!user) return null;

  return (
    <Menu.Root>
      <Menu.Trigger aria-label="Account" className="nav-focus rounded-full" data-focusable>
        <Avatar name={user.displayName ?? user.username} size="sm" decorative />
      </Menu.Trigger>
      <Menu.Popup align="end" aria-label="Account menu">
        <Menu.Group>
          <Menu.GroupLabel>{user.displayName ?? user.username}</Menu.GroupLabel>
          <Menu.Item icon={<IconSettings />} onClick={() => void navigate({ to: "/settings" })}>Settings</Menu.Item>
        </Menu.Group>
        <Menu.Submenu icon={<IconPalette />} label="Theme">
          <Menu.RadioGroup value={preference} onValueChange={(value) => setPreference(value as ThemePreference)}>
            {THEMES.map((theme) => <Menu.RadioItem key={theme.id} value={theme.id}>{theme.label}</Menu.RadioItem>)}
            <Menu.RadioItem value="system">Match system</Menu.RadioItem>
          </Menu.RadioGroup>
        </Menu.Submenu>
        <Menu.CheckboxItem checked={forced === "tv"} onCheckedChange={(checked) => setForced(checked ? "tv" : null)}><span className="flex items-center gap-2"><IconDeviceTv size={18} />TV mode</span></Menu.CheckboxItem>
        <Menu.Separator />
        <Menu.Item tone="danger" icon={<IconLogout />} onClick={() => void session.logout().then(() => navigate({ to: "/login" }))}>Sign out</Menu.Item>
      </Menu.Popup>
    </Menu.Root>
  );
}
