import { useNavigate, useRouterState } from "@tanstack/react-router";
import { Button } from "glass-ux/button";
import { useCommandPalette } from "glass-ux/command-palette";
import { IconArrowLeft, IconCommand } from "glass-ux/icons";
import { SearchTrigger, TopBar as GlassTopBar } from "glass-ux/shell";
import { Tooltip } from "glass-ux/tooltip";

import { CastButton } from "@/features/player/CastButton";
import { requestSearchFocus } from "@/features/search/searchFocus";
import { cn } from "@/lib/utils";

import { Breadcrumbs } from "./Breadcrumbs";
import { useHasShellArt } from "./shellArt";
import { UserMenu } from "./UserMenu";

/**
 * The top bar sits on the canvas with no material of its own, so a title's artwork shows
 * straight through it: where you are on the left, everything you can do on the right.
 */
export function TopBar() {
  const navigate = useNavigate();
  const palette = useCommandPalette();
  const overArt = useHasShellArt();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  // Already searching: going "to search" would clear the query and leave focus on this button.
  const openSearch = () => {
    if (pathname === "/search") requestSearchFocus();
    else void navigate({ to: "/search" });
  };
  const isRoot = pathname === "/" || /^\/(libraries|search|lists|guide|wanted|downloads|activity|settings)$/.test(pathname);

  return (
    <GlassTopBar
      className={cn("app-top", overArt && "over-imagery")}
      data-nav-zone="top"
      data-print="hide"
      breadcrumb={
        <div className="flex min-w-0 flex-1 items-center gap-1">
          {!isRoot ? <Button variant="ghost" size="sm" shape="circle" icon={<IconArrowLeft className="rtl:rotate-180" />} aria-label="Back" onClick={() => window.history.back()} className="nav-focus shrink-0 max-md:hidden" data-focusable /> : null}
          <Breadcrumbs className="min-w-0 flex-1" />
        </div>
      }
      search={<SearchTrigger placeholder="Search library" shortcut="mod+shift+f" onOpen={openSearch} className="nav-focus" data-focusable />}
      actions={
        <>
          <CastButton className="nav-focus" />
          <Tooltip content="Commands">
            <Button variant="ghost" size="sm" shape="circle" icon={<IconCommand />} aria-label="Commands" onClick={palette.open} className="nav-focus" data-focusable />
          </Tooltip>
        </>
      }
      account={<UserMenu />}
    />
  );
}
