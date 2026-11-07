import { ActionBar } from "glass-ux/actions";
import { Outlet, useNavigate, useRouterState } from "@tanstack/react-router";
import { Segmented } from "glass-ux/segmented";
import { createContext, useMemo, useState } from "react";

import { PageScroller } from "@/components/shell/PageScroller";
import { MediaBackdrop, type TabItem } from "@/components/ui";
import type { SpotlightArt } from "@/features/home/HomeSpotlight";
import { useMediaQuery } from "@/hooks/useMediaQuery";
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

/** The ways to browse each type of library; the first is the default. Its files are in its settings. */
export function libraryViews(library: LibraryDetail): TabItem[] {
  const base = `/libraries/${library.id}`;
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
    // Nothing to browse but its files.
    other: [{ key: "files", label: "Files", href: `${base}/files` }],
  };
  return byType[libraryType(library.libraryType).type];
}

export function LibraryLayout({ library }: { library: LibraryDetail }) {
  const scan = useLibraryScan(library.id);
  const files = useQuery(LibraryFileCountDocument, { variables: { libraryId: library.id } });
  const fileCount = files.data?.mediaFiles.pageInfo.totalCount;
  const views = libraryViews(library);
  const primaryCount = views.find((view) => view.count !== undefined)?.count ?? fileCount ?? 0;
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const navigate = useNavigate();
  // A view switch needs the row's room as well, so a library with one collapses sooner.
  const narrow = useMediaQuery(views.length > 1 ? "(max-width: 79.99rem)" : "(max-width: 39.99rem)");
  // Browsers own their result scrolling; Recommended is a page of artwork and shelves that scrolls as a whole.
  const flow = useRouterState({ select: (state) => state.matches.some((match) => match.routeId === "/_app/libraries/$libraryId/recommended") });
  const [art, setArt] = useState<SpotlightArt | null>(null);
  const settings = useLibrarySettingsParam();
  const scanningLibrary = useMemo(() => ({ ...library, scanning: library.scanning || scan.running }), [library, scan.running]);
  const actions = useLibraryActions<typeof scanningLibrary>({ settings: settings.open, open: false });

  const viewSwitch = views.length > 1 ? (
    <Segmented
      aria-label="Library views"
      size="sm"
      value={views.find((view) => view.href && pathname.startsWith(view.href))?.key ?? views[0]!.key}
      onValueChange={(key) => { const href = views.find((view) => view.key === key)?.href; if (href) void navigate({ href }); }}
      options={views.map((view) => ({ value: view.key, label: view.count !== undefined ? `${view.label} ${view.count}` : view.label }))}
    />
  ) : null;

  return (
    <LibraryArtContext value={setArt}>
    <div data-print-flow className="relative flex min-h-0 min-w-0 flex-1 flex-col">
      {flow && art ? <MediaBackdrop src={art.src} fallback={art.fallback} tint={art.tint} /> : null}
      {/* The identity never scrolls away; only the view under it does. Its top padding is room for
          the issue count, which sits over the corner of the tile and would otherwise be clipped. */}
      <div className={cn("page-gutter flex min-w-0 shrink-0 flex-col gap-2 pt-2 pb-3 sm:pb-4 [@media(max-height:35rem)]:pb-2", flow && "over-imagery")}>
        {/* A narrow page collapses the identity to one line, with its actions behind its menu. */}
        <LibraryIdentity
          tier={narrow ? "compact" : "header"}
          heading="h1"
          className="min-w-0"
          library={scanningLibrary}
          items={primaryCount}
          issues={scan.unresolvedIssues}
          actions={actions}
          headerActions={narrow ? undefined : (
            <span data-print="hide" className="flex min-w-0 items-center gap-3">
              {viewSwitch}
              <ActionBar set={actions} targets={[scanningLibrary]} label={`Actions for ${library.name}`} maxButtons={1} />
            </span>
          )}
        />
        {/* On its own line it keeps its own width rather than stretching across the page. */}
        {narrow && viewSwitch ? <div className="max-w-full self-start">{viewSwitch}</div> : null}
      </div>
      <PageScroller data-print-flow scrolling={flow} className={cn("page-gutter flex min-h-0 flex-1 flex-col", !flow && "fa-focus-room", !flow && "pb-[calc(var(--space-3)+var(--focus-reach))] sm:pb-[calc(var(--space-4)+var(--focus-reach))]")}>
        <Outlet />
      </PageScroller>
      <LibrarySettingsDialog library={scanningLibrary} open={settings.tab !== null} onOpenChange={(next) => { if (!next) settings.close(); }} tab={settings.tab ?? "general"} onTabChange={(tab) => settings.open(library, tab)} items={primaryCount} issues={scan.unresolvedIssues} fileCount={fileCount} />
    </div>
    </LibraryArtContext>
  );
}
