import { Tooltip } from "@heroui/react";
import { Link, Outlet } from "@tanstack/react-router";
import { IconAlertTriangle, IconLoader2, IconRefresh, IconSettings } from "@tabler/icons-react";

import { Button, PageHeader, SegmentTabs, type TabItem } from "@/components/ui";
import type { LibraryDetailQuery } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatRelative, pluralize } from "@/lib/format";
import { libraryType, type LibraryType } from "@/lib/library-types";

import { useLibraryScan } from "./useLibraryScan";

export type LibraryDetail = NonNullable<LibraryDetailQuery["library"]>;

/** Tab set per library type; the first entry is the default view. */
export function libraryTabs(library: LibraryDetail): TabItem[] {
  const base = `/libraries/${library.id}`;
  const type = libraryType(library.libraryType).type;
  const byType: Record<LibraryType, TabItem[]> = {
    movies: [
      { key: "movies", label: "Movies", href: `${base}/movies`, count: library.movies.pageInfo.totalCount },
      { key: "collections", label: "Collections", href: `${base}/collections`, count: library.collections.pageInfo.totalCount },
    ],
    tv: [{ key: "shows", label: "Shows", href: `${base}/shows`, count: library.shows.pageInfo.totalCount }],
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
  const primaryCount = tabs[0]?.count ?? 0;

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-5 page-gutter pt-6">
      <PageHeader
        eyebrow={
          <span className={`inline-flex items-center gap-1.5 ${meta.tint}`}>
            <meta.icon size={14} /> {meta.label}
          </span>
        }
        title={library.name}
        meta={
          <span className="inline-flex flex-wrap items-center gap-x-3 gap-y-1">
            <span>{pluralize(primaryCount, meta.singular.toLowerCase())}</span>
            {library.lastScannedAt ? <span>Scanned {formatRelative(library.lastScannedAt)}</span> : null}
            {scan.unresolvedIssues > 0 ? (
              <Link to="/libraries/$libraryId/scans" params={{ libraryId: library.id }} className="nav-focus inline-flex items-center gap-1 rounded text-warning">
                <IconAlertTriangle size={14} /> {pluralize(scan.unresolvedIssues, "issue")}
              </Link>
            ) : null}
          </span>
        }
        actions={
          isAdmin ? (
            <>
              <Tooltip delay={300}>
                <Button variant="secondary" onPress={() => void scan.scan()} isDisabled={scan.running || library.scanning} isPending={scan.scanning}>
                  {scan.running || library.scanning ? <IconLoader2 size={16} className="animate-spin" /> : <IconRefresh size={16} />}
                  {scan.running || library.scanning ? "Scanning" : "Scan"}
                </Button>
                <Tooltip.Content>Look for new, changed and missing files</Tooltip.Content>
              </Tooltip>
              <Link to="/libraries/$libraryId/settings" params={{ libraryId: library.id }}>
                <Button variant="ghost" isIconOnly aria-label="Library settings">
                  <IconSettings size={18} />
                </Button>
              </Link>
            </>
          ) : null
        }
      />
      <SegmentTabs items={tabs} ariaLabel="Library sections" />
      <div className="scrollbar-thin -mx-1 flex min-h-0 flex-1 flex-col overflow-y-auto px-1 pb-8">
        <Outlet />
      </div>
    </div>
  );
}
