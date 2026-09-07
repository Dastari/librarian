import { Link, useRouterState } from "@tanstack/react-router";

import { cn } from "@/lib/utils";

import { PRIMARY_NAV, isNavActive } from "./nav";
import { useShellCounts } from "./useShellCounts";

/** Phone navigation. Sits above the home indicator; hidden on md+ where the rail takes over. */
export function BottomTabs() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const counts = useShellCounts();
  return (
    <nav
      aria-label="Primary"
      className="glass-chrome fixed inset-x-0 bottom-0 z-30 rounded-none border-0 border-t flex h-[calc(var(--tabbar-height)+var(--safe-bottom))] items-start justify-around border-t border-glass-border px-2 pt-1.5 pb-(--safe-bottom) md:hidden"
    >
      {PRIMARY_NAV.filter((item) => item.mobile).map((item) => {
        const active = isNavActive(item.href, pathname);
        const badge = item.badge === "downloads" ? counts.downloads : item.badge === "notifications" ? counts.notifications : 0;
        return (
          <Link
            key={item.key}
            to={item.href as never}
            data-focusable
            aria-current={active ? "page" : undefined}
            className={cn(
              "nav-focus relative flex min-w-14 flex-col items-center gap-0.5 rounded-xl px-2 py-1 text-label-sm transition-colors duration-fast",
              active ? "text-brand" : "text-muted",
            )}
          >
            <item.icon size={22} stroke={active ? 2 : 1.75} />
            <span>{item.label}</span>
            {badge > 0 ? (
              <span className="text-numeric absolute right-1 top-0 grid min-w-4 place-items-center rounded-full bg-brand px-1 text-[10px] leading-4 text-brand-foreground">{badge > 99 ? "99+" : badge}</span>
            ) : null}
          </Link>
        );
      })}
    </nav>
  );
}
