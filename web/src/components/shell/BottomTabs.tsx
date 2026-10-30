import { Surface } from "glass-ux/surface";
import { IconMoreHorizontal } from "glass-ux/icons";
import { Menu } from "glass-ux/menu";
import { Link, useNavigate, useRouterState } from "@tanstack/react-router";

import { cn } from "@/lib/utils";

import { PRIMARY_NAV, isNavActive } from "./nav";
import { useShellCounts } from "./useShellCounts";

/** Phone navigation. Sits above the home indicator; hidden on md+ where the rail takes over. */
export function BottomTabs() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const navigate = useNavigate();
  const counts = useShellCounts();
  const primary = PRIMARY_NAV.filter((item) => ["home", "libraries", "search", "downloads"].includes(item.key));
  const secondary = PRIMARY_NAV.filter((item) => !primary.includes(item));
  const moreActive = secondary.some((item) => isNavActive(item.href, pathname));
  return (
    <Surface as="nav" material="chrome" radius="none" padding="none"
      aria-label="Primary"
      data-print="hide"
      className="fixed inset-x-0 bottom-0 z-30 rounded-none border-0 border-t flex h-[calc(var(--tabbar-height)+var(--safe-bottom))] items-start justify-around border-t border-glass-border px-2 pt-1.5 pb-(--safe-bottom) md:hidden"
    >
      {primary.map((item) => {
        const active = isNavActive(item.href, pathname);
        const badge = item.badge === "downloads" ? counts.downloads : item.badge === "notifications" ? counts.notifications : 0;
        return (
          <Link
            key={item.key}
            to={item.href as never}
            data-focusable
            aria-current={active ? "page" : undefined}
            className={cn(
              "nav-focus relative flex min-w-0 flex-1 flex-col items-center gap-0.5 rounded-xl px-1 py-1 text-label-sm transition-colors duration-fast",
              active ? "text-brand" : "text-muted",
            )}
          >
            <item.icon size={22} stroke={active ? 2 : 1.75} />
            <span>{item.label}</span>
            {badge > 0 ? (
              <span className="text-numeric absolute end-1 top-0 grid min-w-4 place-items-center rounded-full bg-brand px-1 text-[10px] leading-4 text-brand-foreground">{badge > 99 ? "99+" : badge}</span>
            ) : null}
          </Link>
        );
      })}
      <Menu.Root>
        <Menu.Trigger aria-label="More destinations" data-focusable
          className={cn("nav-focus flex min-w-0 flex-1 flex-col items-center gap-0.5 rounded-xl px-1 py-1 text-label-sm", moreActive ? "text-brand" : "text-muted")}>
          <IconMoreHorizontal className="size-[22px]" />
          <span>More</span>
        </Menu.Trigger>
        <Menu.Popup side="top" align="end" aria-label="More destinations">
          {secondary.map((item) => <Menu.Item key={item.key} icon={<item.icon size={20} />}
            onClick={() => void navigate({ to: item.href as never })}>
            {item.label}
          </Menu.Item>)}
        </Menu.Popup>
      </Menu.Root>
    </Surface>
  );
}
