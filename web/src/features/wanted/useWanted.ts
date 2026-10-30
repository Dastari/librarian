import { useMemo, useRef } from "react";

import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { useContentStatusState } from "@/hooks/useContentStatuses";
import { useLiveEpoch } from "@/lib/graphql/live/hooks";
import { useSession } from "@/lib/auth/useSession";
import {
  UpgradableFilesDocument,
  WantedListChaptersDocument,
  WantedListEpisodesDocument,
  WantedListMoviesDocument,
  WantedListTracksDocument,
  type ContentStatus,
} from "@/graphql/generated/graphql";
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

/**
 * Everything the acquisition queue needs, across every media type.
 *
 * `missing`, `wanted` and `downloading` read the entity lists (items without a file);
 * `downloading` narrows them with the computed `ContentStatus`. `upgradable` starts from the
 * media files the quality engine marked as below their profile and resolves the owning titles.
 */
export function useWanted(tab: WantedTab) {
  const { user, status } = useSession();
  const epoch = useLiveEpoch();
  const principal = `${epoch}:${user ? `${user.id}:${user.role}` : status}`;
  const snapshots = useRef<{ principal: string; tabs: Map<WantedTab, { groups: WantedGroup[]; statuses: Map<string, ContentStatus>; itemCount: number }> }>({ principal, tabs: new Map() });
  if (snapshots.current.principal !== principal) snapshots.current = { principal, tabs: new Map() };
  const listsActive = tab !== "upgradable";
  const wantedOnly = tab !== "missing";

  const episodes = useCompleteConnection(
    WantedListEpisodesDocument,
    { where: { mediaFileId: { isNull: true }, wanted: { eq: wantedOnly } } },
    (data) => data.episodes,
    { skip: !listsActive },
  );
  const movies = useCompleteConnection(
    WantedListMoviesDocument,
    { where: { hasFile: { eq: false }, wanted: { eq: wantedOnly } } },
    (data) => data.movies,
    { skip: !listsActive },
  );
  const tracks = useCompleteConnection(
    WantedListTracksDocument,
    { where: { mediaFileId: { isNull: true }, wanted: { eq: wantedOnly } } },
    (data) => data.tracks,
    { skip: !listsActive },
  );
  const chapters = useCompleteConnection(
    WantedListChaptersDocument,
    { where: { mediaFileId: { isNull: true }, wanted: { eq: wantedOnly } } },
    (data) => data.chapters,
    { skip: !listsActive },
  );
  const files = useCompleteConnection(UpgradableFilesDocument, {}, (data) => data.mediaFiles, { skip: listsActive });

  const upgradeIds = useMemo(() => {
    const pick = (key: "episodeId" | "movieId" | "trackId" | "chapterId") => files.rows.map((file) => file[key]).filter((id): id is string => Boolean(id));
    return { episodes: pick("episodeId"), movies: pick("movieId"), tracks: pick("trackId"), chapters: pick("chapterId") };
  }, [files.rows]);

  const upgradeEpisodes = useCompleteConnection(WantedListEpisodesDocument, { where: { id: { inList: upgradeIds.episodes } } }, (data) => data.episodes, { skip: listsActive || !files.complete || upgradeIds.episodes.length === 0 });
  const upgradeMovies = useCompleteConnection(WantedListMoviesDocument, { where: { id: { inList: upgradeIds.movies } } }, (data) => data.movies, { skip: listsActive || !files.complete || upgradeIds.movies.length === 0 });
  const upgradeTracks = useCompleteConnection(WantedListTracksDocument, { where: { id: { inList: upgradeIds.tracks } } }, (data) => data.tracks, { skip: listsActive || !files.complete || upgradeIds.tracks.length === 0 });
  const upgradeChapters = useCompleteConnection(WantedListChaptersDocument, { where: { id: { inList: upgradeIds.chapters } } }, (data) => data.chapters, { skip: listsActive || !files.complete || upgradeIds.chapters.length === 0 });

  const episodeRows = listsActive ? episodes.rows : upgradeEpisodes.rows;
  const movieRows = listsActive ? movies.rows : upgradeMovies.rows;
  const trackRows = listsActive ? tracks.rows : upgradeTracks.rows;
  const chapterRows = listsActive ? chapters.rows : upgradeChapters.rows;

  const episodeStatuses = useContentStatusState("EPISODE", useMemo(() => episodeRows.map((row) => row.id), [episodeRows]));
  const movieStatuses = useContentStatusState("MOVIE", useMemo(() => movieRows.map((row) => row.id), [movieRows]));
  const trackStatuses = useContentStatusState("TRACK", useMemo(() => trackRows.map((row) => row.id), [trackRows]));
  const chapterStatuses = useContentStatusState("CHAPTER", useMemo(() => chapterRows.map((row) => row.id), [chapterRows]));

  const statuses = useMemo(() => {
    const map = new Map<string, ContentStatus>();
    for (const [kind, source] of [["episode", episodeStatuses], ["movie", movieStatuses], ["track", trackStatuses], ["chapter", chapterStatuses]] as const) {
      for (const [id, status] of source.statuses) map.set(`${kind}:${id}`, status);
    }
    return map;
  }, [episodeStatuses.statuses, movieStatuses.statuses, trackStatuses.statuses, chapterStatuses.statuses]);

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

  const sources = listsActive ? [episodes, movies, tracks, chapters] : [files, upgradeEpisodes, upgradeMovies, upgradeTracks, upgradeChapters];
  const statusSources = [episodeStatuses, movieStatuses, trackStatuses, chapterStatuses];
  const fetching = sources.some((source) => source.loading) || statusSources.some((source) => source.loading);
  const error = sources.find((source) => source.error)?.error ?? statusSources.find((source) => source.error)?.error;
  const refetch = () => {
    for (const source of sources) void source.refetch();
    for (const source of statusSources) source.refetch();
  };
  const itemCount = groups.reduce((total, group: WantedGroup) => total + group.items.length, 0);
  // Group actions must always target an entire title. Stage all entity, upgrade
  // and computed-status batches, then replace one complete snapshot at once.
  // Keep the prior complete tab visible during refresh and on a failed batch.
  if (!fetching && !error) snapshots.current.tabs.set(tab, { groups, statuses, itemCount });
  const snapshot = snapshots.current.tabs.get(tab);

  return {
    groups: snapshot?.groups ?? [],
    statuses: snapshot?.statuses ?? new Map<string, ContentStatus>(),
    itemCount: snapshot?.itemCount ?? 0,
    loading: fetching && !snapshot && !error,
    fetching,
    refreshing: fetching && Boolean(snapshot),
    error,
    complete: Boolean(snapshot),
    queryKey: JSON.stringify([principal, tab]),
    hasMore: sources.some((source) => source.hasMore),
    loadingMore: sources.some((source) => source.loadingMore),
    loadMore: async () => {
      await Promise.all(sources.filter((source) => source.hasMore).map((source) => source.loadMore()));
    },
    refetch,
  };
}
