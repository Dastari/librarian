import { Tooltip } from "@heroui/react";
import { BrandMark, Button } from "@/components/ui";
import { Link, useNavigate, useRouterState } from "@tanstack/react-router";
import { IconArrowLeft, IconBell, IconSearch } from "@tabler/icons-react";
import { useRef } from "react";

import { useRefraction } from "@/lib/refraction";
import { cn } from "@/lib/utils";

import { Breadcrumbs } from "./Breadcrumbs";
import { UserMenu } from "./UserMenu";
import { useShellCounts } from "./useShellCounts";

interface TopBarProps {
  /** Over hero art the bar is fully transparent until the page scrolls. */
  overlay?: boolean;
}

/**
 * Top bar: back button (when not at a root destination), breadcrumbs, search, activity and
 * account. It floats over hero banners and becomes glass when the page scrolls.
 */
export function TopBar({ overlay }: TopBarProps) {
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const counts = useShellCounts();
  const isRoot = pathname === "/" || /^\/(libraries|search|downloads|activity|settings)$/.test(pathname);
  const ref = useRef<HTMLElement>(null);
  const filter = useRefraction(ref, { depth: 10, strength: 24, blur: 9, brightness: 1.02, saturate: 1.3 }, !overlay);

  return (
    <header
      ref={ref}
      style={filter ? { backdropFilter: filter, WebkitBackdropFilter: filter } : undefined}
      className={cn(
        "sticky top-0 z-20 flex h-[calc(var(--topbar-height)+var(--safe-top))] items-center gap-2 pt-(--safe-top) transition-[background-color,backdrop-filter] duration-base",
        "page-gutter",
        overlay ? "bg-gradient-to-b from-background/70 to-transparent" : "topbar-glass",
      )}
    >
      <Link to="/" className="nav-focus mr-1 flex items-center rounded-lg md:hidden" aria-label="Home" data-focusable>
        <BrandMark size={26} />
      </Link>
      {!isRoot ? (
        <Button variant="ghost" isIconOnly size="sm" aria-label="Back" onPress={() => window.history.back()} className="nav-focus hidden shrink-0 md:inline-flex" data-focusable>
          <IconArrowLeft size={18} />
        </Button>
      ) : null}
      <Breadcrumbs className="hidden flex-1 md:block" />
      <div className="flex-1 md:hidden" />

      <div className="flex shrink-0 items-center gap-1">
        <Tooltip delay={400}>
          <Button variant="ghost" isIconOnly size="sm" aria-label="Search" onPress={() => void navigate({ to: "/search" })} className="nav-focus" data-focusable>
            <IconSearch size={18} />
          </Button>
          <Tooltip.Content>Search</Tooltip.Content>
        </Tooltip>
        <Tooltip delay={400}>
          <Button variant="ghost" isIconOnly size="sm" aria-label="Activity" onPress={() => void navigate({ to: "/activity" })} className="nav-focus relative" data-focusable>
            <IconBell size={18} />
            {counts.notifications > 0 ? <span className="absolute right-1.5 top-1.5 size-2 rounded-full bg-brand ring-2 ring-background" /> : null}
          </Button>
          <Tooltip.Content>Activity</Tooltip.Content>
        </Tooltip>
        <UserMenu />
      </div>
    </header>
  );
}
