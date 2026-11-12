import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { useCallback, useEffect, useMemo } from "react";

import { EntityMediaFileGetDocument, EpisodeDetailDocument, MovieDetailDocument } from "@/graphql/generated/graphql";

import { episodePlayItem, moviePlayItem } from "./playItems";
import { canAdvance, currentQueueItem, playerStore, usePlayerState, type PlayItem } from "./store";
import { VideoPlayer } from "./VideoPlayer";

interface WatchPageProps {
  mediaFileId: string;
  onBack: () => void;
  mini?: boolean;
  userId?: string;
  onMinimise?: () => void;
  onExpand?: () => void;
  onClose?: () => void;
}

/**
 * Resolves the item to play when the route is opened directly (deep link, refresh) and wires
 * Next and the end of a video to the play queue. Audio playback is paused while video plays.
 */
export function WatchPage({ mediaFileId, onBack, mini = false, userId, onMinimise, onExpand, onClose }: WatchPageProps) {
  const state = usePlayerState();
  const known = state.video?.mediaFileId === mediaFileId ? state.video : null;

  const file = useQuery(EntityMediaFileGetDocument, { variables: { id: mediaFileId }, skip: Boolean(known) });
  const movieId = file.data?.mediaFile?.movieId ?? null;
  const episodeId = file.data?.mediaFile?.episodeId ?? null;
  const movie = useQuery(MovieDetailDocument, { variables: { id: movieId ?? "" }, skip: !movieId });
  const episode = useQuery(EpisodeDetailDocument, { variables: { id: episodeId ?? "" }, skip: !episodeId });

  const resolved = useMemo<PlayItem | null>(() => {
    if (known) return known;
    if (movie.data?.movie) return moviePlayItem({ ...movie.data.movie, mediaFileId });
    if (episode.data?.episode?.show) return episodePlayItem({ ...episode.data.episode, mediaFileId }, episode.data.episode.show);
    return null;
  }, [known, movie.data, episode.data, mediaFileId]);

  // Opened directly (a link, a refresh): this is the queue, so what follows it can play on.
  useEffect(() => {
    if (resolved && !known) playerStore.playQueue([resolved], 0);
  }, [resolved, known]);

  useEffect(() => {
    playerStore.updateTransport({ playing: false });
  }, []);

  // Next and the end of a video both move the queue on, which also switches to the audio dock
  // when what is next is audio. The next entry is the queue's own; at the queue's end the
  // continuation (the rest of the show) is looked up when it is asked for.
  const queued = currentQueueItem(state)?.mediaFileId === resolved?.mediaFileId;
  const upcoming = queued ? state.queue[state.index + 1] : undefined;
  const hasNext = queued && canAdvance(state);
  const nextLabel = !hasNext ? undefined : upcoming ? (upcoming.entity.kind === "episode" ? `Next: ${/E\d+/.exec(upcoming.subtitle ?? "")?.[0] ?? upcoming.title}` : `Next: ${upcoming.title}`) : "Next episode";
  const advance = useCallback(() => { void playerStore.advance(); }, []);
  const skip = useCallback(() => playerStore.skip(1), []);

  if (!resolved) {
    return <div className="h-dvh w-full bg-black" aria-busy />;
  }

  return (
    <VideoPlayer
      key={resolved.mediaFileId}
      item={resolved}
      onBack={onBack}
      onNext={hasNext ? skip : undefined}
      nextLabel={nextLabel}
      onEnded={advance}
      mini={mini}
      userId={userId}
      onMinimise={onMinimise}
      onExpand={onExpand}
      onClose={onClose}
    />
  );
}
