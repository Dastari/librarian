import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { useNavigate } from "@tanstack/react-router";
import { IconBookmark, IconBookmarkOff, IconDownload, IconEyeOff, IconListSearch, IconRefresh, IconSearch, IconTargetArrow } from "@tabler/icons-react";
import { parseAsStringEnum, useQueryState } from "nuqs";
import { useMemo, useState } from "react";

import { Artwork, Button, DataTable, type DataTableColumn, type DataTableRowAction, EmptyState, ErrorState, PageHeader, SegmentTabs, StatusChip } from "@/components/ui";
import { ReleaseSearchDialog, type ReleaseTarget } from "@/features/downloads/ReleaseSearchDialog";
import {
  EntityChapterUpdateManyDocument,
  EntityEpisodeUpdateManyDocument,
  EntityMovieUpdateManyDocument,
  EntityTrackUpdateManyDocument,
  NavLibrariesDocument,
  SearchMissingDocument,
} from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { statusMeta } from "@/lib/status";

import { commonSeason, summariseItems, type WantedGroup, type WantedKind } from "./grouping";
import { WANTED_TABS, WANTED_TAB_LABEL, useWanted, type WantedTab } from "./useWanted";

const KIND_META: Record<WantedKind, { label: string; tint: string }> = {
  episode: { label: "TV", tint: LIBRARY_TYPES.tv.tintVar },
  movie: { label: "Movie", tint: LIBRARY_TYPES.movies.tintVar },
  track: { label: "Music", tint: LIBRARY_TYPES.music.tintVar },
  chapter: { label: "Audiobook", tint: LIBRARY_TYPES.audiobooks.tintVar },
};

const EMPTY: Record<WantedTab, { title: string; description: string }> = {
  missing: { title: "Nothing is missing", description: "Items without a file that are not marked wanted appear here." },
  wanted: { title: "Nothing is wanted", description: "Mark an episode, movie, track or chapter as wanted and it shows up here." },
  downloading: { title: "Nothing is downloading", description: "Grabs in progress appear here until their files are imported." },
  upgradable: { title: "Nothing to upgrade", description: "Every file meets its quality profile." },
};

/** The acquisition queue: what is missing, wanted, downloading or below its quality profile. */
export function WantedPage() {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const [tab, setTab] = useQueryState("show", parseAsStringEnum<WantedTab>([...WANTED_TABS]).withDefault("wanted"));
  const list = useWanted(tab);
  const [searching, setSearching] = useState<WantedGroup | null>(null);

  const [updateEpisodes] = useMutation(EntityEpisodeUpdateManyDocument);
  const [updateMovies] = useMutation(EntityMovieUpdateManyDocument);
  const [updateTracks] = useMutation(EntityTrackUpdateManyDocument);
  const [updateChapters] = useMutation(EntityChapterUpdateManyDocument);
  const [searchMissing, { loading: triggering }] = useMutation(SearchMissingDocument);
  const libraries = useQuery(NavLibrariesDocument, { skip: !isAdmin });

  const apply = async (group: WantedGroup, input: { wanted?: boolean; ignored?: boolean }, message: string) => {
    const ids = group.items.map((item) => item.id);
    const where = { id: { inList: ids } };
    try {
      if (group.kind === "episode") assertSuccess((await updateEpisodes({ variables: { where, input } })).data?.updateEpisodes, "Could not update");
      else if (group.kind === "movie") assertSuccess((await updateMovies({ variables: { where, input } })).data?.updateMovies, "Could not update");
      else if (group.kind === "track") assertSuccess((await updateTracks({ variables: { where, input } })).data?.updateTracks, "Could not update");
      else assertSuccess((await updateChapters({ variables: { where, input } })).data?.updateChapters, "Could not update");
      toast.success(`${group.title}: ${message}`);
      list.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  /** `searchMissing` needs a scope, so "search everything" is one call per library. */
  const searchAll = async () => {
    const ids = libraries.data?.libraries.edges.map((edge) => edge.node.id) ?? [];
    if (ids.length === 0) return;
    try {
      let searched = 0;
      let queued = 0;
      const failures: string[] = [];
      for (const libraryId of ids) {
        const { data } = await searchMissing({ variables: { input: { libraryId } } });
        const result = data?.searchMissing;
        if (result?.success) {
          searched += result.searched;
          queued += result.queued;
        } else if (result?.error) failures.push(result.error);
      }
      if (failures.length) toast.warning(failures.join(" · "));
      else toast.success(searched === 0 ? "Nothing left to search" : `Searched ${searched}, grabbed ${queued}`);
      list.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  /** Hunt for one title now, ignoring the search backoff. */
  const searchGroup = async (group: WantedGroup) => {
    const season = group.kind === "episode" ? commonSeason(group.items) : null;
    const scope =
      group.kind === "episode"
        ? { showId: group.parentId, season }
        : group.kind === "movie"
          ? { movieId: group.parentId }
          : group.kind === "track"
            ? { albumId: group.parentId }
            : { audiobookId: group.parentId };
    try {
      const { data } = await searchMissing({ variables: { input: scope } });
      const result = data?.searchMissing;
      if (result?.success) toast.success(`${group.title}: searched ${result.searched}, grabbed ${result.queued}`);
      else toast.warning(result?.error ?? "The search did not run");
      list.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const open = (group: WantedGroup) => {
    if (group.kind === "episode") void navigate({ to: "/shows/$showId", params: { showId: group.parentId } });
    else if (group.kind === "movie") void navigate({ to: "/movies/$movieId", params: { movieId: group.parentId } });
    else if (group.kind === "track") void navigate({ to: "/albums/$albumId", params: { albumId: group.parentId } });
    else void navigate({ to: "/audiobooks/$audiobookId", params: { audiobookId: group.parentId } });
  };

  const columns = useMemo<Array<DataTableColumn<WantedGroup>>>(
    () => [
      {
        id: "title",
        header: "Title",
        cell: (group) => (
          <span className="flex items-center gap-3">
            <Artwork src={group.poster} alt="" aspect={group.square ? "square" : "poster"} tint={KIND_META[group.kind].tint} className={group.square ? "w-12 shrink-0 rounded-sm" : "w-8 shrink-0 rounded-sm"} />
            <span className="min-w-0">
              <span className="block truncate text-body-sm text-foreground">{group.title}</span>
              <span className="block truncate text-label-sm text-muted">{[group.subtitle, group.kind === "movie" ? null : summariseItems(group.items)].filter(Boolean).join(" · ")}</span>
            </span>
          </span>
        ),
      },
      { id: "kind", header: "Type", size: 120, hideBelow: "sm", cell: (group) => <span className="text-muted">{KIND_META[group.kind].label}</span> },
      { id: "count", header: "Items", size: 90, align: "end", numeric: true, cell: (group) => group.items.length },
      {
        id: "status",
        header: "Status",
        size: 150,
        hideBelow: "md",
        cell: (group) => <StatusChip status={statusMeta(list.statuses.get(`${group.kind}:${group.items[0]!.id}`))} />,
      },
    ],
    [list.statuses],
  );

  const rowActions = useMemo<Array<DataTableRowAction<WantedGroup>>>(
    () =>
      isAdmin
        ? [
            { key: "hunt", label: "Search now", icon: <IconTargetArrow size={16} />, onAction: (group) => searchGroup(group) },
            { key: "search", label: "Browse releases…", icon: <IconSearch size={16} />, onAction: (group) => setSearching(group) },
            { key: "want", label: "Mark wanted", icon: <IconBookmark size={16} />, hidden: (group) => group.items.every((item) => item.wanted), onAction: (group) => apply(group, { wanted: true, ignored: false }, "marked wanted") },
            { key: "unwant", label: "Unwant", icon: <IconBookmarkOff size={16} />, hidden: (group) => group.items.every((item) => !item.wanted), onAction: (group) => apply(group, { wanted: false }, "no longer wanted") },
            { key: "ignore", label: "Ignore", icon: <IconEyeOff size={16} />, onAction: (group) => apply(group, { ignored: true, wanted: false }, "ignored") },
          ]
        : [],
    [isAdmin],
  );

  return (
    <div className="page-gutter flex flex-col gap-6 py-8">
      <PageHeader
        title="Wanted"
        meta={list.groups.length > 0 ? <span className="text-numeric">{list.groups.length} titles · {list.itemCount} items</span> : undefined}
        actions={
          <>
            <Button variant="ghost" isIconOnly aria-label="Refresh" onPress={() => list.refetch()}>
              <IconRefresh size={18} />
            </Button>
            {isAdmin ? (
              <Button variant="primary" onPress={() => void searchAll()} isPending={triggering}>
                <IconDownload size={16} /> Search all missing
              </Button>
            ) : null}
          </>
        }
      />
      <SegmentTabs ariaLabel="Wanted filter" items={WANTED_TABS.map((key) => ({ key, label: WANTED_TAB_LABEL[key] }))} selected={tab} onSelect={(key) => void setTab(key as WantedTab)} />
      <DataTable<WantedGroup>
        columns={columns}
        rows={list.groups}
        getRowId={(group) => group.key}
        isLoading={list.loading}
        rowActions={rowActions}
        noun="titles"
        onRowClick={open}
        infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
        error={list.error && list.groups.length === 0 ? <ErrorState error={list.error} onRetry={() => list.refetch()} /> : undefined}
        emptyState={<EmptyState icon={IconListSearch} title={EMPTY[tab].title} description={EMPTY[tab].description} />}
      />
      <ReleaseSearchDialog
        isOpen={Boolean(searching)}
        onOpenChange={(open) => !open && setSearching(null)}
        query={searching ? [searching.kind === "track" || searching.kind === "chapter" ? searching.subtitle : null, searching.title].filter(Boolean).join(" ") : ""}
        artist={searching?.kind === "track" ? searching.subtitle : null}
        album={searching?.kind === "track" ? searching.title : null}
        author={searching?.kind === "chapter" ? searching.subtitle : null}
        year={searching?.kind === "movie" ? searching.year : null}
        imdbId={searching?.imdbId}
        season={searching ? commonSeason(searching.items) : null}
        libraryId={searching?.libraryId}
        target={searching ? releaseTarget(searching) : undefined}
      />
    </div>
  );
}

/** Where a grab from this row should be recorded. Episode groups grab at show level. */
function releaseTarget(group: WantedGroup): ReleaseTarget {
  switch (group.kind) {
    case "episode":
      return { showId: group.parentId, episodeId: group.items.length === 1 ? group.items[0]!.id : undefined };
    case "movie":
      return { movieId: group.parentId };
    case "track":
      return { albumId: group.parentId, trackId: group.items.length === 1 ? group.items[0]!.id : undefined };
    default:
      return { audiobookId: group.parentId, chapterId: group.items.length === 1 ? group.items[0]!.id : undefined };
  }
}
