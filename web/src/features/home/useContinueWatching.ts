import { useQuery } from "@apollo/client/react";
import { useMemo } from "react";

import {
  ChaptersByIdsDocument,
  ContinueWatchingDocument,
  EpisodesByIdsDocument,
  MediaFilesByIdsDocument,
  MoviesByIdsDocument,
  TracksByIdsDocument,
} from "@/graphql/generated/graphql";
import { albumCover, audiobookCover, movieBackdrop, showBackdrop } from "@/lib/artwork";
import { useSession } from "@/lib/auth/useSession";
import { isPresent } from "@/lib/utils";

export type ResumeKind = "movie" | "episode" | "track" | "chapter";

export interface ResumeItem {
  key: string;
  kind: ResumeKind;
  mediaFileId: string;
  title: string;
  subtitle?: string;
  image: string | undefined;
  progress: number;
  position: number;
  duration: number | undefined;
  /** Route to the owning entity's detail page. */
  href: string;
  updatedAt: string;
}

/**
 * Resolves in-progress playback into displayable items. Progress rows reference media files;
 * media files reference exactly one of movie/episode/track/chapter.
 */
export function useContinueWatching() {
  const { user } = useSession();
  const progress = useQuery(ContinueWatchingDocument, { variables: { userId: user?.id ?? "" }, skip: !user });
  const rows = progress.data?.playbackProgresses.edges.map((edge) => edge.node) ?? [];
  const fileIds = rows.map((row) => row.mediaFileId).filter(isPresent);

  const files = useQuery(MediaFilesByIdsDocument, { variables: { ids: fileIds }, skip: fileIds.length === 0 });
  const fileList = files.data?.mediaFiles.edges.map((edge) => edge.node) ?? [];
  const movieIds = fileList.map((file) => file.movieId).filter(isPresent);
  const episodeIds = fileList.map((file) => file.episodeId).filter(isPresent);
  const trackIds = fileList.map((file) => file.trackId).filter(isPresent);
  const chapterIds = fileList.map((file) => file.chapterId).filter(isPresent);

  const movies = useQuery(MoviesByIdsDocument, { variables: { ids: movieIds }, skip: movieIds.length === 0 });
  const episodes = useQuery(EpisodesByIdsDocument, { variables: { ids: episodeIds }, skip: episodeIds.length === 0 });
  const tracks = useQuery(TracksByIdsDocument, { variables: { ids: trackIds }, skip: trackIds.length === 0 });
  const chapters = useQuery(ChaptersByIdsDocument, { variables: { ids: chapterIds }, skip: chapterIds.length === 0 });

  const items = useMemo<ResumeItem[]>(() => {
    const byFile = new Map(fileList.map((file) => [file.id, file]));
    const movieMap = new Map(movies.data?.movies.edges.map((edge) => [edge.node.id, edge.node]) ?? []);
    const episodeMap = new Map(episodes.data?.episodes.edges.map((edge) => [edge.node.id, edge.node]) ?? []);
    const trackMap = new Map(tracks.data?.tracks.edges.map((edge) => [edge.node.id, edge.node]) ?? []);
    const chapterMap = new Map(chapters.data?.chapters.edges.map((edge) => [edge.node.id, edge.node]) ?? []);

    return rows
      .map((row): ResumeItem | null => {
        if (!row.mediaFileId) return null;
        const file = byFile.get(row.mediaFileId);
        if (!file) return null;
        const base = {
          key: row.id,
          mediaFileId: row.mediaFileId,
          progress: row.progressPercent / 100,
          position: row.currentPosition,
          duration: row.duration ?? file.duration ?? undefined,
          updatedAt: row.updatedAt,
        };
        if (file.movieId) {
          const movie = movieMap.get(file.movieId);
          if (!movie) return null;
          return { ...base, kind: "movie", title: movie.title, subtitle: movie.year ? String(movie.year) : undefined, image: movieBackdrop(movie.id), href: `/movies/${movie.id}` };
        }
        if (file.episodeId) {
          const episode = episodeMap.get(file.episodeId);
          if (!episode) return null;
          const code = `S${String(episode.season).padStart(2, "0")} · E${String(episode.episode).padStart(2, "0")}`;
          return {
            ...base,
            kind: "episode",
            title: episode.show?.name ?? episode.title ?? "Episode",
            subtitle: [code, episode.title].filter(Boolean).join("  "),
            image: episode.show ? showBackdrop(episode.show) : undefined,
            href: `/shows/${episode.showId}`,
          };
        }
        if (file.trackId) {
          const track = trackMap.get(file.trackId);
          if (!track) return null;
          return { ...base, kind: "track", title: track.title, subtitle: track.album?.name ?? track.artistName ?? undefined, image: track.album ? albumCover(track.album) : undefined, href: `/albums/${track.albumId}` };
        }
        if (file.chapterId) {
          const chapter = chapterMap.get(file.chapterId);
          if (!chapter) return null;
          return {
            ...base,
            kind: "chapter",
            title: chapter.audiobook?.title ?? chapter.title ?? "Audiobook",
            subtitle: chapter.title ?? `Chapter ${chapter.chapterNumber}`,
            image: chapter.audiobook ? audiobookCover(chapter.audiobook) : undefined,
            href: `/audiobooks/${chapter.audiobookId}`,
          };
        }
        return null;
      })
      .filter(isPresent);
  }, [rows, fileList, movies.data, episodes.data, tracks.data, chapters.data]);

  return {
    items,
    loading: progress.loading || files.loading || movies.loading || episodes.loading || tracks.loading || chapters.loading,
    error: progress.error,
    refetch: progress.refetch,
  };
}
