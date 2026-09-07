import { Avatar, Dropdown } from "@heroui/react";
import { useNavigate } from "@tanstack/react-router";
import { IconDeviceTv, IconLogout, IconPalette, IconSettings, IconSunMoon, IconUserCircle } from "@tabler/icons-react";
import type { Key } from "react";

import { session } from "@/lib/auth/session";
import { useSession } from "@/lib/auth/useSession";
import { useInputMode } from "@/lib/input-mode";
import { THEMES, useTheme, type ThemePreference } from "@/lib/theme";

function initials(name: string | null | undefined, fallback: string): string {
  const source = (name ?? fallback).trim();
  const parts = source.split(/\s+/).filter(Boolean);
  const first = parts[0]?.charAt(0) ?? "";
  const second = parts[1]?.charAt(0) ?? "";
  return (first + second).toUpperCase() || "?";
}

export function UserMenu() {
  const { user } = useSession();
  const { preference, setPreference } = useTheme();
  const { forced, setForced } = useInputMode();
  const navigate = useNavigate();
  if (!user) return null;

  const onAction = (key: Key) => {
    const value = String(key);
    if (value.startsWith("theme-") && value !== "theme-system") {
      setPreference(value.slice(6) as ThemePreference);
      return;
    }
    switch (key) {
      case "settings":
        void navigate({ to: "/settings" });
        break;
      case "theme-system":
        setPreference("system");
        break;
      case "tv-mode":
        setForced(forced === "tv" ? null : "tv");
        break;
      case "logout":
        void session.logout().then(() => navigate({ to: "/login" }));
        break;
    }
  };

  return (
    <Dropdown>
      <Dropdown.Trigger aria-label="Account" className="nav-focus rounded-full" data-focusable>
        <Avatar size="sm" color="accent" aria-hidden>
          <Avatar.Fallback>{initials(user.displayName, user.username)}</Avatar.Fallback>
        </Avatar>
      </Dropdown.Trigger>
      <Dropdown.Popover placement="bottom end" className="glass-surface min-w-60">
        <Dropdown.Menu aria-label="Account menu" onAction={onAction}>
          <Dropdown.Section>
            <Dropdown.Item id="profile" textValue={user.username} isDisabled>
              <span className="flex items-center gap-3">
                <IconUserCircle size={18} className="text-muted" />
                <span className="flex flex-col">
                  <span className="text-body-sm text-foreground">{user.displayName ?? user.username}</span>
                  <span className="text-label-sm text-muted">{user.email ?? user.role}</span>
                </span>
              </span>
            </Dropdown.Item>
          </Dropdown.Section>
          <Dropdown.Section>
            {THEMES.map((theme) => (
              <Dropdown.Item key={theme.id} id={`theme-${theme.id}`} textValue={theme.label}>
                <IconPalette size={18} style={{ color: theme.swatch[1] }} /> {theme.label} {preference === theme.id ? <Dropdown.ItemIndicator /> : null}
              </Dropdown.Item>
            ))}
            <Dropdown.Item id="theme-system" textValue="Match system">
              <IconSunMoon size={18} /> Match system {preference === "system" ? <Dropdown.ItemIndicator /> : null}
            </Dropdown.Item>
            <Dropdown.Item id="tv-mode" textValue="TV mode">
              <IconDeviceTv size={18} /> TV mode {forced === "tv" ? <Dropdown.ItemIndicator /> : null}
            </Dropdown.Item>
          </Dropdown.Section>
          <Dropdown.Section>
            <Dropdown.Item id="settings" textValue="Settings">
              <IconSettings size={18} /> Settings
            </Dropdown.Item>
            <Dropdown.Item id="logout" textValue="Sign out" variant="danger">
              <IconLogout size={18} /> Sign out
            </Dropdown.Item>
          </Dropdown.Section>
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown>
  );
}
