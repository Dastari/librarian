import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { IconAdjustments, IconDownload, IconPlayerPlayFilled, IconRefresh, IconTrash } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { ConfirmDialog, ErrorState, GlassButton, KeyValueList, MetaLine, Panel, SegmentTabs, SkeletonHero } from "@/components/ui";
import { AcquisitionChip } from "@/features/acquisition/AcquisitionChip";
import { AcquisitionDialog } from "@/features/acquisition/AcquisitionDialog";
import { autoDownloadMeta } from "@/features/acquisition/mode";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { usePlayer } from "@/features/player/usePlayer";
import { EntityShowDeleteDocument, PlaybackProgressForFilesDocument, RefreshShowDocument, ShowDetailDocument, ShowEpisodesDocument, type ShowEpisodesQuery } from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { showBackdrop, showPoster } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatDate, formatRuntime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";


import { DetailShell } from "./DetailShell";
import { EpisodeList } from "./EpisodeList";

export type EpisodeRow = ShowEpisodesQuery["episodes"]["edges"][number]["node"];

export function ShowPage({ showId }: { showId: string }) {
  const navigate = useNavigate();
  const player = usePlayer();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const { data, previousData, loading, error, refetch } = useQuery(ShowDetailDocument, { variables: { id: showId } });
  const show = (data ?? previousData)?.show ?? null;
  const episodes = useQuery(ShowEpisodesDocument, { variables: { showId } });
  const list = useMemo(() => episodes.data?.episodes.edges.map((edge) => edge.node) ?? [], [episodes.data]);
  const seasons = useMemo(() => [...new Set(list.map((episode) => episode.season))].sort((a, b) => a - b), [list]);
  const [season, setSeason] = useState<number | null>(null);
  const activeSeason = season ?? seasons.find((value) => value > 0) ?? seasons[0] ?? 1;
  const seasonEpisodes = useMemo(() => list.filter((episode) => episode.season === activeSeason), [list, activeSeason]);

  const ids = useMemo(() => seasonEpisodes.map((episode) => episode.id), [seasonEpisodes]);
  const statuses = useContentStatuses("EPISODE", ids);
  const fileIds = useMemo(() => seasonEpisodes.map((episode) => episode.mediaFileId).filter((id): id is string => Boolean(id)), [seasonEpisodes]);
  const progress = useQuery(PlaybackProgressForFilesDocument, { variables: { userId: user?.id ?? "", mediaFileIds: fileIds }, skip: !user || fileIds.length === 0 });
  const progressByFile = useMemo(() => new Map(progress.data?.playbackProgresses.edges.map((edge) => [edge.node.mediaFileId, edge.node]) ?? []), [progress.data]);

  const [refreshShow, { loading: refreshing }] = useMutation(RefreshShowDocument);
  const [deleteShow, { loading: deleting }] = useMutation(EntityShowDeleteDocument);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [searching, setSearching] = useState(false);
  const [acquisition, setAcquisition] = useState(false);

  if (error && !show) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!show) return loading ? <SkeletonHero /> : null;

  const downloaded = list.filter((episode) => episode.mediaFileId).length;
  const nextUp = list.find((episode) => episode.mediaFileId && !progressByFile.get(episode.mediaFileId!)?.isWatched) ?? list.find((episode) => episode.mediaFileId);

  const play = (episode: EpisodeRow) => {
    if (!episode.mediaFileId) return;
    const code = `S${String(episode.season).padStart(2, "0")} · E${String(episode.episode).padStart(2, "0")}`;
    const saved = progressByFile.get(episode.mediaFileId);
    player.playVideo({ mediaFileId: episode.mediaFileId, title: show.name, subtitle: [code, episode.title].filter(Boolean).join("  "), artwork: showPoster(show), entity: { kind: "episode", id: episode.id }, href: `/shows/${show.id}`, startPosition: saved && !saved.isWatched && saved.currentPosition > 5 ? saved.currentPosition : undefined });
    void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: episode.mediaFileId } });
  };

  const refresh = async () => {
    try {
      const { data: result } = await refreshShow({ variables: { id: show.id } });
      assertSuccess(result?.refreshShow, "Could not refresh");
      toast.success("Show metadata refreshed");
      void episodes.refetch();
    } catch (mutationError) {
      toast.danger(errorMessage(mutationError));
    }
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
      hero={{
        backdrop: showBackdrop(show),
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
            <MetaLine items={[show.year, show.network, show.contentRating, show.runtime ? formatRuntime(show.runtime, "minutes") : null, `${downloaded}/${list.length} episodes`]} />
            <AcquisitionChip status={autoDownloadMeta(show.autoDownloadMode)} onPress={() => setAcquisition(true)} />
          </>
        ),
        description: show.overview,
        actions: (
          <>
            {nextUp?.mediaFileId ? (
              <GlassButton emphasis="brand" size="lg" refract onPress={() => play(nextUp)}>
                <IconPlayerPlayFilled /> Play S{String(nextUp.season).padStart(2, "0")}E{String(nextUp.episode).padStart(2, "0")}
              </GlassButton>
            ) : null}
            {isAdmin ? (
              <>
                <GlassButton size="lg" refract onPress={() => setSearching(true)}>
                  <IconDownload /> Find releases
                </GlassButton>
                <GlassButton size="lg" refract isIconOnly aria-label="Download settings" onPress={() => setAcquisition(true)}>
                  <IconAdjustments />
                </GlassButton>
                <GlassButton size="lg" refract isIconOnly aria-label="Refresh metadata" onPress={() => void refresh()} isDisabled={refreshing}>
                  <IconRefresh className={refreshing ? "animate-spin" : undefined} />
                </GlassButton>
                <GlassButton size="lg" refract isIconOnly aria-label="Remove from library" onPress={() => setConfirmDelete(true)}>
                  <IconTrash />
                </GlassButton>
              </>
            ) : null}
          </>
        ),
      }}
    >
      <section className="flex flex-col gap-4">
        <SegmentTabs
          ariaLabel="Seasons"
          items={seasons.map((value) => ({ key: String(value), label: value === 0 ? "Specials" : `Season ${value}`, count: list.filter((episode) => episode.season === value).length }))}
          selected={String(activeSeason)}
          onSelect={(key) => setSeason(Number(key))}
        />
        <EpisodeList episodes={seasonEpisodes} statuses={statuses} progressByFile={progressByFile} loading={episodes.loading && list.length === 0} onPlay={play} showId={show.id} showName={show.name} libraryId={show.libraryId} onChanged={() => void episodes.refetch()} />
      </section>

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
