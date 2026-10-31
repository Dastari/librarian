import { Tooltip } from "glass-ux/tooltip";
import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { createContext, useState } from "react";
import { IconWarning, IconSpinner, IconRefresh, IconSettings } from "glass-ux/icons";

import { Button, MediaBackdrop, PageHeader, SegmentTabs, type TabItem } from "@/components/ui";
import type { SpotlightArt } from "@/features/home/HomeSpotlight";
import type { LibraryDetailQuery } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatRelative, pluralize } from "@/lib/format";
import { libraryType, type LibraryType } from "@/lib/library-types";
import { cn } from "@/lib/utils";

import { useLibraryScan } from "./useLibraryScan";

export type LibraryDetail = NonNullable<LibraryDetailQuery["library"]>;

/** Lets the Recommended view put its spotlight's art behind the whole library page. */
export const LibraryArtContext = createContext<(art: SpotlightArt) => void>(() => {});

/** Tab set per library type; the first entry is the default view. */
export function libraryTabs(library: LibraryDetail): TabItem[] {
  const base = `/libraries/${library.id}`;
  const type = libraryType(library.libraryType).type;
  const byType: Record<LibraryType, TabItem[]> = {
    movies: [
      { key: "recommended", label: "Recommended", href: `${base}/recommended` },
      { key: "movies", label: "Browse", href: `${base}/movies`, count: library.movies.pageInfo.totalCount },
      { key: "collections", label: "Collections", href: `${base}/collections`, count: library.collections.pageInfo.totalCount },
    ],
    tv: [
      { key: "recommended", label: "Recommended", href: `${base}/recommended` },
      { key: "shows", label: "Browse", href: `${base}/shows`, count: library.shows.pageInfo.totalCount },
    ],
    music: [
      { key: "albums", label: "Albums", href: `${base}/albums`, count: library.albums.pageInfo.totalCount },
      { key: "artists", label: "Artists", href: `${base}/artists` },
      { key: "tracks", label: "Tracks", href: `${base}/tracks` },
    ],
    audiobooks: [{ key: "audiobooks", label: "Audiobooks", href: `${base}/audiobooks`, count: library.audiobooks.pageInfo.totalCount }],
    other: [],
  };
  return [
    ...byType[type],
    { key: "files", label: "Files", href: `${base}/files`, count: library.mediaFiles.pageInfo.totalCount },
    { key: "scans", label: "Scans", href: `${base}/scans` },
  ];
}

export function LibraryLayout({ library }: { library: LibraryDetail }) {
  const meta = libraryType(library.libraryType);
  const isAdmin = useIsAdmin();
  const scan = useLibraryScan(library.id);
  const tabs = libraryTabs(library);
  const primaryCount = tabs.find((tab) => tab.count !== undefined)?.count ?? 0;
  // Browsers own their result scrolling; forms and scan history need a scrolling pane;
  // Recommended is a page of artwork and shelves that scrolls as a whole.
  const scrollOutlet = useRouterState({
    select: (state) => state.matches.some((match) => match.routeId === "/_app/libraries/$libraryId/settings" || match.routeId === "/_app/libraries/$libraryId/scans"),
  });
  const flow = useRouterState({ select: (state) => state.matches.some((match) => match.routeId === "/_app/libraries/$libraryId/recommended") });
  const [art, setArt] = useState<SpotlightArt | null>(null);

  return (
    <LibraryArtContext value={setArt}>
    <div data-print-flow className={cn("relative isolate flex min-w-0 flex-col gap-3 page-gutter sm:gap-4 [@media(max-height:35rem)]:gap-2",
      flow ? "overflow-x-clip pt-[calc(var(--topbar-height)+var(--safe-top)+var(--space-3))]" : "min-h-0 flex-1 pt-3 sm:pt-5 [@media(max-height:35rem)]:pt-2")}>
      {flow && art ? <MediaBackdrop src={art.src} fallback={art.fallback} tint={art.tint} /> : null}
      {/* The breadcrumb names the library; the heading stays for assistive technology and print. */}
      <PageHeader className="sr-only" titleVisibility="accessible" title={library.name} />
      <div className={cn("flex min-w-0 shrink-0 items-center gap-x-4 gap-y-2 max-sm:flex-wrap", flow && "over-imagery")}>
        <SegmentTabs items={tabs} ariaLabel="Library sections" className="min-w-0 flex-1" />
        <div className="flex min-w-0 shrink-0 items-center gap-3 text-label-sm text-muted max-sm:w-full max-sm:justify-between">
          <span className="inline-flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
            <span className={`inline-flex items-center gap-1.5 ${meta.tint}`}><meta.icon size={14} /> <span className="text-muted">{pluralize(primaryCount, meta.singular.toLowerCase())}</span></span>
            {library.lastScannedAt ? <span className="max-md:hidden">Scanned {formatRelative(library.lastScannedAt)}</span> : null}
            {scan.unresolvedIssues > 0 ? (
              <Link to="/libraries/$libraryId/scans" params={{ libraryId: library.id }} className="nav-focus inline-flex items-center gap-1 rounded text-warning">
                <IconWarning size="sm" /> {pluralize(scan.unresolvedIssues, "issue")}
              </Link>
            ) : null}
          </span>
          {isAdmin ? (
            <span data-print="hide" className="flex items-center gap-1">
              <Tooltip content={scan.running || library.scanning ? "A library scan is already running" : "Look for new, changed and missing files"}>
                <Button variant="ghost" size="sm" onPress={() => void scan.scan()} isDisabled={scan.running || library.scanning} isPending={scan.scanning} focusableWhenDisabled>
                  {scan.running || library.scanning ? <IconSpinner className="animate-spin" /> : <IconRefresh />}
                  {scan.running || library.scanning ? "Scanning" : "Scan"}
                </Button>
              </Tooltip>
              <Button variant="ghost" size="sm" isIconOnly aria-label="Library settings" nativeButton={false} render={<Link to="/libraries/$libraryId/settings" params={{ libraryId: library.id }} />}>
                <IconSettings />
              </Button>
            </span>
          ) : null}
        </div>
      </div>
      <div data-print-flow className={cn("fa-focus-room flex min-h-0 min-w-0 flex-1 flex-col", flow ? "" : scrollOutlet ? "scrollbar-thin overflow-y-auto pb-[calc(var(--space-8)+var(--focus-reach))]" : "overflow-hidden pb-[calc(var(--space-3)+var(--focus-reach))] sm:pb-[calc(var(--space-4)+var(--focus-reach))]")}>
        <Outlet />
      </div>
    </div>
    </LibraryArtContext>
  );
}
