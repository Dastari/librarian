import { useCommandPalette } from "glass-ux/command-palette";
import { Tooltip } from "glass-ux/tooltip";
import { BrandMark, Button } from "@/components/ui";
import { Link, useNavigate, useRouterState } from "@tanstack/react-router";
import { IconArrowLeft, IconBell, IconCommand, IconSearch } from "glass-ux/icons";

import { Surface } from "glass-ux/surface";
import { cn } from "@/lib/utils";
import { CastButton } from "@/features/player/CastButton";

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
  const palette = useCommandPalette();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const counts = useShellCounts();
  const isRoot = pathname === "/" || /^\/(libraries|search|downloads|activity|settings)$/.test(pathname);

  return (
    <Surface as="header" material={overlay ? "control" : "chrome"} radius="none" padding="none" data-print="hide" data-nav-zone="top"
      className={cn(
        "sticky top-0 z-20 flex h-[calc(var(--topbar-height)+var(--safe-top))] items-center gap-2 pt-(--safe-top) transition-[background-color,backdrop-filter] duration-base",
        "page-gutter",
        overlay && "over-imagery",
      )}
    >
      <Link to="/" className="nav-focus me-1 flex items-center rounded-lg md:hidden" aria-label="Home" data-focusable>
        <BrandMark size={26} />
      </Link>
      {!isRoot ? (
        <Button variant="ghost" isIconOnly size="sm" aria-label="Back" onPress={() => window.history.back()} className="nav-focus hidden shrink-0 md:inline-flex" data-focusable>
          <IconArrowLeft className="rtl:rotate-180" />
        </Button>
      ) : null}
      <Breadcrumbs className="hidden flex-1 md:block" />
      <div className="flex-1 md:hidden" />

      <div className="flex shrink-0 items-center gap-1">
        <CastButton className="nav-focus" />
        <Tooltip content="Search">
          <Button variant="ghost" isIconOnly size="sm" aria-label="Search" onPress={() => void navigate({ to: "/search" })} className="nav-focus" data-focusable>
            <IconSearch />
          </Button>
        </Tooltip>
        <Tooltip content="Activity">
          <Button variant="ghost" isIconOnly size="sm" aria-label="Activity" onPress={() => void navigate({ to: "/activity" })} className="nav-focus relative" data-focusable>
            <IconBell />
            {counts.notifications > 0 ? <span className="absolute end-1.5 top-1.5 size-2 rounded-full bg-brand ring-2 ring-background" /> : null}
          </Button>
        </Tooltip>
        <Button variant="ghost" isIconOnly size="sm" aria-label="Commands" onPress={palette.open}><IconCommand /></Button>
        <UserMenu />
      </div>
    </Surface>
  );
}
