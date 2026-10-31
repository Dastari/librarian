import { useMutation } from "@apollo/client/react";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, filters, IdentityCell, NumericCell, type DataTableSearch } from "glass-ux/data-table";
import { IconExternalLink, IconRefresh, IconSearch } from "glass-ux/icons";
import { Segmented } from "glass-ux/segmented";
import { toast } from "glass-ux/toast";
import { useNavigate } from "@tanstack/react-router";
import { IconBookmark, IconBookmarkOff, IconDownload, IconEyeOff, IconTargetArrow } from "@tabler/icons-react";
import { parseAsStringEnum, useQueryState } from "nuqs";
import { useMemo, useState } from "react";

import { Artwork, Button, ErrorState } from "@/components/ui";
import { ReleaseSearchDialog, type ReleaseTarget } from "@/features/downloads/ReleaseSearchDialog";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import {
  type ContentStatus,
  EntityChapterUpdateManyDocument,
  EntityEpisodeUpdateManyDocument,
  EntityMovieUpdateManyDocument,
  EntityTrackUpdateManyDocument,
  EntityLibraryListDocument,
  SearchMissingDocument,
} from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { entityPath, glyph } from "@/lib/entities/kinds";
import { badgeTone } from "@/lib/entities/linked";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { statusMeta } from "@/lib/status";

import { commonSeason, summariseItems, type WantedGroup, type WantedKind } from "./grouping";
import { WANTED_TABS, WANTED_TAB_LABEL, useWanted, type WantedTab } from "./useWanted";

const KIND_META: Record<WantedKind, { label: string; tint: string; page: "show" | "movie" | "album" | "audiobook" }> = {
  episode: { label: "TV", tint: LIBRARY_TYPES.tv.tintVar, page: "show" },
  movie: { label: "Movie", tint: LIBRARY_TYPES.movies.tintVar, page: "movie" },
  track: { label: "Music", tint: LIBRARY_TYPES.music.tintVar, page: "album" },
  chapter: { label: "Audiobook", tint: LIBRARY_TYPES.audiobooks.tintVar, page: "audiobook" },
};
const HuntIcon = glyph(IconTargetArrow, "target-arrow");
const WantIcon = glyph(IconBookmark, "bookmark");
const UnwantIcon = glyph(IconBookmarkOff, "bookmark-off");
const IgnoreIcon = glyph(IconEyeOff, "eye-off");

function columnsFor(statuses: Map<string, ContentStatus>) {
  const column = createDataTableColumns<WantedGroup>();
  const statusOf = (group: WantedGroup) => statusMeta(statuses.get(`${group.kind}:${group.items[0]!.id}`));
  return column.columns([
    column.accessor("title", {
      header: "Title", width: 28, priority: 1, cardRole: "title", sortable: true,
      cell: ({ row }) => {
        const group = row.original;
        const artwork = <Artwork src={group.poster} alt="" aspect={group.square ? "square" : "poster"} tint={KIND_META[group.kind].tint} className={group.square ? "w-8 rounded-sm" : "w-6 rounded-sm"} />;
        return <IdentityCell icon={artwork} title={group.title} subtitle={[group.subtitle, group.kind === "movie" ? null : summariseItems(group.items)].filter(Boolean).join(" · ")} />;
      },
    }),
    column.accessor((group) => group.kind, {
      id: "kind", header: "Type", width: 8, priority: 2, sortable: true,
      filter: filters.options({ field: "kind", nullable: false, operators: ["in"], options: (Object.keys(KIND_META) as WantedKind[]).map((value) => ({ value, label: KIND_META[value].label })) }),
      cell: ({ row }) => <Badge size="sm" tone="neutral">{KIND_META[row.original.kind].label}</Badge>,
    }),
    column.accessor((group) => group.items.length, { id: "count", header: "Items", width: 6, priority: 2, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <NumericCell value={row.original.items.length} /> }),
    column.accessor((group) => statusOf(group).label, { id: "status", header: "Status", width: 10, priority: 2, cardRole: "status", sortable: true, cell: ({ row }) => { const meta = statusOf(row.original); return <Badge size="sm" marker="dot" tone={badgeTone(meta.tone)}>{meta.label}</Badge>; } }),
  ]);
}

const search: DataTableSearch<WantedGroup> = { fields: ["title", "subtitle"], placeholder: "Search titles" };
const pagePath = (group: WantedGroup) => entityPath(KIND_META[group.kind].page, group.parentId);

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
  const libraries = useCompleteConnection(EntityLibraryListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.libraries, { skip: !isAdmin });

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
    const ids = libraries.rows.map((library) => library.id);
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

  const { user } = useSession();
  const columns = useMemo(() => columnsFor(list.statuses), [list.statuses]);
  const actions = useMemo(() => {
    const each = async (groups: readonly WantedGroup[], input: { wanted?: boolean; ignored?: boolean }, message: string) => { for (const group of groups) await apply(group, input, message); };
    return defineActions<WantedGroup>("wanted", [
      { id: "open", label: (groups) => `Open ${KIND_META[groups[0]!.kind].page}`, icon: IconExternalLink, group: "open", priority: 3, visible: (groups) => groups.length === 1, run: (groups, context) => { const href = pagePath(groups[0]!); if (href) context.navigate(href); } },
      ...(isAdmin ? [
        { id: "hunt", label: (groups: readonly WantedGroup[]) => (groups.length === 1 ? "Search now" : `Search ${groups.length} titles now`), icon: HuntIcon, group: "manage" as const, priority: 1, bulk: true, run: async (groups: readonly WantedGroup[]) => { for (const group of groups) await searchGroup(group); } },
        { id: "browse", label: "Browse releases…", icon: IconSearch, group: "manage" as const, priority: 2, visible: (groups: readonly WantedGroup[]) => groups.length === 1, run: (groups: readonly WantedGroup[]) => setSearching(groups[0]!) },
        { id: "want", label: "Mark wanted", icon: WantIcon, group: "organise" as const, priority: 2, bulk: true, visible: (groups: readonly WantedGroup[]) => groups.some((group) => group.items.some((item) => !item.wanted)), run: (groups: readonly WantedGroup[]) => each(groups, { wanted: true, ignored: false }, "marked wanted") },
        { id: "unwant", label: "Unwant", icon: UnwantIcon, group: "organise" as const, bulk: true, visible: (groups: readonly WantedGroup[]) => groups.some((group) => group.items.some((item) => item.wanted)), run: (groups: readonly WantedGroup[]) => each(groups, { wanted: false }, "no longer wanted") },
        { id: "ignore", label: "Ignore", icon: IgnoreIcon, group: "danger" as const, bulk: true, run: (groups: readonly WantedGroup[]) => each(groups, { ignored: true, wanted: false }, "ignored") },
      ] : []),
    ]);
  }, [apply, isAdmin, searchGroup]);

  return (
    <div data-print-flow className="page-gutter flex min-h-0 flex-1 flex-col gap-4 overflow-hidden py-4 md:py-6 [@media(max-height:35rem)]:gap-2 [@media(max-height:35rem)]:py-2">
      <h1 className="sr-only">Wanted</h1>
      <div className="flex shrink-0 flex-wrap items-center gap-x-6 gap-y-3">
        <Segmented<WantedTab> aria-label="Show" value={tab} onValueChange={(value) => void setTab(value === "wanted" ? null : value)} options={WANTED_TABS.map((value) => ({ value, label: WANTED_TAB_LABEL[value] }))} />
        {list.groups.length > 0 ? <span className="text-numeric text-label-sm text-muted">{list.groups.length} titles · {list.itemCount} items</span> : null}
        {isAdmin ? (
          <div className="ms-auto">
            <Button variant="primary" onPress={() => void searchAll()} isPending={triggering || libraries.loading} isDisabled={Boolean(libraries.error)}><IconDownload size={16} /> Search all missing</Button>
          </div>
        ) : null}
      </div>
      {isAdmin && libraries.error ? <ErrorState compact error={libraries.error} onRetry={() => void libraries.refetch()} /> : null}
      {list.error && list.groups.length > 0 ? <ErrorState compact error={list.error} onRetry={list.refetch} /> : null}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <DataTable<WantedGroup>
          id={`librarian.${user?.id ?? "session"}.wanted.${tab}`}
          label="Wanted"
          queryKey={list.queryKey}
          height="fill"
          columns={columns}
          rows={list.groups}
          getRowId={(group) => group.key}
          getRowLabel={(group) => group.title}
          loading={list.loading}
          refetching={list.refreshing}
          search={search}
          actions={actions}
          selectable={isAdmin}
          noun={{ one: "title", other: "titles" }}
          onRowOpen={(group) => { const href = pagePath(group); if (href) void navigate({ to: href }); }}
          rowOpen="click"
          error={list.error && list.groups.length === 0 ? <ErrorState error={list.error} onRetry={() => list.refetch()} /> : undefined}
          emptyMessage={`${EMPTY[tab].title}. ${EMPTY[tab].description}`}
          toolbar={<Button size="sm" variant="ghost" isIconOnly aria-label="Refresh" isPending={list.refreshing} onPress={() => list.refetch()}><IconRefresh /></Button>}
        />
      </div>
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
