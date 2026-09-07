import { useQuery } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo } from "react";

import { EntityMediaFileGetDocument, EpisodeDetailDocument, MovieDetailDocument, ShowEpisodesDocument } from "@/graphql/generated/graphql";
import { moviePoster, showPoster } from "@/lib/artwork";

import { playerStore, usePlayerState, type PlayItem } from "./store";
import { VideoPlayer } from "./VideoPlayer";

interface WatchPageProps {
  mediaFileId: string;
  onBack: () => void;
}

/**
 * Resolves the item to play when the route is opened directly (deep link, refresh) and wires
 * "next episode" for shows. Audio playback is paused while video plays.
 */
export function WatchPage({ mediaFileId, onBack }: WatchPageProps) {
  const navigate = useNavigate();
  const state = usePlayerState();
  const known = state.video?.mediaFileId === mediaFileId ? state.video : null;

  const file = useQuery(EntityMediaFileGetDocument, { variables: { id: mediaFileId }, skip: Boolean(known) });
  const movieId = file.data?.mediaFile?.movieId ?? null;
  const episodeId = file.data?.mediaFile?.episodeId ?? null;
  const movie = useQuery(MovieDetailDocument, { variables: { id: movieId ?? "" }, skip: !movieId });
  const episode = useQuery(EpisodeDetailDocument, { variables: { id: episodeId ?? "" }, skip: !episodeId });

  const resolved = useMemo<PlayItem | null>(() => {
    if (known) return known;
    if (movie.data?.movie) {
      const item = movie.data.movie;
      return { mediaFileId, title: item.title, subtitle: item.year ? String(item.year) : undefined, artwork: moviePoster(item.id), entity: { kind: "movie", id: item.id }, href: `/movies/${item.id}` };
    }
    if (episode.data?.episode) {
      const item = episode.data.episode;
      const code = `S${String(item.season).padStart(2, "0")} · E${String(item.episode).padStart(2, "0")}`;
      return {
        mediaFileId,
        title: item.show?.name ?? item.title ?? "Episode",
        subtitle: [code, item.title].filter(Boolean).join("  "),
        artwork: item.show ? showPoster(item.show) : undefined,
        entity: { kind: "episode", id: item.id },
        href: `/shows/${item.showId}`,
      };
    }
    return null;
  }, [known, movie.data, episode.data, mediaFileId]);

  useEffect(() => {
    if (resolved && !known) playerStore.setVideo(resolved);
  }, [resolved, known]);

  useEffect(() => {
    playerStore.updateTransport({ playing: false });
  }, []);

  // Next episode lookup for shows.
  const showId = resolved?.entity.kind === "episode" ? (episode.data?.episode?.showId ?? state.video?.href?.split("/").pop() ?? null) : null;
  const episodes = useQuery(ShowEpisodesDocument, { variables: { showId: showId ?? "" }, skip: !showId });
  const nextEpisode = useMemo(() => {
    if (!resolved || resolved.entity.kind !== "episode") return null;
    const list = episodes.data?.episodes.edges.map((edge) => edge.node) ?? [];
    const index = list.findIndex((candidate) => candidate.id === resolved.entity.id);
    return list.slice(index + 1).find((candidate) => candidate.mediaFileId) ?? null;
  }, [episodes.data, resolved]);

  const playNext = useCallback(() => {
    if (!nextEpisode?.mediaFileId || !resolved) return;
    const code = `S${String(nextEpisode.season).padStart(2, "0")} · E${String(nextEpisode.episode).padStart(2, "0")}`;
    playerStore.setVideo({ mediaFileId: nextEpisode.mediaFileId, title: resolved.title, subtitle: [code, nextEpisode.title].filter(Boolean).join("  "), artwork: resolved.artwork, entity: { kind: "episode", id: nextEpisode.id }, href: resolved.href });
    void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: nextEpisode.mediaFileId }, replace: true });
  }, [navigate, nextEpisode, resolved]);

  if (!resolved) {
    return <div className="h-svh w-full bg-black" aria-busy />;
  }

  return (
    <VideoPlayer
      key={resolved.mediaFileId}
      item={resolved}
      onBack={onBack}
      onNext={nextEpisode ? playNext : undefined}
      nextLabel={nextEpisode ? `Next: E${String(nextEpisode.episode).padStart(2, "0")}` : undefined}
      onEnded={nextEpisode ? playNext : undefined}
    />
  );
}
