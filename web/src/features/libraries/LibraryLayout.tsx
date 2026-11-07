import { ActionBar } from "glass-ux/actions";
import { Outlet, useRouterState } from "@tanstack/react-router";
import { createContext, useMemo, useState } from "react";

import { PageScroller } from "@/components/shell/PageScroller";
import { MediaBackdrop, PageHeader, SegmentTabs, type TabItem } from "@/components/ui";
import type { SpotlightArt } from "@/features/home/HomeSpotlight";
import { LibraryFileCountDocument, type LibraryDetailQuery } from "@/graphql/generated/graphql";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { libraryType, type LibraryType } from "@/lib/library-types";
import { cn } from "@/lib/utils";

import { LibraryIdentity } from "./LibraryIdentity";
import { LibrarySettingsDialog } from "./LibrarySettingsDialog";
import { useLibraryActions, useLibrarySettingsParam } from "./useLibraryActions";
import { useLibraryScan } from "./useLibraryScan";

export type LibraryDetail = NonNullable<LibraryDetailQuery["library"]>;

/** Lets the Recommended view put its spotlight's art behind the whole library page. */
export const LibraryArtContext = createContext<(art: SpotlightArt) => void>(() => {});

/** Tab set per library type; the first entry is the default view. */
export function libraryTabs(library: LibraryDetail, fileCount?: number | null): TabItem[] {
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
    { key: "files", label: "Files", href: `${base}/files`, count: fileCount ?? undefined },
  ];
}

export function LibraryLayout({ library }: { library: LibraryDetail }) {
  const scan = useLibraryScan(library.id);
  const files = useQuery(LibraryFileCountDocument, { variables: { libraryId: library.id } });
  const tabs = libraryTabs(library, files.data?.mediaFiles.pageInfo.totalCount);
  const primaryCount = tabs.find((tab) => tab.count !== undefined)?.count ?? 0;
  // Browsers own their result scrolling; Recommended is a page of artwork and shelves that scrolls as a whole.
  const flow = useRouterState({ select: (state) => state.matches.some((match) => match.routeId === "/_app/libraries/$libraryId/recommended") });
  const [art, setArt] = useState<SpotlightArt | null>(null);
  const settings = useLibrarySettingsParam();
  const scanningLibrary = useMemo(() => ({ ...library, scanning: library.scanning || scan.running }), [library, scan.running]);
  const actions = useLibraryActions<typeof scanningLibrary>({ settings: settings.open, open: false });

  return (
    <LibraryArtContext value={setArt}>
    <div data-print-flow className="relative flex min-h-0 min-w-0 flex-1 flex-col">
      {flow && art ? <MediaBackdrop src={art.src} fallback={art.fallback} tint={art.tint} /> : null}
      {/* The breadcrumb names the library; the heading stays for assistive technology and print. */}
      <PageHeader className="sr-only" titleVisibility="accessible" title={library.name} />
      {/* The tabs never scroll away; only the view under them does. */}
      <div className={cn("page-gutter flex min-w-0 shrink-0 items-center gap-x-4 gap-y-2 pb-3 max-sm:flex-wrap sm:pb-4 [@media(max-height:35rem)]:pb-2", flow && "over-imagery")}>
        <SegmentTabs items={tabs} ariaLabel="Library sections" className="min-w-0 flex-1" />
        <div className="flex min-w-0 shrink-0 items-center gap-2 max-sm:w-full max-sm:justify-between">
          <LibraryIdentity tier="row" className="w-60 min-w-0 max-md:w-44" library={scanningLibrary} items={primaryCount} issues={scan.unresolvedIssues} onOpen={() => settings.open(library, scan.unresolvedIssues > 0 ? "issues" : undefined)} />
          <span data-print="hide"><ActionBar set={actions} targets={[scanningLibrary]} label={`Actions for ${library.name}`} maxButtons={1} /></span>
        </div>
      </div>
      <PageScroller data-print-flow scrolling={flow} className={cn("page-gutter flex min-h-0 flex-1 flex-col", !flow && "fa-focus-room", !flow && "pb-[calc(var(--space-3)+var(--focus-reach))] sm:pb-[calc(var(--space-4)+var(--focus-reach))]")}>
        <Outlet />
      </PageScroller>
      <LibrarySettingsDialog library={scanningLibrary} open={settings.tab !== null} onOpenChange={(next) => { if (!next) settings.close(); }} tab={settings.tab ?? "general"} onTabChange={(tab) => settings.open(library, tab)} items={primaryCount} issues={scan.unresolvedIssues} />
    </div>
    </LibraryArtContext>
  );
}
