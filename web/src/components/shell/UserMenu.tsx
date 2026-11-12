import { useNavigate } from "@tanstack/react-router";
import { CountBadge } from "glass-ux/badge";
import { IconBell, IconChartLine, IconDisplays, IconLogOut, IconPalette, IconSettings, IconUser } from "glass-ux/icons";
import { Menu } from "glass-ux/menu";
import { AccountButton, Rail } from "glass-ux/shell";

import { useOpenProfile } from "@/features/users/profileDialog";
import { userAvatarSrc } from "@/features/users/UserIdentity";
import { session } from "@/lib/auth/session";
import { useSession } from "@/lib/auth/useSession";
import { useInputMode } from "@/lib/input-mode";
import { THEMES, useTheme, type ThemePreference } from "@/lib/theme";

import { useShellCounts } from "./useShellCounts";

/**
 * The signed-in account: its avatar carries the unread count, so there is no separate bell. In TV
 * mode it lives at the foot of the side rail as a rail item (`placement="rail"`), opening the same
 * menu beside the rail; a remote reaches it with the rest of the rail, and closing the menu
 * returns focus to that item.
 */
export function UserMenu({ placement = "top" }: { placement?: "top" | "rail" }) {
  const { user } = useSession();
  const { preference, setPreference } = useTheme();
  const { forced, setForced } = useInputMode();
  const counts = useShellCounts();
  const navigate = useNavigate();
  const openProfile = useOpenProfile();
  const rail = placement === "rail";
  if (!user) return null;

  return (
    <Menu.Root>
      <Menu.Trigger render={rail
        ? <Rail.Item className="nav-focus" label={user.displayName ?? user.username} icon={<IconUser />} badge={counts.notifications > 0 ? <CountBadge value={counts.notifications} overlay /> : undefined} data-focusable />
        : <AccountButton name={user.displayName ?? user.username} avatarSrc={userAvatarSrc(user, 96)} unread={counts.notifications} className="nav-focus" data-focusable />} />
      <Menu.Popup side={rail ? "right" : undefined} align={rail ? "end" : "end"} aria-label="Account menu">
        <Menu.Group>
          <Menu.GroupLabel>{user.displayName ?? user.username}</Menu.GroupLabel>
          <Menu.Item icon={<IconUser />} onClick={() => openProfile("profile")}>Profile</Menu.Item>
          <Menu.Item icon={<IconChartLine />} onClick={() => openProfile("statistics")}>Statistics</Menu.Item>
          <Menu.Item icon={<IconBell />} onClick={() => void navigate({ to: "/activity" })}>{counts.notifications > 0 ? `Activity (${counts.notifications})` : "Activity"}</Menu.Item>
          <Menu.Item icon={<IconSettings />} onClick={() => void navigate({ to: "/settings" })}>Settings</Menu.Item>
        </Menu.Group>
        <Menu.Submenu icon={<IconPalette />} label="Theme">
          <Menu.RadioGroup value={preference} onValueChange={(value) => setPreference(value as ThemePreference)}>
            {THEMES.map((theme) => <Menu.RadioItem key={theme.id} value={theme.id}>{theme.label}</Menu.RadioItem>)}
          </Menu.RadioGroup>
        </Menu.Submenu>
        <Menu.CheckboxItem checked={forced === "tv"} onCheckedChange={(checked) => setForced(checked ? "tv" : null)}><span className="flex items-center gap-2"><IconDisplays size="sm" />TV mode</span></Menu.CheckboxItem>
        <Menu.Separator />
        <Menu.Item tone="danger" icon={<IconLogOut />} onClick={() => void session.logout().then(() => navigate({ to: "/login" }))}>Sign out</Menu.Item>
      </Menu.Popup>
    </Menu.Root>
  );
}
