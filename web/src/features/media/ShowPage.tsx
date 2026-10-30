import { IconInfo } from "glass-ux/icons";
import { useMutation } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { toast } from "glass-ux/toast";
import { Link, useNavigate } from "@tanstack/react-router";
import { useMemo, useState } from "react";

import { Button, ConfirmDialog, EmptyState, ErrorState, HeroFacts, KeyValueList, MediaRow, MetaLine, Panel, PosterCard, Section, SkeletonHero, ViewToggle, type ViewMode } from "@/components/ui";
import { AcquisitionChip } from "@/features/acquisition/AcquisitionChip";
import { AcquisitionDialog } from "@/features/acquisition/AcquisitionDialog";
import { autoDownloadMeta } from "@/features/acquisition/mode";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { usePlayer } from "@/features/player/usePlayer";
import { EntitySeasonListDocument, EntityShowDeleteDocument, RefreshShowDocument, ShowDetailDocument, type ShowEpisodesQuery } from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { seasonPoster, showBackdrop, showPoster } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatDate, formatRuntime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { usePref } from "@/lib/prefs";


import { useMediaProgress } from "./useMediaProgress";
import { useShowEpisodes } from "./useShowEpisodes";
import { MediaActions } from "./MediaActions";
import { DetailShell } from "./DetailShell";
import { EpisodeList } from "./EpisodeList";

export type EpisodeRow = ShowEpisodesQuery["episodes"]["edges"][number]["node"];

export function ShowPage({ showId }: { showId: string }) {
  const navigate = useNavigate();
  const player = usePlayer();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const { data, previousData, loading, error, refetch } = useQuery(ShowDetailDocument, { variables: { id: showId } });
  const candidate = (data ?? previousData)?.show;
  const show = candidate?.id === showId ? candidate : null;
  const episodes = useShowEpisodes(showId);
  // Provider seasons carry names, dates and cached posters; episodes decide which seasons show.
  const seasonRows = useQuery(EntitySeasonListDocument, { variables: { where: { showId: { eq: showId } }, orderBy: [{ season: "ASC" }], page: { limit: 100, offset: 0 } } });
  const seasonMeta = useMemo(() => new Map((seasonRows.data?.seasons.edges ?? []).map((edge) => [edge.node.season, edge.node])), [seasonRows.data]);
  const list = episodes.rows;
  const seasons = useMemo(() => [...new Set(list.map((episode) => episode.season))].sort((a, b) => a - b), [list]);
  const [season, setSeason] = useState<{ showId: string; value: number } | null>(null);
  const chosenSeason = season?.showId === showId && seasons.includes(season.value) ? season.value : null;
  const activeSeason = chosenSeason ?? seasons.find((value) => value > 0) ?? seasons[0] ?? 1;
  const seasonEpisodes = useMemo(() => list.filter((episode) => episode.season === activeSeason), [list, activeSeason]);

  const ids = useMemo(() => seasonEpisodes.map((episode) => episode.id), [seasonEpisodes]);
  const statuses = useContentStatuses("EPISODE", ids);
  const fileIds = useMemo(() => list.map((episode) => episode.mediaFileId).filter((id): id is string => Boolean(id)), [list]);
  const progress = useMediaProgress(fileIds, user?.id);
  const { progressByFile } = progress;

  const [refreshShow, { loading: refreshing }] = useMutation(RefreshShowDocument);
  const [deleteShow, { loading: deleting }] = useMutation(EntityShowDeleteDocument);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [searching, setSearching] = useState(false);
  const [acquisition, setAcquisition] = useState(false);
  const [episodeView, setEpisodeView] = usePref<ViewMode>("show.episodes.view", "card");

  if (error && !show) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!show) return loading ? <SkeletonHero /> : <EmptyState icon={IconInfo} title="Show not found" description="This item may have been removed from the library." className="page-gutter my-8" />;

  const downloaded = list.filter((episode) => episode.mediaFileId).length;
  const nextUp = list.find((episode) => episode.mediaFileId && !progressByFile.get(episode.mediaFileId!)?.isWatched) ?? list.find((episode) => episode.mediaFileId);

  const play = (episode: EpisodeRow) => {
    if (!episode.mediaFileId || progress.loading || progress.error) return;
    const code = `S${String(episode.season).padStart(2, "0")} · E${String(episode.episode).padStart(2, "0")}`;
    const saved = progressByFile.get(episode.mediaFileId);
    player.playVideo({ mediaFileId: episode.mediaFileId, title: show.name, subtitle: [code, episode.title].filter(Boolean).join("  "), artwork: showPoster(show), entity: { kind: "episode", id: episode.id }, href: `/shows/${show.id}`, startPosition: saved && !saved.isWatched && saved.currentPosition > 5 ? saved.currentPosition : undefined });
    void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: episode.mediaFileId } });
  };

  const refresh = async () => {
    const { data: result } = await refreshShow({ variables: { id: show.id } });
    assertSuccess(result?.refreshShow, "Could not refresh metadata");
    await episodes.refetch();
  };

  const remove = async () => {
    try {
      const { data: result } = await deleteShow({ variables: { id: show.id } });
      assertSuccess(result?.deleteShow, "Could not remove show");
      toast.success("Show removed from the library");
      await navigate({ to: "/libraries/$libraryId/shows", params: { libraryId: show.libraryId } });
    } catch (mutationError) {
      toast.danger(errorMessage(mutationError));
    }
  };

  return (
    <DetailShell
      backdrop={showBackdrop(show)}
      hero={{
        poster: showPoster(show),
        tint: LIBRARY_TYPES.tv.tintVar,
        eyebrow: show.library ? (
          <Link to="/libraries/$libraryId/shows" params={{ libraryId: show.library.id }} className="nav-focus rounded hover:underline">
            {show.library.name}
          </Link>
        ) : null,
        title: show.name,
        meta: (
          <>
            <MetaLine items={[show.year, show.network, show.contentRating, show.runtime ? formatRuntime(show.runtime, "minutes") : null, episodes.complete ? `${downloaded}/${list.length} episodes` : "Loading episodes"]} />
            <AcquisitionChip status={autoDownloadMeta(show.autoDownloadMode)} onPress={isAdmin ? () => setAcquisition(true) : undefined} />
          </>
        ),
        description: show.overview,
        facts: <HeroFacts items={[{ label: "Genres", value: show.genres.join(", ") }]} />,
        actions: (
          <MediaActions
            personal={show.userId === user?.id ? { kind: "show", id: show.id } : undefined}
            id={show.id}
            label={show.name}
            play={nextUp?.mediaFileId ? { label: `${(progressByFile.get(nextUp.mediaFileId!)?.currentPosition ?? 0) > 5 && !progressByFile.get(nextUp.mediaFileId!)?.isWatched ? "Resume" : "Play"} S${String(nextUp.season).padStart(2, "0")}E${String(nextUp.episode).padStart(2, "0")}`, run: () => play(nextUp), disabledReason: !episodes.complete ? "Loading the complete episode list" : progress.loading ? "Loading watch progress" : progress.error ? "Watch progress could not be loaded" : undefined } : undefined}
            find={isAdmin ? () => setSearching(true) : undefined}
            settings={isAdmin ? () => setAcquisition(true) : undefined}
            refresh={isAdmin ? { pending: refreshing, run: refresh } : undefined}
            remove={isAdmin ? () => setConfirmDelete(true) : undefined}
          />
        ),
      }}
    >
      {/* One section: seasons and the chosen season's episodes share the screen. */}
      <div className="flex min-w-0 flex-col gap-8">
        {seasons.length > 1 ? (
          <Section title="Seasons" bleed>
            <MediaRow ariaLabel="Seasons">
              {seasons.map((value) => {
                const inSeason = list.filter((episode) => episode.season === value);
                const info = seasonMeta.get(value);
                const label = value === 0 ? "Specials" : `Season ${value}`;
                const year = info?.premiereDate?.slice(0, 4);
                return (
                  <PosterCard key={value} width="compact" title={info?.name ? `${label} · ${info.name}` : label} selected={value === activeSeason}
                    meta={[year, `${inSeason.filter((episode) => episode.mediaFileId).length} of ${info?.episodeOrder ?? inSeason.length} episodes`].filter(Boolean).join(" · ")}
                    image={seasonPoster(info, show)} tint={LIBRARY_TYPES.tv.tintVar} badge={info?.posterUrl ? undefined : value === 0 ? "SP" : `S${value}`}
                    onPress={() => setSeason({ showId, value })} />
                );
              })}
            </MediaRow>
          </Section>
        ) : null}

        <section className="flex flex-col gap-4" aria-label="Episodes">
          <EpisodeList view={episodeView === "card" ? "row" : "table"} tint={LIBRARY_TYPES.tv.tintVar}
            headerExtra={<ViewToggle value={episodeView} onChange={setEpisodeView} className="shrink-0" />}
            complete={episodes.complete} episodes={seasonEpisodes} statuses={statuses} progressByFile={progressByFile} loading={episodes.loading && list.length === 0} playbackUnavailable={progress.loading ? "Loading watch progress" : progress.error ? "Retry watch progress" : undefined}
            status={episodes.error || progress.error ? <span role="status">Could not load {episodes.error ? "all episodes" : "watch progress"}. <Button size="sm" onPress={() => { void episodes.refetch().catch(() => {}); void progress.refetch().catch(() => {}); }}>Retry</Button></span> : !episodes.complete || progress.loading ? "Loading episodes and watch progress…" : undefined}
            onPlay={play} showId={show.id} showName={show.name} libraryId={show.libraryId} onChanged={() => { void episodes.refetch().catch(() => {}); }} />
        </section>
      </div>

      <Panel title="Details" className="lg:max-w-2xl">
        <KeyValueList
          columns={2}
          items={[
            { label: "Genres", value: show.genres.join(", ") || undefined },
            { label: "Network", value: show.network },
            { label: "Rating", value: show.contentRating },
            { label: "Folder", value: show.path, mono: true },
            { label: "TVmaze", value: show.tvmazeId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://www.tvmaze.com/shows/${show.tvmazeId}`} target="_blank" rel="noreferrer">{show.tvmazeId}</a> : undefined },
            { label: "IMDb", value: show.imdbId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://www.imdb.com/title/${show.imdbId}`} target="_blank" rel="noreferrer">{show.imdbId}</a> : undefined },
            { label: "Added", value: formatDate(show.createdAt) },
          ]}
        />
      </Panel>

      <ConfirmDialog isOpen={confirmDelete} onOpenChange={setConfirmDelete} title={`Remove ${show.name}?`} description="The show and its episodes leave the catalogue. Files stay on disk." confirmLabel="Remove" destructive isPending={deleting} onConfirm={remove} />
      <AcquisitionDialog isOpen={acquisition} onOpenChange={setAcquisition} target={{ kind: "show", id: show.id, title: show.name, libraryId: show.libraryId, autoDownloadMode: show.autoDownloadMode, qualityProfileId: show.qualityProfileId }} onSaved={() => void refetch()} />
      <ReleaseSearchDialog isOpen={searching} onOpenChange={setSearching} query={show.name} imdbId={show.imdbId} season={activeSeason} libraryId={show.libraryId} target={{ showId: show.id }} />
    </DetailShell>
  );
}
