import { Link, useRouterState } from "@tanstack/react-router";
import { CountBadge } from "glass-ux/badge";
import { Separator } from "glass-ux/separator";
import { Rail } from "glass-ux/shell";

import { NowPlayingRailItem } from "@/features/player/nowPlaying";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { useLibraryPreferences } from "@/features/personal/usePersonalLibrary";
import { EntityLibraryListDocument } from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { useInputMode } from "@/lib/input-mode";
import { libraryType } from "@/lib/library-types";
import { cn } from "@/lib/utils";

import { PRIMARY_NAV, isNavActive, type NavItem } from "./nav";
import { useHasShellArt } from "./shellArt";
import { useShellCounts } from "./useShellCounts";
import { UserMenu } from "./UserMenu";

/**
 * Level-one navigation: Glass UX's icon rail on the canvas, each item named by its tooltip,
 * with your favourite libraries under a divider and Settings at the foot. The zone attributes put it in the
 * remote's spatial navigation, entered and left with Left and Right.
 */
export function NavRail() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const counts = useShellCounts();
  const isAdmin = useIsAdmin();
  const { status } = useSession();
  const overArt = useHasShellArt();
  const tv = useInputMode().mode === "tv";
  const libraries = useCompleteConnection(EntityLibraryListDocument, {}, (data) => data.libraries, { skip: status !== "authenticated" });
  const preferences = useLibraryPreferences();
  const favouriteIds = new Set(preferences.rows.filter((row) => row.favourite).map((row) => row.libraryId));
  const libraryRows = libraries.rows.filter((row) => favouriteIds.has(row.id));
  const items = PRIMARY_NAV.filter((item) => !item.adminOnly || isAdmin);
  const entry = (item: NavItem) => {
    const count = item.badge === "downloads" ? counts.downloads : item.badge === "notifications" ? counts.notifications : 0;
    return (
      <Rail.Item
        key={item.key}
        className="nav-focus"
        label={item.label}
        icon={<item.icon />}
        active={isNavActive(item.href, pathname)}
        badge={count > 0 ? <CountBadge value={count} overlay /> : undefined}
        render={<Link to={item.href as never} data-focusable />}
      />
    );
  };

  return (
    <Rail.Root aria-label="Primary" overflow={tv ? "scroll" : undefined} className={cn("app-shell-rail", overArt && "over-imagery")} data-nav-zone="rail" data-nav-side="" data-print="hide">
      {/* On a TV, what is playing is the first stop on the rail (design.md Q125). */}
      <NowPlayingRailItem />
      {items.filter((item) => item.key !== "settings").map(entry)}
      {libraryRows.length ? <li aria-hidden className="app-rail-divider"><Separator /></li> : null}
      {libraryRows.map((node) => {
        const meta = libraryType(node.libraryType);
        return (
          <Rail.Item
            key={node.id}
            className="nav-focus"
            label={node.name}
            icon={<meta.icon className={node.scanning ? "animate-pulse-soft" : undefined} />}
            active={isNavActive(`/libraries/${node.id}`, pathname)}
            render={<Link to="/libraries/$libraryId" params={{ libraryId: node.id }} data-focusable />}
          />
        );
      })}
      <Rail.Spacer />
      {items.filter((item) => item.key === "settings").map(entry)}
      {tv ? <UserMenu placement="rail" /> : null}
    </Rail.Root>
  );
}
