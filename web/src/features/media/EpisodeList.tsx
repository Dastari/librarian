import { copyText } from "glass-ux/copy-button";
import { useApolloClient, useMutation } from "@apollo/client/react";
import { ActionBar, ActionMenu, defineActions, type ActionSet } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, filters, NumericCell, TextCell, TwoLineCell } from "glass-ux/data-table";
import { IconBookmark, IconCheck, IconCopy, IconDownload, IconEye, IconEyeOff, IconPlay } from "glass-ux/icons";
import { MeterCell } from "glass-ux/meter-cell";
import { Progress } from "glass-ux/progress";
import { Surface } from "glass-ux/surface";
import { toast } from "glass-ux/toast";
import { createContext, useContext, useCallback, useEffect, useMemo, useState, type ReactNode } from "react";

import { Artwork } from "@/components/ui/Artwork";
import { MediaRow } from "@/components/ui/MediaRow";
import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { EntityEpisodeUpdateDocument, EntityEpisodeUpdateManyDocument, type PlaybackProgressFieldsFragment, type UpdateEpisodeInput } from "@/graphql/generated/graphql";
import { episodeThumb } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatDate } from "@/lib/format";
import { activeContentStatusQueries } from "@/lib/graphql/content-status";
import { assertSuccess } from "@/lib/graphql/errors";
import { contentStatusMeta, type ContentStatusMap } from "@/hooks/useContentStatuses";
import { qualityStatus, statusMeta } from "@/lib/status";

import { groupBySeason, type SeasonGroup } from "./seasons";
import type { EpisodeRow } from "./ShowPage";
import { filesWithProgress, resetProgressAction, useResetProgress, type ProgressReset } from "./progressReset";
import { noRowMenu, RowActions, type RowPlayState } from "./RowPlayAction";

interface EpisodeListProps {
  /** An episode to bring into view and mark. */
  focusEpisodeId?: string;
  episodes: EpisodeRow[];
  statuses: ContentStatusMap;
  progressByFile: Map<string | null | undefined, PlaybackProgressFieldsFragment>;
  loading: boolean;
  complete?: boolean;
  status?: ReactNode;
  playbackUnavailable?: string;
  onPlay: (episode: EpisodeRow) => void;
  showId: string;
  showName: string;
  libraryId: string;
  /** Called after a bulk or single update so the parent can refetch. */
  onChanged?: () => void;
  /** `row` is the watching view (a row of stills); `table` is the management view. */
  view?: "row" | "table";
  /** Artwork tint for episodes without a thumbnail. */
  tint?: string;
  /** Controls for the season header's end (the view toggle). */
  headerExtra?: ReactNode;
}

const episodeNoun = { one: "episode", other: "episodes" };
const episodeId = (episode: EpisodeRow) => episode.id;
const episodeCode = (episode: EpisodeRow) => `S${String(episode.season).padStart(2, "0")}E${String(episode.episode).padStart(2, "0")}`;
const episodeLabel = (episode: EpisodeRow) => `${episodeCode(episode)} · ${episode.title ?? `Episode ${episode.episode}`}`;

// TanStack renders cell functions as component types. Context keeps cell data
// reactive without replacing a focused control when a status or action changes.
interface EpisodePresentation {
  actions: ActionSet<EpisodeRow>;
  statuses: ContentStatusMap;
  progressByFile: EpisodeListProps["progressByFile"];
  playEpisode: (episode: EpisodeRow) => void;
  wantEpisode?: (episode: EpisodeRow) => Promise<unknown>;
  playbackUnavailable?: string;
  /** Marks a whole season unwatched, from its header. */
  seasonReset: ProgressReset<SeasonGroup<EpisodeRow>>;
  resetFiles: (mediaFileIds: readonly string[]) => Promise<void>;
}
const EpisodePresentationContext = createContext<EpisodePresentation | null>(null);
const useEpisodePresentation = () => {
  const value = useContext(EpisodePresentationContext);
  if (!value) throw new Error("Episode cells require their presentation provider");
  return value;
};
type EpisodeCellProps = { row: { original: EpisodeRow } };
function EpisodeStatusCell({ row }: EpisodeCellProps) {
  const { statuses } = useEpisodePresentation();
  const status = row.original.ignored ? statusMeta("IGNORED") : contentStatusMeta(statuses, row.original.id);
  return <Badge size="sm" marker="dot" tone={status.tone === "default" ? "neutral" : status.tone}>{status.label}</Badge>;
}
function EpisodeProgressCell({ row }: EpisodeCellProps) {
  const { progressByFile, playbackUnavailable } = useEpisodePresentation();
  const saved = row.original.mediaFileId ? progressByFile.get(row.original.mediaFileId) : undefined;
  if (playbackUnavailable && row.original.mediaFileId) return <TextCell muted>{playbackUnavailable}</TextCell>;
  if (!saved) return <TextCell muted>{row.original.mediaFileId ? "Unwatched" : "—"}</TextCell>;
  const fraction = saved.isWatched ? 1 : saved.duration && saved.duration > 0 ? Math.max(0, Math.min(1, saved.currentPosition / saved.duration)) : 0;
  return <MeterCell label={`Playback progress for ${episodeCode(row.original)}`} value={fraction * 100} max={100} valueText={saved.isWatched ? "Watched" : `${Math.round(fraction * 100)}%`} warningAt={2} dangerAt={2} tone="data" />;
}
function EpisodeActionsCell({ row: { original: episode } }: EpisodeCellProps) {
  const { actions, progressByFile, playbackUnavailable, playEpisode, wantEpisode } = useEpisodePresentation();
  const saved = episode.mediaFileId ? progressByFile.get(episode.mediaFileId) : undefined;
  const state: RowPlayState = saved?.isWatched ? "finished" : saved && saved.currentPosition > 0 ? "resume" : "ready";
  return (
    <RowActions
      set={actions}
      target={episode}
      label={episodeLabel(episode)}
      play={episode.mediaFileId ? { state, onPress: () => playEpisode(episode), disabledReason: playbackUnavailable ?? undefined } : undefined}
      want={wantEpisode && !episode.mediaFileId && !episode.ignored ? { wanted: episode.wanted, onPress: () => wantEpisode(episode) } : undefined}
    />
  );
}

function createEpisodeColumns() {
    const col = createDataTableColumns<EpisodeRow>();
    return col.columns([
      col.accessor("title", {
        header: "Title", sortable: true, filter: filters.text({ field: "title" }), cardRole: "title", priority: 1, width: 18, minWidth: 10, hideable: false,
        cell: ({ row }) => <TextCell>{row.original.title ?? `Episode ${row.original.episode}`}</TextCell>,
      }),
      col.accessor("episode", {
        header: "Episode", sortable: true, filter: filters.number({ field: "episode", server: "int" }), cardRole: "subtitle", priority: 2, width: 7, minWidth: 6, grow: false,
        cell: ({ row }) => <TextCell identifier>{episodeCode(row.original)}</TextCell>,
      }),
      col.display({
        id: "status", header: "Status", cardRole: "status", priority: 2, width: 10, minWidth: 7,
        cell: EpisodeStatusCell,
      }),
      col.accessor("airDate", {
        header: "Air date", sortable: true, filter: filters.date({ field: "airDate", storage: "calendar-date" }), priority: 3, width: 9, minWidth: 7,
        cell: ({ row }) => <TextCell>{formatDate(row.original.airDate)}</TextCell>,
      }),
      col.accessor("runtime", {
        header: "Runtime", sortable: true, filter: filters.number({ field: "runtime", server: "int", unit: "min" }), priority: 3, width: 7, minWidth: 5, align: "end", grow: false,
        cell: ({ row }) => <NumericCell value={row.original.runtime || (row.original.mediaFile?.duration ? Math.round(row.original.mediaFile.duration / 60) : null)} unit="min" />,
      }),
      col.display({
        id: "file", header: "Quality / file", priority: 2, width: 16, minWidth: 10,
        cell: ({ row }) => {
          const file = row.original.mediaFile;
          if (!file) return <TextCell muted>{row.original.mediaFileId ? "File linked" : "No file"}</TextCell>;
          return <TwoLineCell title={[file.resolution, file.videoCodec?.toUpperCase()].filter(Boolean).join(" · ")} subtitle={qualityStatus(file.qualityStatus).label} />;
        },
      }),
      col.display({
        id: "progress", header: "Progress", priority: 3, width: 10, minWidth: 7,
        cell: EpisodeProgressCell,
      }),
      // A real column keeps promoted actions visible without relying on the table's
      // hover-enhanced trailing menu; priority 1 keeps them in responsive cards too.
      col.display({
        id: "episodeActions", header: "", priority: 1, width: 7, minWidth: 6, hideable: false, grow: false,
        cell: EpisodeActionsCell,
      }),
      col.accessor("overview", {
        header: "Overview", priority: 5, cardRole: "hidden", width: 24,
        cell: ({ row }) => <p className="whitespace-normal">{row.original.overview ?? "—"}</p>,
      }),
    ]);
}
const episodeColumns = createEpisodeColumns();

/** Season tables over the parent-owned episode query, playback and acquisition handlers. */
export function EpisodeList({ focusEpisodeId, episodes, statuses, progressByFile, loading, complete = true, status, playbackUnavailable, onPlay, showId, showName, libraryId, onChanged, view = "table", tint, headerExtra }: EpisodeListProps) {
  const client = useApolloClient();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  // A link to an episode lands on it: its row scrolls into view and stays marked for a moment.
  const focusLoaded = Boolean(focusEpisodeId) && episodes.some((episode) => episode.id === focusEpisodeId);
  useEffect(() => {
    if (!focusEpisodeId || !focusLoaded) return;
    // The table draws its rows a moment after the episodes arrive.
    let tries = 0;
    let clear: ReturnType<typeof setTimeout> | undefined;
    const look = setInterval(() => {
      const row = document.querySelector<HTMLElement>(`[data-row-id="${CSS.escape(focusEpisodeId)}"]`);
      if (!row && ++tries < 40) return;
      clearInterval(look);
      if (!row) return;
      row.scrollIntoView({ block: "center", inline: "center" });
      row.classList.add("episode-focus");
      clear = setTimeout(() => row.classList.remove("episode-focus"), 4000);
    }, 100);
    return () => { clearInterval(look); if (clear) clearTimeout(clear); };
  }, [focusEpisodeId, focusLoaded]);
  const [updateEpisode] = useMutation(EntityEpisodeUpdateDocument, { refetchQueries: () => activeContentStatusQueries(client) });
  const [updateEpisodes] = useMutation(EntityEpisodeUpdateManyDocument, { refetchQueries: () => activeContentStatusQueries(client) });
  const resetProgress = useResetProgress();
  const [searching, setSearching] = useState<EpisodeRow | null>(null);
  const groups = useMemo(() => groupBySeason(episodes), [episodes]);
  const tableScope = `episodes:${user?.id ?? "anonymous"}:${libraryId}:${showId}`;

  // The shared action runner owns error feedback. Reject unsuccessful payloads before
  // telling the parent to refresh, so a failed mutation never looks successful.
  const update = useCallback(async (episode: EpisodeRow, input: UpdateEpisodeInput) => {
    if (!isAdmin) return;
    const { data } = await updateEpisode({ variables: { id: episode.id, input } });
    assertSuccess(data?.updateEpisode, "Could not update episode");
    onChanged?.();
  }, [isAdmin, updateEpisode, onChanged]);

  const bulk = useCallback(async (season: number, input: UpdateEpisodeInput, missingOnly: boolean, message: string) => {
    if (!isAdmin) return;
    const { data } = await updateEpisodes({
      variables: {
        where: missingOnly ? { showId: { eq: showId }, season: { eq: season }, mediaFileId: { isNull: true } } : { showId: { eq: showId }, season: { eq: season } },
        input,
      },
    });
    const result = assertSuccess(data?.updateEpisodes, "Could not update the season");
    toast.success(`${message} (${result.affectedCount})`);
    onChanged?.();
  }, [isAdmin, updateEpisodes, showId, onChanged]);

  const updateSelected = useCallback(async (selected: readonly EpisodeRow[], input: UpdateEpisodeInput) => {
    if (!isAdmin) throw new Error("Administrator access required");
    // The action explicitly manages missing episodes; downloaded episodes stay untouched.
    const rows = selected.filter((episode) => !episode.mediaFileId);
    if (!rows.length) throw new Error("No missing episodes selected");
    try {
      const { data } = await updateEpisodes({ variables: { where: { id: { inList: rows.map((row) => row.id) }, showId: { eq: showId }, mediaFileId: { isNull: true } }, input } });
      const result = assertSuccess(data?.updateEpisodes, "Could not update selected episodes");
      if (result.affectedCount !== rows.length) throw new Error(`Updated ${result.affectedCount} of ${rows.length} selected episodes. Refresh and retry the remaining items.`);
    } finally { onChanged?.(); }
  }, [isAdmin, updateEpisodes, showId, onChanged]);

  // Only what is on this page can be reset, so an unfinished episode list or progress read waits.
  const episodeReset = useMemo<ProgressReset<EpisodeRow>>(() => ({
    files: (rows) => filesWithProgress(rows.map((row) => row.mediaFileId), progressByFile),
    disabledReason: () => playbackUnavailable,
  }), [progressByFile, playbackUnavailable]);
  const seasonReset = useMemo<ProgressReset<SeasonGroup<EpisodeRow>>>(() => ({
    files: (seasons) => filesWithProgress(seasons.flatMap((season) => season.episodes.map((episode) => episode.mediaFileId)), progressByFile),
    disabledReason: () => (!complete ? "Loading the complete episode list" : playbackUnavailable),
  }), [progressByFile, playbackUnavailable, complete]);
  const actions = useMemo(() => defineActions<EpisodeRow>("episode", [
    {
      id: "play", label: "Play", icon: IconPlay, group: "open", primary: true, bulk: true,
      visible: (rows) => rows.length === 1 && Boolean(rows[0]?.mediaFileId),
      disabledReason: () => playbackUnavailable,
      run: ([episode]) => { if (episode?.mediaFileId) onPlay(episode); },
    },
    {
      id: "wanted", label: ([episode]) => episode?.wanted ? "Wanted" : "Want", icon: IconBookmark, group: "manage", priority: 1, bulk: true,
      visible: (rows) => rows.length === 1 && isAdmin && Boolean(rows[0] && !rows[0].mediaFileId && !rows[0].ignored),
      run: ([episode]) => episode && update(episode, { wanted: !episode.wanted }),
    },
    {
      id: "find", label: "Find", icon: IconDownload, group: "open", priority: 2, bulk: true,
      visible: (rows) => rows.length === 1 && isAdmin && Boolean(rows[0] && !rows[0].mediaFileId && !rows[0].ignored),
      run: ([episode]) => { if (isAdmin && episode) setSearching(episode); },
    },
    {
      id: "ignore", label: "Ignore", icon: IconEyeOff, group: "manage", priority: 3, bulk: true,
      visible: (rows) => rows.length === 1 && isAdmin && Boolean(rows[0] && !rows[0].mediaFileId && !rows[0].ignored),
      run: ([episode]) => episode && update(episode, { ignored: true, wanted: false }),
    },
    {
      id: "unignore", label: "Unignore", icon: IconEye, group: "manage", priority: 1, bulk: true,
      visible: (rows) => rows.length === 1 && isAdmin && Boolean(rows[0] && !rows[0].mediaFileId && rows[0].ignored),
      run: ([episode]) => episode && update(episode, { ignored: false }),
    },
    {
      id: "want-selected", label: "Want selected missing episodes", icon: IconBookmark, group: "manage", bulk: true,
      visible: (rows) => isAdmin && rows.length > 1 && rows.some((episode) => !episode.mediaFileId && (!episode.wanted || episode.ignored)),
      run: (rows) => updateSelected(rows, { wanted: true, ignored: false }), successMessage: () => "Selected missing episodes wanted",
    },
    {
      id: "unwant-selected", label: "Unwant selected missing episodes", icon: IconBookmark, group: "manage", bulk: true,
      visible: (rows) => isAdmin && rows.length > 1 && rows.some((episode) => !episode.mediaFileId && episode.wanted),
      run: (rows) => updateSelected(rows, { wanted: false }), successMessage: () => "Selected missing episodes no longer wanted",
    },
    {
      id: "ignore-selected", label: "Ignore selected missing episodes", icon: IconEyeOff, group: "manage", bulk: true,
      visible: (rows) => isAdmin && rows.length > 1 && rows.some((episode) => !episode.mediaFileId && !episode.ignored),
      run: (rows) => updateSelected(rows, { ignored: true, wanted: false }), successMessage: () => "Selected missing episodes ignored",
    },
    {
      id: "unignore-selected", label: "Unignore selected missing episodes", icon: IconEye, group: "manage", bulk: true,
      visible: (rows) => isAdmin && rows.length > 1 && rows.some((episode) => !episode.mediaFileId && episode.ignored),
      run: (rows) => updateSelected(rows, { ignored: false }), successMessage: () => "Selected missing episodes no longer ignored",
    },
    ...resetProgressAction("video", episodeReset, resetProgress),
    { id: "copy-titles", label: "Copy episode titles", icon: IconCopy, group: "organise", bulk: true,
      run: async (rows) => { await copyText(rows.map(episodeLabel).join("\n")); }, successMessage: () => "Episode titles copied" },
  ]), [isAdmin, onPlay, update, updateSelected, playbackUnavailable, episodeReset, resetProgress]);

  const columns = episodeColumns;
  const presentation = useMemo(() => ({
    actions, statuses, progressByFile, playbackUnavailable, seasonReset, resetFiles: resetProgress,
    playEpisode: onPlay,
    wantEpisode: isAdmin ? (episode: EpisodeRow) => update(episode, { wanted: !episode.wanted }) : undefined,
  }), [actions, statuses, progressByFile, playbackUnavailable, seasonReset, resetProgress, onPlay, isAdmin, update]);

  if (view === "row") {
    return (
      <EpisodePresentationContext.Provider value={presentation}>
        <div className="flex min-w-0 flex-col gap-3">
          {loading ? <EpisodeRowSkeleton /> : episodes.length === 0 ? <p className="text-body-sm text-muted">No episodes in this season</p> : groups.map((group) => (
            <EpisodeShelf key={`${group.season}:${focusEpisodeId ?? ""}`} focusEpisodeId={focusEpisodeId} group={group} isAdmin={isAdmin} onBulk={bulk} onPlay={onPlay} tint={tint} headerExtra={headerExtra} />
          ))}
          {status ? <div className="text-label-sm text-muted">{status}</div> : null}
          <ReleaseSearchDialog isOpen={Boolean(searching)} onOpenChange={(open) => !open && setSearching(null)} query={showName} season={searching?.season} episode={searching?.episode} libraryId={libraryId} target={{ showId, episodeId: searching?.id }} />
        </div>
      </EpisodePresentationContext.Provider>
    );
  }

  if (loading || episodes.length === 0) {
    return <EpisodePresentationContext.Provider value={presentation}><LoadedDataTable id={`${tableScope}:empty`} label={`${showName} episodes`} rows={[]} columns={columns} getRowId={episodeId} loading={loading} status={status} height="auto" selectable actions={actions} renderRowActions={noRowMenu} renderBulkActions={(rows) => <TableBulkActions set={actions} rows={rows} noun={episodeNoun} />} complete={complete} search={{ fields: ["title", { field: "episodeCode", value: episodeCode }], placeholder: "Search episodes" }} noun={episodeNoun} emptyMessage="No episodes in this season" defaultHiddenColumns={["overview"]} /></EpisodePresentationContext.Provider>;
  }

  return (
    <EpisodePresentationContext.Provider value={presentation}>
    <div className="flex min-w-0 flex-col gap-6">
      {groups.map((group) => (
        <section key={group.season} aria-label={`${group.label} episodes`} className="flex min-w-0 flex-col gap-2">
          <SeasonHeader group={group} isAdmin={isAdmin} onBulk={bulk} extra={headerExtra} />
          <LoadedDataTable
            id={`${tableScope}:season:${group.season}`}
            label={`${group.label} episodes`}
            rows={group.episodes}
            columns={columns}
            getRowId={episodeId}
            getRowLabel={episodeLabel}
            height="auto"
            status={status}
            selectable
            complete={complete} search={{ fields: ["title", { field: "episodeCode", value: episodeCode }], placeholder: "Search episodes" }}
            noun={episodeNoun}
            actions={actions} renderRowActions={noRowMenu}
            renderBulkActions={(rows) => <TableBulkActions set={actions} rows={rows} noun={episodeNoun} />}
            defaultHiddenColumns={["overview"]}
          />
        </section>
      ))}
      <ReleaseSearchDialog isOpen={Boolean(searching)} onOpenChange={(open) => !open && setSearching(null)} query={showName} season={searching?.season} episode={searching?.episode} libraryId={libraryId} target={{ showId, episodeId: searching?.id }} />
    </div>
    </EpisodePresentationContext.Provider>
  );
}

interface SeasonHeaderProps {
  group: SeasonGroup<EpisodeRow>;
  isAdmin: boolean;
  onBulk: (season: number, input: UpdateEpisodeInput, missingOnly: boolean, message: string) => Promise<void>;
  extra?: ReactNode;
}

function SeasonHeader({ group, isAdmin, onBulk, extra }: SeasonHeaderProps) {
  const { seasonReset, resetFiles } = useEpisodePresentation();
  const actions: ActionSet<SeasonGroup<EpisodeRow>> = useMemo(() => defineActions<SeasonGroup<EpisodeRow>>("season", [
    {
      id: "want-missing", label: "Want all missing", icon: IconBookmark, group: "manage", priority: 1,
      visible: () => isAdmin,
      disabledReason: () => group.missing === 0 ? "No missing episodes in this season" : null,
      run: ([season]) => season && onBulk(season.season, { wanted: true, ignored: false }, true, `${season.label}: missing episodes wanted`),
    },
    {
      id: "unwant", label: "Unwant all", icon: IconBookmark, group: "manage", priority: 2,
      visible: () => isAdmin,
      run: ([season]) => season && onBulk(season.season, { wanted: false }, false, `${season.label}: no longer wanted`),
    },
    {
      id: "ignore", label: group.allIgnored ? "Unignore season" : "Ignore season", icon: group.allIgnored ? IconEye : IconEyeOff, group: "manage", priority: 3,
      visible: () => isAdmin,
      run: ([season]) => season && onBulk(season.season, season.allIgnored ? { ignored: false } : { ignored: true, wanted: false }, false, `${season.label}: ${season.allIgnored ? "no longer ignored" : "ignored"}`),
    },
    // Every member can reset their own progress; the rest of the bar is administration.
    ...resetProgressAction("video", seasonReset, resetFiles, { id: "reset-progress", label: "Mark season as unwatched" }),
  ]), [group, onBulk, isAdmin, seasonReset, resetFiles]);
  const showBar = isAdmin || seasonReset.files([group]).length > 0;

  return (
    <header className="flex flex-wrap items-center gap-x-3 gap-y-2">
      <h3 className="text-title-sm text-foreground">{group.label}</h3>
      <span className="text-numeric text-label-sm text-muted">
        {group.have} of {group.total} · {group.missing} missing
        {group.wanted > 0 ? ` · ${group.wanted} wanted` : ""}
        {group.ignored > 0 ? ` · ${group.ignored} ignored` : ""}
      </span>
      {/* Season management is for pointer and keyboard; the remote moves straight to episodes. */}
      <span data-tv-skip className="ms-auto flex min-w-0 max-w-full items-center gap-2">
        {showBar ? <ActionBar set={actions} targets={[group]} label={group.label} className="min-w-0 max-w-full" /> : null}
        {extra}
      </span>
    </header>
  );
}

/** A season's episodes as a row of stills with the focused episode's summary underneath. */
function EpisodeShelf({ focusEpisodeId, group, isAdmin, onBulk, onPlay, tint, headerExtra }: { focusEpisodeId?: string; group: SeasonGroup<EpisodeRow>; isAdmin: boolean; onBulk: SeasonHeaderProps["onBulk"]; onPlay: (episode: EpisodeRow) => void; tint?: string; headerExtra?: ReactNode }) {
  const { progressByFile } = useEpisodePresentation();
  // Start on the next episode to watch; the summary follows remote focus and the pointer.
  const nextUp = group.episodes.find((episode) => episode.mediaFileId && !progressByFile.get(episode.mediaFileId)?.isWatched) ?? group.episodes[0];
  const [activeId, setActiveId] = useState<string | null>(focusEpisodeId ?? null);
  const active = group.episodes.find((episode) => episode.id === activeId) ?? nextUp;
  return (
    <section aria-label={`${group.label} episodes`} className="flex min-w-0 flex-col gap-3">
      <SeasonHeader group={group} isAdmin={isAdmin} onBulk={onBulk} extra={headerExtra} />
      <div className="detail-bleed">
        <MediaRow ariaLabel={`${group.label} episodes`}>
          {group.episodes.map((episode) => <EpisodeTile key={episode.id} episode={episode} onPlay={onPlay} tint={tint} active={episode.id === active?.id} nextUp={episode.id === nextUp?.id} onActive={setActiveId} />)}
        </MediaRow>
      </div>
      {active ? <EpisodeSummary episode={active} /> : null}
    </section>
  );
}

/**
 * One episode still. With a file, the tile plays it; without one, the tile opens its
 * actions (Want, Find, Ignore), so a remote's OK does the useful thing either way. A
 * pointer also gets the ⋯ menu on playable tiles.
 */
function EpisodeTile({ episode, onPlay, tint, active, nextUp, onActive }: { episode: EpisodeRow; onPlay: (episode: EpisodeRow) => void; tint?: string; active: boolean; nextUp: boolean; onActive: (id: string) => void }) {
  const { actions, statuses, progressByFile, playbackUnavailable } = useEpisodePresentation();
  const isAdmin = useIsAdmin();
  const saved = episode.mediaFileId ? progressByFile.get(episode.mediaFileId) : undefined;
  const fraction = saved ? (saved.isWatched ? 1 : saved.duration && saved.duration > 0 ? Math.max(0, Math.min(1, saved.currentPosition / saved.duration)) : 0) : 0;
  const state = episode.ignored ? statusMeta("IGNORED") : contentStatusMeta(statuses, episode.id);
  const title = episode.title ?? `Episode ${episode.episode}`;
  const playable = Boolean(episode.mediaFileId);
  const label = `${episodeCode(episode)} · ${title}`;
  const targetClass = "poster-navigation absolute inset-0 z-10 rounded-tile outline-none";
  const focus = { onFocus: () => onActive(episode.id), onMouseEnter: () => onActive(episode.id) };
  return (
    <article data-row-id={episode.id} data-active={active ? "true" : undefined} className="poster-card episode-tile relative flex w-[calc(var(--poster-min)*1.75)] shrink-0 flex-col xl:w-[calc(var(--poster-max)*1.6)]" {...focus}>
      {playable ? (
        <button type="button" data-focusable data-nav-default={nextUp ? "" : undefined} className={targetClass} aria-label={`Play ${label}`} aria-disabled={playbackUnavailable ? true : undefined} aria-description={playbackUnavailable}
          onClick={() => { if (!playbackUnavailable) onPlay(episode); }} />
      ) : isAdmin ? (
        <ActionMenu set={actions} targets={[episode]} label={label} align="start"
          trigger={<button type="button" data-focusable data-nav-default={nextUp ? "" : undefined} className={targetClass} aria-label={`${label} · ${state.label}`} />} />
      ) : (
        // Nothing to do yet, but a remote can still browse onto it to read its summary.
        <button type="button" data-focusable aria-disabled data-nav-default={nextUp ? "" : undefined} className={targetClass} aria-label={`${label} · ${state.label}`} />
      )}
      <Surface material="card" radius="tile" padding="none" className="poster-art pointer-events-none relative overflow-clip">
        <Artwork src={episodeThumb(episode.id)} alt="" aspect="backdrop" tint={tint} className={playable ? undefined : "opacity-55"} displayWidth={360} />
        {playable ? (
          <span aria-hidden className="over-imagery absolute inset-0 grid place-items-center">
            <Surface as="span" material="control" radius="control" padding="none" className="grid size-10 place-items-center rounded-full"><IconPlay /></Surface>
          </span>
        ) : (
          <span className="over-imagery absolute start-2 top-2"><Badge size="sm" marker="dot" tone={state.tone === "default" ? "neutral" : state.tone}>{state.label}</Badge></span>
        )}
        {saved?.isWatched ? <span className="over-imagery absolute end-2 top-2"><Badge size="sm" tone="neutral" icon={<IconCheck />}>Watched</Badge></span> : null}
        {fraction > 0 && !saved?.isWatched ? (
          <div className="over-imagery absolute inset-x-0 bottom-0"><Progress value={fraction} max={1} showValue={false} size="sm" aria-label={`Watched progress for ${episodeCode(episode)}`} /></div>
        ) : null}
      </Surface>
      {playable ? (
        <span data-tv-skip className="absolute end-1 top-1 z-20" data-print="hide"><ActionMenu set={actions} targets={[episode]} label={label} /></span>
      ) : null}
      <div className="pointer-events-none min-w-0 px-0.5 pt-2">
        <p className="truncate text-title-sm text-foreground">{episode.episode} · {title}</p>
        <p className="text-numeric truncate text-label-sm text-muted">{[formatDate(episode.airDate), episode.runtime ? `${episode.runtime} min` : null].filter(Boolean).join(" · ") || "\u00a0"}</p>
      </div>
    </article>
  );
}

/** The focused episode's facts and overview, in a reserved block so nothing moves. */
function EpisodeSummary({ episode }: { episode: EpisodeRow }) {
  const runtime = episode.runtime || (episode.mediaFile?.duration ? Math.round(episode.mediaFile.duration / 60) : null);
  return (
    <div aria-live="polite" className="min-h-[calc(var(--type-body-size)*1.5*3+var(--type-label-size)*1.5*2+var(--space-2))] max-w-3xl">
      <p className="text-numeric text-label-sm text-muted">{[episodeCode(episode), formatDate(episode.airDate), runtime ? `${runtime} min` : null].filter(Boolean).join(" · ")}</p>
      <p className="text-title-sm text-foreground">{episode.title ?? `Episode ${episode.episode}`}</p>
      <p className="mt-1 line-clamp-3 text-body text-muted">{episode.overview ?? "No summary yet."}</p>
    </div>
  );
}

function EpisodeRowSkeleton() {
  return (
    <div aria-hidden className="flex gap-(--poster-gap) overflow-hidden">
      {Array.from({ length: 5 }, (_, index) => (
        <div key={index} className="flex w-[calc(var(--poster-min)*1.75)] shrink-0 flex-col gap-2 xl:w-[calc(var(--poster-max)*1.6)]">
          <div className="skeleton aspect-video rounded-tile" />
          <div className="skeleton h-3 w-3/4" />
        </div>
      ))}
    </div>
  );
}
