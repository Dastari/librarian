import { IconInfo } from "glass-ux/icons";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Link, useNavigate } from "@tanstack/react-router";
import { useMemo, useState } from "react";

import { Button, EmptyState, ErrorState, HeroFacts, MediaRow, MetaLine, PosterCard, Section, SkeletonHero, ViewToggle, type ViewMode } from "@/components/ui";
import { AcquisitionChip } from "@/features/acquisition/AcquisitionChip";
import { autoDownloadMeta } from "@/features/acquisition/mode";
import { CastAndCrew, CastRow, useTitleCredits } from "@/features/people/CastAndCrew";
import { episodePlayItem } from "@/features/player/playItems";
import type { PlayItem } from "@/features/player/store";
import { usePlayer } from "@/features/player/usePlayer";
import { EntitySeasonListDocument, ShowDetailDocument, type ShowEpisodesQuery } from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { seasonPoster, showBackdrop, showPoster } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatRuntime } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { usePref } from "@/lib/prefs";


import { filesWithProgress, type ProgressReset } from "./progressReset";
import { useMediaProgress } from "./useMediaProgress";
import { useShowEpisodes } from "./useShowEpisodes";
import { MediaActions } from "./MediaActions";
import { useTitleDetailFacts } from "./titleDetailFacts";
import { useShowEntity } from "./useShowEntity";
import { DetailShell } from "./DetailShell";
import { EpisodeList } from "./EpisodeList";

export type EpisodeRow = ShowEpisodesQuery["episodes"]["edges"][number]["node"];

export function ShowPage({ showId, initialSeason, focusEpisodeId }: {
  showId: string;
  /** The season to open on, when a link asks for one. */
  initialSeason?: number;
  /** An episode to bring into view and mark. */
  focusEpisodeId?: string;
}) {
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
  const linkedSeason = initialSeason !== undefined && seasons.includes(initialSeason) ? initialSeason : null;
  const activeSeason = chosenSeason ?? linkedSeason ?? seasons.find((value) => value > 0) ?? seasons[0] ?? 1;
  const seasonEpisodes = useMemo(() => list.filter((episode) => episode.season === activeSeason), [list, activeSeason]);

  const ids = useMemo(() => seasonEpisodes.map((episode) => episode.id), [seasonEpisodes]);
  const { credits } = useTitleCredits({ showId: { eq: showId } });
  // The chosen season's guest stars, across its episodes.
  const guests = useTitleCredits({ episodeId: { inList: ids }, guest: { eq: true } }, ids.length === 0);
  const details = useTitleDetailFacts({ showId: { eq: showId } }, { networks: true });
  const statuses = useContentStatuses("EPISODE", ids);
  const fileIds = useMemo(() => list.map((episode) => episode.mediaFileId).filter((id): id is string => Boolean(id)), [list]);
  const progress = useMediaProgress(fileIds, user?.id);
  const { progressByFile } = progress;

  const [episodeView, setEpisodeView] = usePref<ViewMode>("show.episodes.view", "card");

  const nextUp = list.find((episode) => episode.mediaFileId && !progressByFile.get(episode.mediaFileId!)?.isWatched) ?? list.find((episode) => episode.mediaFileId);
  const play = (episode: EpisodeRow) => {
    if (!show || !episode.mediaFileId || progress.loading || progress.error) return;
    const saved = progressByFile.get(episode.mediaFileId);
    const first = episodePlayItem(episode, show, saved && !saved.isWatched && saved.currentPosition > 5 ? saved.currentPosition : undefined);
    if (!first) return;
    // The rest of the show follows in the queue; if the list is still loading, the queue finds them when it runs out.
    const at = list.findIndex((candidate) => candidate.id === episode.id);
    const following = episodes.complete && at >= 0 ? list.slice(at + 1).map((candidate) => episodePlayItem(candidate, show)).filter((item): item is PlayItem => item !== null) : [];
    player.playVideo(first, following);
    void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: episode.mediaFileId } });
  };

  const playNext = nextUp?.mediaFileId ? { label: `${(progressByFile.get(nextUp.mediaFileId!)?.currentPosition ?? 0) > 5 && !progressByFile.get(nextUp.mediaFileId!)?.isWatched ? "Resume" : "Play"} S${String(nextUp.season).padStart(2, "0")}E${String(nextUp.episode).padStart(2, "0")}`, run: () => play(nextUp), disabledReason: !episodes.complete ? "Loading the complete episode list" : progress.loading ? "Loading watch progress" : progress.error ? "Watch progress could not be loaded" : undefined } : undefined;
  // The whole show: every episode's file, so it waits for the complete list and for progress.
  const reset = useMemo<ProgressReset<unknown>>(() => ({
    files: () => filesWithProgress(fileIds, progressByFile),
    disabledReason: () => (!episodes.complete ? "Loading the complete episode list" : progress.loading ? "Loading watch progress" : progress.error ? "Watch progress could not be loaded" : undefined),
  }), [fileIds, progressByFile, episodes.complete, progress.loading, progress.error]);
  const { actions, dialogs, openSettings } = useShowEntity<NonNullable<typeof show>>({ open: false, play: playNext, season: activeSeason, onRefreshed: () => { void episodes.refetch().catch(() => {}); }, reset });

  if (error && !show) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!show) return loading ? <SkeletonHero /> : <EmptyState icon={IconInfo} title="Show not found" description="This item may have been removed from the library." className="page-gutter my-8" />;

  const downloaded = list.filter((episode) => episode.mediaFileId).length;

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
            <MetaLine items={[show.year, show.network, show.status ?? show.contentRating, show.runtime ? formatRuntime(show.runtime, "minutes") : null, episodes.complete ? `${downloaded}/${list.length} episodes` : "Loading episodes"]} />
            <AcquisitionChip status={autoDownloadMeta(show.autoDownloadMode)} onPress={isAdmin ? () => openSettings(show, "downloads") : undefined} />
          </>
        ),
        description: show.overview,
        facts: <HeroFacts items={[
          { label: "Genres", value: show.genres.join(", ") },
          ...details.facts.filter((fact) => !(fact.label === "Network" && show.network) && !(fact.label === "Rated" && show.contentRating)),
          { label: "Links", value: show.tvmazeId || show.imdbId ? <ProviderLinks tvmazeId={show.tvmazeId} imdbId={show.imdbId} /> : null },
        ]} />,
        actions: (
          <MediaActions target={show} label={show.name} actions={actions} play={playNext} />
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
          <EpisodeList focusEpisodeId={focusEpisodeId} view={episodeView === "card" ? "row" : "table"} tint={LIBRARY_TYPES.tv.tintVar}
            headerExtra={<ViewToggle value={episodeView} onChange={setEpisodeView} className="shrink-0" />}
            complete={episodes.complete} episodes={seasonEpisodes} statuses={statuses} progressByFile={progressByFile} loading={episodes.loading && list.length === 0} playbackUnavailable={progress.loading ? "Loading watch progress" : progress.error ? "Retry watch progress" : undefined}
            status={episodes.error || progress.error ? <span role="status">Could not load {episodes.error ? "all episodes" : "watch progress"}. <Button size="sm" onPress={() => { void episodes.refetch().catch(() => {}); void progress.refetch().catch(() => {}); }}>Retry</Button></span> : !episodes.complete || progress.loading ? "Loading episodes and watch progress…" : undefined}
            onPlay={play} showId={show.id} showName={show.name} libraryId={show.libraryId} ownerId={show.userId} onChanged={() => { void episodes.refetch().catch(() => {}); }} />
        </section>
      </div>

      <CastAndCrew credits={credits} castTitle="Cast" showEpisodes />

      <CastRow credits={guests.credits} guest title={activeSeason === 0 ? "Guest stars in the specials" : `Guest stars in season ${activeSeason}`} />

      {dialogs}
    </DetailShell>
  );
}

function ProviderLinks({ tvmazeId, imdbId }: { tvmazeId?: number | null; imdbId?: string | null }) {
  const link = "nav-focus rounded text-brand hover:underline";
  return (
    <span className="flex flex-wrap gap-x-3">
      {tvmazeId ? <a className={link} href={`https://www.tvmaze.com/shows/${tvmazeId}`} target="_blank" rel="noreferrer">TVmaze</a> : null}
      {imdbId ? <a className={link} href={`https://www.imdb.com/title/${imdbId}`} target="_blank" rel="noreferrer">IMDb</a> : null}
    </span>
  );
}
