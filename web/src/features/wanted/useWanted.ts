import { useQuery } from "@apollo/client/react";
import { useMemo } from "react";

import { useInfiniteConnection } from "@/features/libraries/browser/useInfiniteConnection";
import {
  UpgradableFilesDocument,
  WantedListChaptersDocument,
  WantedListEpisodesDocument,
  WantedListMoviesDocument,
  WantedListTracksDocument,
  type ContentStatus,
} from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { albumCover, audiobookCover, moviePoster, showPoster } from "@/lib/artwork";

import { buildGroups, episodeCode, groupKey, type WantedEntry, type WantedGroup } from "./grouping";

export const WANTED_TABS = ["missing", "wanted", "downloading", "upgradable"] as const;
export type WantedTab = (typeof WANTED_TABS)[number];

export const WANTED_TAB_LABEL: Record<WantedTab, string> = {
  missing: "Missing",
  wanted: "Wanted",
  downloading: "Downloading",
  upgradable: "Upgradable",
};

const TARGET_PAGE = { limit: 200, offset: 0 };

/**
 * Everything the acquisition queue needs, across every media type.
 *
 * `missing`, `wanted` and `downloading` read the entity lists (items without a file);
 * `downloading` narrows them with the computed `ContentStatus`. `upgradable` starts from the
 * media files the quality engine marked as below their profile and resolves the owning titles.
 */
export function useWanted(tab: WantedTab) {
  const listsActive = tab !== "upgradable";
  const wantedOnly = tab !== "missing";

  const episodes = useInfiniteConnection(
    WantedListEpisodesDocument,
    { where: { mediaFileId: { isNull: true }, wanted: { eq: wantedOnly } } },
    (data) => data.episodes,
    { skip: !listsActive },
  );
  const movies = useInfiniteConnection(
    WantedListMoviesDocument,
    { where: { hasFile: { eq: false }, wanted: { eq: wantedOnly } } },
    (data) => data.movies,
    { skip: !listsActive },
  );
  const tracks = useInfiniteConnection(
    WantedListTracksDocument,
    { where: { mediaFileId: { isNull: true }, wanted: { eq: wantedOnly } } },
    (data) => data.tracks,
    { skip: !listsActive },
  );
  const chapters = useInfiniteConnection(
    WantedListChaptersDocument,
    { where: { mediaFileId: { isNull: true }, wanted: { eq: wantedOnly } } },
    (data) => data.chapters,
    { skip: !listsActive },
  );
  const files = useInfiniteConnection(UpgradableFilesDocument, {}, (data) => data.mediaFiles, { skip: listsActive });

  const upgradeIds = useMemo(() => {
    const pick = (key: "episodeId" | "movieId" | "trackId" | "chapterId") => files.rows.map((file) => file[key]).filter((id): id is string => Boolean(id));
    return { episodes: pick("episodeId"), movies: pick("movieId"), tracks: pick("trackId"), chapters: pick("chapterId") };
  }, [files.rows]);

  const upgradeEpisodes = useQuery(WantedListEpisodesDocument, { variables: { where: { id: { inList: upgradeIds.episodes } }, page: TARGET_PAGE }, skip: upgradeIds.episodes.length === 0 });
  const upgradeMovies = useQuery(WantedListMoviesDocument, { variables: { where: { id: { inList: upgradeIds.movies } }, page: TARGET_PAGE }, skip: upgradeIds.movies.length === 0 });
  const upgradeTracks = useQuery(WantedListTracksDocument, { variables: { where: { id: { inList: upgradeIds.tracks } }, page: TARGET_PAGE }, skip: upgradeIds.tracks.length === 0 });
  const upgradeChapters = useQuery(WantedListChaptersDocument, { variables: { where: { id: { inList: upgradeIds.chapters } }, page: TARGET_PAGE }, skip: upgradeIds.chapters.length === 0 });

  const episodeRows = useMemo(() => (listsActive ? episodes.rows : upgradeEpisodes.data?.episodes.edges.map((edge) => edge.node) ?? []), [listsActive, episodes.rows, upgradeEpisodes.data]);
  const movieRows = useMemo(() => (listsActive ? movies.rows : upgradeMovies.data?.movies.edges.map((edge) => edge.node) ?? []), [listsActive, movies.rows, upgradeMovies.data]);
  const trackRows = useMemo(() => (listsActive ? tracks.rows : upgradeTracks.data?.tracks.edges.map((edge) => edge.node) ?? []), [listsActive, tracks.rows, upgradeTracks.data]);
  const chapterRows = useMemo(() => (listsActive ? chapters.rows : upgradeChapters.data?.chapters.edges.map((edge) => edge.node) ?? []), [listsActive, chapters.rows, upgradeChapters.data]);

  const episodeStatuses = useContentStatuses("EPISODE", useMemo(() => episodeRows.map((row) => row.id), [episodeRows]));
  const movieStatuses = useContentStatuses("MOVIE", useMemo(() => movieRows.map((row) => row.id), [movieRows]));
  const trackStatuses = useContentStatuses("TRACK", useMemo(() => trackRows.map((row) => row.id), [trackRows]));
  const chapterStatuses = useContentStatuses("CHAPTER", useMemo(() => chapterRows.map((row) => row.id), [chapterRows]));

  const statuses = useMemo(() => {
    const map = new Map<string, ContentStatus>();
    for (const [kind, source] of [["episode", episodeStatuses], ["movie", movieStatuses], ["track", trackStatuses], ["chapter", chapterStatuses]] as const) {
      for (const [id, status] of source) map.set(`${kind}:${id}`, status);
    }
    return map;
  }, [episodeStatuses, movieStatuses, trackStatuses, chapterStatuses]);

  const groups = useMemo(() => {
    const entries: WantedEntry[] = [];
    const keep = (kind: string, id: string, ignored?: boolean | null) => {
      if (ignored === true) return false;
      if (tab === "downloading") return statuses.get(`${kind}:${id}`) === "DOWNLOADING";
      return true;
    };

    for (const episode of episodeRows) {
      if (!episode.show || !keep("episode", episode.id, episode.ignored)) continue;
      entries.push({
        group: {
          key: groupKey("episode", episode.show.id),
          kind: "episode",
          parentId: episode.show.id,
          title: episode.show.name,
          subtitle: null,
          poster: showPoster(episode.show),
          square: false,
          libraryId: episode.show.libraryId,
          imdbId: episode.show.imdbId,
        },
        item: { id: episode.id, code: episodeCode(episode.season, episode.episode), title: episode.title ?? `Episode ${episode.episode}`, season: episode.season, wanted: episode.wanted, ignored: episode.ignored },
      });
    }
    for (const movie of movieRows) {
      if (!keep("movie", movie.id, movie.ignored)) continue;
      entries.push({
        group: {
          key: groupKey("movie", movie.id),
          kind: "movie",
          parentId: movie.id,
          title: movie.title,
          subtitle: movie.year ? String(movie.year) : null,
          poster: moviePoster(movie.id),
          square: false,
          libraryId: movie.libraryId,
          imdbId: movie.imdbId,
          year: movie.year,
        },
        item: { id: movie.id, code: movie.year ? String(movie.year) : "Movie", title: movie.title, wanted: movie.wanted, ignored: movie.ignored },
      });
    }
    for (const track of trackRows) {
      if (!track.album || !keep("track", track.id, track.ignored)) continue;
      entries.push({
        group: {
          key: groupKey("track", track.album.id),
          kind: "track",
          parentId: track.album.id,
          title: track.album.name,
          subtitle: track.artistName ?? null,
          poster: albumCover(track.album),
          square: true,
          libraryId: track.album.libraryId,
          year: track.album.year,
        },
        item: { id: track.id, code: String(track.trackNumber), title: track.title, wanted: track.wanted, ignored: track.ignored },
      });
    }
    for (const chapter of chapterRows) {
      if (!chapter.audiobook || !keep("chapter", chapter.id, chapter.ignored)) continue;
      entries.push({
        group: {
          key: groupKey("chapter", chapter.audiobook.id),
          kind: "chapter",
          parentId: chapter.audiobook.id,
          title: chapter.audiobook.title,
          subtitle: chapter.audiobook.authorName ?? null,
          poster: audiobookCover(chapter.audiobook),
          square: false,
          libraryId: chapter.audiobook.libraryId,
        },
        item: { id: chapter.id, code: String(chapter.chapterNumber), title: chapter.title ?? `Chapter ${chapter.chapterNumber}`, wanted: chapter.wanted, ignored: chapter.ignored },
      });
    }
    return buildGroups(entries);
  }, [episodeRows, movieRows, trackRows, chapterRows, statuses, tab]);

  const sources = listsActive ? [episodes, movies, tracks, chapters] : [files];
  const itemCount = groups.reduce((total, group: WantedGroup) => total + group.items.length, 0);

  return {
    groups,
    statuses,
    itemCount,
    loading: sources.some((source) => source.loading),
    error: sources.find((source) => source.error)?.error,
    hasMore: sources.some((source) => source.hasMore),
    loadingMore: sources.some((source) => source.loadingMore),
    loadMore: async () => {
      await Promise.all(sources.filter((source) => source.hasMore).map((source) => source.loadMore()));
    },
    refetch: () => {
      for (const source of sources) void source.refetch();
    },
  };
}
