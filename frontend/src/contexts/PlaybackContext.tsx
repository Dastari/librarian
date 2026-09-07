/**
 * Playback Context
 *
 * Provides shared playback state across the application.
 * Manages persistent video/audio playback with database sync.
 * Supports all content types: episodes, movies, tracks, and audiobooks.
 */

import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useRef,
  type ReactNode,
} from "react";
import { useRouteContext } from "@tanstack/react-router";
import { apolloClient } from "../lib/graphql/client";
import {
  type PlaybackSession,
  type StartPlaybackInput,
  type UpdatePlaybackInput,
  type PlaybackContentType,
  type Episode,
  type TvShow,
  type Track,
  type Album,
  type Audiobook,
  type AudiobookChapter,
  type AlbumWithTracks,
  type AudiobookWithChapters,
} from "../lib/graphql";
import {
  AlbumDetailRouteDocument,
  AudiobookDetailRouteDocument,
  ContentStatusesDocument,
  ContentStatusType,
  CreatePlaybackProgressContextDocument,
  CreatePlaybackSessionContextDocument,
  MeDocument,
  PlaybackProgressByMediaFileContextDocument,
  PlaybackSessionsDocument,
  UpdatePlaybackProgressContextDocument,
  UpdatePlaybackSessionContextDocument,
  type AlbumDetailRouteQuery,
  type AudiobookDetailRouteQuery,
  type PlaybackProgressByMediaFileContextQuery,
} from "../lib/graphql/generated/graphql";
import type { Movie } from "../lib/graphql/generated/graphql";

/** Map GraphQL PlaybackSessions node (PascalCase) to app PlaybackSession (camelCase) */
function mapNodeToSession(node: {
  id: string;
  userId: string;
  mediaFileId?: string | null;
  currentPosition: number;
  duration?: number | null;
  volume: number;
  isMuted: boolean;
  isPlaying: boolean;
  startedAt: string;
  lastUpdatedAt: string;
  completedAt?: string | null;
  createdAt: string;
  updatedAt: string;
  contentType?: string | null;
  episodeId?: string | null;
  movieId?: string | null;
  trackId?: string | null;
  audiobookId?: string | null;
  tvShowId?: string | null;
  albumId?: string | null;
}): PlaybackSession {
  const ct = node.contentType?.toUpperCase() as PlaybackContentType | undefined;
  return {
    id: node.id,
    userId: node.userId,
    contentType: ct ?? null,
    mediaFileId: node.mediaFileId ?? null,
    contentId: node.episodeId ?? node.movieId ?? node.trackId ?? null,
    episodeId: node.episodeId ?? null,
    movieId: node.movieId ?? null,
    trackId: node.trackId ?? null,
    audiobookId: node.audiobookId ?? null,
    tvShowId: node.tvShowId ?? null,
    albumId: node.albumId ?? null,
    currentPosition: node.currentPosition,
    duration: node.duration ?? null,
    volume: node.volume,
    isMuted: node.isMuted,
    isPlaying: node.isPlaying,
    startedAt: node.startedAt,
    lastUpdatedAt: node.lastUpdatedAt,
  };
}

/** Metadata for the currently playing content */
export interface CurrentContentMetadata {
  contentType: PlaybackContentType;
  title: string;
  subtitle?: string;
  posterUrl?: string | null;
  backdropUrl?: string | null;
}

/** Queue item for audio playback */
export interface QueueItem {
  id: string;
  mediaFileId: string;
  title: string;
  artist?: string;
  duration?: number;
  coverUrl?: string | null;
  // For tracks
  track?: Track;
  // For audiobook chapters
  chapter?: AudiobookChapter;
}

/** Repeat mode for audio playback */
export type RepeatMode = "off" | "all" | "one";

interface PlaybackContextValue {
  session: PlaybackSession | null;
  isLoading: boolean;
  currentContent: CurrentContentMetadata | null;
  currentEpisode: Episode | null;
  currentShow: TvShow | null;
  currentMovie: Movie | null;
  shouldExpand: boolean;

  // Audio-specific state
  currentTrack: Track | null;
  currentAlbum: Album | null;
  currentAudiobook: Audiobook | null;
  currentChapter: AudiobookChapter | null;
  queue: QueueItem[];
  queueIndex: number;
  shuffleEnabled: boolean;
  repeatMode: RepeatMode;

  // Base playback methods
  startPlayback: (
    input: StartPlaybackInput,
    metadata?: CurrentContentMetadata,
  ) => Promise<boolean>;
  startEpisodePlayback: (
    episodeId: string,
    mediaFileId: string,
    tvShowId: string,
    episode?: Episode,
    show?: TvShow,
    startPosition?: number,
    duration?: number,
  ) => Promise<boolean>;
  startMoviePlayback: (
    movieId: string,
    mediaFileId: string,
    movie?: Movie,
    startPosition?: number,
    duration?: number,
  ) => Promise<boolean>;

  // Audio-specific playback methods
  startTrackPlayback: (
    track: Track,
    album: Album,
    allTracks: Track[],
    startPosition?: number,
  ) => Promise<boolean>;
  startAudiobookPlayback: (
    audiobook: Audiobook,
    chapter: AudiobookChapter,
    allChapters: AudiobookChapter[],
    startPosition?: number,
  ) => Promise<boolean>;
  playNext: () => Promise<boolean>;
  playPrevious: () => Promise<boolean>;
  playQueueItem: (index: number) => Promise<boolean>;
  toggleShuffle: () => void;
  setRepeatMode: (mode: RepeatMode) => void;

  // Common methods
  updatePlayback: (input: UpdatePlaybackInput) => Promise<boolean>;
  stopPlayback: (
    finalPosition?: number,
    finalDuration?: number,
  ) => Promise<boolean>;
  refreshSession: () => Promise<void>;
  setCurrentEpisode: (episode: Episode | null) => void;
  setCurrentShow: (show: TvShow | null) => void;
  setCurrentMovie: (movie: Movie | null) => void;
  clearExpandFlag: () => void;
}

const PlaybackContext = createContext<PlaybackContextValue | null>(null);
type PlaybackProgressNode =
  PlaybackProgressByMediaFileContextQuery["playbackProgresses"]["edges"][number]["node"];
type AlbumDetailNode = NonNullable<AlbumDetailRouteQuery["album"]>;
type AlbumTrackNode = AlbumDetailRouteQuery["tracks"]["edges"][number]["node"];
type AudiobookDetailNode = NonNullable<AudiobookDetailRouteQuery["audiobook"]>;
type AudiobookChapterNode =
  AudiobookDetailNode["chapters"]["edges"][number]["node"];

export function PlaybackProvider({ children }: { children: ReactNode }) {
  // Get auth context from router - only fetch playback session if authenticated
  const { auth } = useRouteContext({ from: "__root__" });

  const [session, setSession] = useState<PlaybackSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [currentContent, setCurrentContent] =
    useState<CurrentContentMetadata | null>(null);
  const [currentEpisode, setCurrentEpisode] = useState<Episode | null>(null);
  const [currentShow, setCurrentShow] = useState<TvShow | null>(null);
  const [currentMovie, setCurrentMovie] = useState<Movie | null>(null);
  const [shouldExpand, setShouldExpand] = useState(false);

  // Audio-specific state
  const [currentTrack, setCurrentTrack] = useState<Track | null>(null);
  const [currentAlbum, setCurrentAlbum] = useState<Album | null>(null);
  const [currentAudiobook, setCurrentAudiobook] = useState<Audiobook | null>(
    null,
  );
  const [currentChapter, setCurrentChapter] = useState<AudiobookChapter | null>(
    null,
  );
  const [queue, setQueue] = useState<QueueItem[]>([]);
  const [queueIndex, setQueueIndex] = useState(0);
  const [shuffleEnabled, setShuffleEnabled] = useState(false);
  const [repeatMode, setRepeatModeState] = useState<RepeatMode>("off");

  const lastSyncedPosition = useRef<number>(0);
  const userIdRef = useRef<string | null>(null);
  const playbackProgressIdsRef = useRef<Map<string, string>>(new Map());

  const clearExpandFlag = useCallback(() => {
    setShouldExpand(false);
  }, []);

  const clearAllState = useCallback(() => {
    setSession(null);
    setCurrentContent(null);
    setCurrentEpisode(null);
    setCurrentShow(null);
    setCurrentMovie(null);
    setCurrentTrack(null);
    setCurrentAlbum(null);
    setCurrentAudiobook(null);
    setCurrentChapter(null);
    setQueue([]);
    setQueueIndex(0);
    userIdRef.current = null;
    playbackProgressIdsRef.current.clear();
  }, []);

  const resolveCurrentUserId = useCallback(async (): Promise<string | null> => {
    if (userIdRef.current) {
      return userIdRef.current;
    }

    const meResult = await apolloClient.query({
      query: MeDocument,
      fetchPolicy: "network-only",
    });
    const userId = meResult.data?.me?.id ?? null;
    userIdRef.current = userId;
    return userId;
  }, []);

  const fetchPlaybackProgress = useCallback(
    async (
      userId: string,
      mediaFileId: string,
    ): Promise<PlaybackProgressNode | null> => {
      const result = await apolloClient.query({
        query: PlaybackProgressByMediaFileContextDocument,
        variables: {
          where: {
            userId: { eq: userId },
            mediaFileId: { eq: mediaFileId },
          },
          page: { limit: 1, offset: 0 },
          orderBy: [{ updatedAt: "DESC" }],
        },
        fetchPolicy: "network-only",
      });

      return result.data?.playbackProgresses?.edges?.[0]?.node ?? null;
    },
    [],
  );

  const upsertPlaybackProgress = useCallback(
    async (
      mediaFileId: string,
      currentPosition: number,
      duration?: number,
      markWatched = false,
    ): Promise<void> => {
      if (!mediaFileId) return;
      const userId = await resolveCurrentUserId();
      if (!userId) return;

      const safePosition = Math.max(
        0,
        Number.isFinite(currentPosition) ? currentPosition : 0,
      );
      let safeDuration =
        duration !== undefined && Number.isFinite(duration) && duration > 0
          ? duration
          : undefined;
      let existingId = playbackProgressIdsRef.current.get(mediaFileId);
      let existing: PlaybackProgressNode | null = null;

      if (!existingId) {
        existing = await fetchPlaybackProgress(userId, mediaFileId);
        if (existing) {
          existingId = existing.id;
          playbackProgressIdsRef.current.set(mediaFileId, existing.id);
        }
      }

      if (!safeDuration && existing?.duration && existing.duration > 0) {
        safeDuration = existing.duration;
      }

      const progressPercent = safeDuration
        ? Math.max(0, Math.min(1, safePosition / safeDuration))
        : 0;
      const isWatched = markWatched || progressPercent >= 0.9;
      const watchedAt = isWatched ? new Date().toISOString() : null;

      if (existingId) {
        const updateResult = await apolloClient.mutate<{
          updatePlaybackProgress?: {
            success: boolean;
            playbackProgress?: PlaybackProgressNode | null;
          };
        }>({
          mutation: UpdatePlaybackProgressContextDocument,
          variables: {
            id: existingId,
            input: {
              currentPosition: safePosition,
              duration: safeDuration,
              progressPercent: progressPercent,
              isWatched: isWatched,
              watchedAt: watchedAt,
            },
          },
        });

        const updated =
          updateResult.data?.updatePlaybackProgress?.playbackProgress;
        if (updated?.id) {
          playbackProgressIdsRef.current.set(mediaFileId, updated.id);
        }
        return;
      }

      const createResult = await apolloClient.mutate<{
        createPlaybackProgress?: {
          success: boolean;
          playbackProgress?: PlaybackProgressNode | null;
        };
      }>({
        mutation: CreatePlaybackProgressContextDocument,
        variables: {
          input: {
            userId: userId,
            mediaFileId: mediaFileId,
            currentPosition: safePosition,
            duration: safeDuration,
            progressPercent: progressPercent,
            isWatched: isWatched,
            watchedAt: watchedAt,
          },
        },
      });

      const created =
        createResult.data?.createPlaybackProgress?.playbackProgress;
      if (created?.id) {
        playbackProgressIdsRef.current.set(mediaFileId, created.id);
      }
    },
    [fetchPlaybackProgress, resolveCurrentUserId],
  );

  const refreshSession = useCallback(async () => {
    try {
      const meResult = await apolloClient.query({
        query: MeDocument,
        fetchPolicy: "network-only",
      });
      const userId = meResult.data?.me?.id;
      if (!userId) {
        clearAllState();
        setIsLoading(false);
        return;
      }
      userIdRef.current = userId;

      const result = await apolloClient.query({
        query: PlaybackSessionsDocument,
        variables: {
          where: { userId: { eq: userId } },
          orderBy: [{ lastUpdatedAt: "DESC" }],
          page: { limit: 1, offset: 0 },
        },
        fetchPolicy: "network-only",
      });
      const node = result.data?.playbackSessions?.edges?.[0]?.node;
      if (node) {
        const session = mapNodeToSession(node);
        setSession(session);

        // For audio sessions, fetch the track/album or audiobook data
        if (
          session.contentType === "TRACK" &&
          session.albumId &&
          session.trackId
        ) {
          try {
            const albumResult = await apolloClient.query({
              query: AlbumDetailRouteDocument,
              variables: { id: session.albumId },
              fetchPolicy: "network-only",
            });

            if (albumResult.data?.album) {
              const albumNode: AlbumDetailNode = albumResult.data.album;
              const trackEdges = albumResult.data.tracks?.edges ?? [];
              const statusResult = await apolloClient.query({
                query: ContentStatusesDocument,
                variables: {
                  inputs: trackEdges.map((edge) => ({
                    contentType: ContentStatusType.TRACK,
                    id: edge.node.id,
                  })),
                },
                fetchPolicy: "network-only",
              });
              const statusById = new Map(
                (statusResult.data?.contentStatuses ?? []).map((item) => [
                  item.id,
                  item.status,
                ]),
              );
              const album: AlbumWithTracks["album"] = {
                id: albumNode.id,
                artistId: albumNode.artistId,
                libraryId: albumNode.libraryId,
                name: albumNode.name,
                sortName: albumNode.sortName ?? null,
                year: albumNode.year ?? null,
                musicbrainzId: albumNode.musicbrainzId ?? null,
                albumType: albumNode.albumType ?? null,
                genres: albumNode.genres,
                label: albumNode.label ?? null,
                country: albumNode.country ?? null,
                releaseDate: albumNode.releaseDate ?? null,
                coverUrl: albumNode.coverUrl ?? null,
                trackCount: albumNode.trackCount ?? null,
                discCount: albumNode.discCount ?? null,
                totalDurationSecs: albumNode.totalDurationSecs ?? null,
                hasFiles: albumNode.hasFiles,
                sizeBytes: albumNode.sizeBytes ?? null,
                path: albumNode.path ?? null,
                downloadedTrackCount: null,
              };
              const tracks: AlbumWithTracks["tracks"] = trackEdges.map(
                (edge: { node: AlbumTrackNode }) => ({
                  track: {
                    id: edge.node.id,
                    albumId: edge.node.albumId,
                    libraryId: edge.node.libraryId,
                    title: edge.node.title,
                    trackNumber: edge.node.trackNumber,
                    discNumber: edge.node.discNumber ?? 1,
                    musicbrainzId: edge.node.musicbrainzId ?? null,
                    isrc: edge.node.isrc ?? null,
                    durationSecs: edge.node.durationSecs ?? null,
                    explicit: edge.node.explicit,
                    artistName: edge.node.artistName ?? null,
                    artistId: edge.node.artistId ?? null,
                    mediaFileId: edge.node.mediaFileId ?? null,
                    hasFile: Boolean(edge.node.mediaFileId),
                    status:
                      statusById.get(edge.node.id) ??
                      (edge.node.mediaFileId
                        ? "AVAILABLE"
                        : edge.node.wanted
                          ? "WANTED"
                          : "MISSING"),
                    downloadProgress: null,
                  },
                  hasFile: Boolean(edge.node.mediaFileId),
                  filePath: null,
                  fileSize: null,
                  audioCodec: null,
                  bitrate: null,
                  audioChannels: null,
                }),
              );
              setCurrentAlbum(album);

              // Find the current track
              const trackData = tracks.find(
                (t) => t.track.id === session.trackId,
              );
              if (trackData) {
                setCurrentTrack(trackData.track);

                // Build queue from all available tracks
                const allTracks = tracks
                  .filter((t) => t.track.mediaFileId)
                  .map((t) => t.track);
                const newQueue = allTracks.map((track) => ({
                  id: track.id,
                  mediaFileId: track.mediaFileId!,
                  title: track.title,
                  artist: track.artistName || undefined,
                  duration: track.durationSecs || undefined,
                  coverUrl: album.coverUrl,
                  track,
                }));
                setQueue(newQueue);

                const idx = newQueue.findIndex((q) => q.id === session.trackId);
                setQueueIndex(idx >= 0 ? idx : 0);

                // Set content metadata for display
                setCurrentContent({
                  contentType: "TRACK",
                  title: trackData.track.title,
                  subtitle: trackData.track.artistName || album.name,
                  posterUrl: album.coverUrl,
                });
              }
            }
          } catch (err) {
            console.error("Failed to fetch track/album data:", err);
          }
        } else if (session.contentType === "AUDIOBOOK" && session.audiobookId) {
          try {
            const audiobookResult = await apolloClient.query({
              query: AudiobookDetailRouteDocument,
              variables: { id: session.audiobookId },
              fetchPolicy: "network-only",
            });

            if (audiobookResult.data?.audiobook) {
              const audiobookNode: AudiobookDetailNode =
                audiobookResult.data.audiobook;
              const chapterEdges = audiobookNode.chapters?.edges ?? [];
              const statusResult = await apolloClient.query({
                query: ContentStatusesDocument,
                variables: {
                  inputs: chapterEdges.map((edge) => ({
                    contentType: ContentStatusType.CHAPTER,
                    id: edge.node.id,
                  })),
                },
                fetchPolicy: "network-only",
              });
              const statusById = new Map(
                (statusResult.data?.contentStatuses ?? []).map((item) => [
                  item.id,
                  item.status,
                ]),
              );
              const audiobook: AudiobookWithChapters["audiobook"] = {
                id: audiobookNode.id,
                authorId: null,
                libraryId: audiobookNode.libraryId,
                title: audiobookNode.title,
                sortTitle: audiobookNode.sortTitle ?? null,
                subtitle: null,
                openlibraryId: null,
                isbn: audiobookNode.isbn ?? null,
                description: audiobookNode.description ?? null,
                publisher: audiobookNode.publisher ?? null,
                language: audiobookNode.language ?? null,
                narrators: audiobookNode.narrators,
                seriesName: null,
                durationSecs: audiobookNode.totalDurationSecs ?? null,
                coverUrl: audiobookNode.coverUrl ?? null,
                hasFiles: audiobookNode.hasFiles,
                sizeBytes: audiobookNode.sizeBytes ?? null,
                path: audiobookNode.path ?? null,
                chapterCount: null,
                downloadedChapterCount: null,
              };
              const chapters = chapterEdges.map(
                (edge: { node: AudiobookChapterNode }) => ({
                  id: edge.node.id,
                  audiobookId: edge.node.audiobookId,
                  chapterNumber: edge.node.chapterNumber,
                  title: edge.node.title ?? null,
                  startSecs: Math.floor(edge.node.startTimeSecs),
                  endSecs:
                    edge.node.endTimeSecs != null
                      ? Math.floor(edge.node.endTimeSecs)
                      : 0,
                  durationSecs: edge.node.durationSecs ?? null,
                  mediaFileId: edge.node.mediaFileId ?? null,
                  status:
                    statusById.get(edge.node.id) ??
                    (edge.node.mediaFileId
                      ? "AVAILABLE"
                      : edge.node.wanted
                        ? "WANTED"
                        : "MISSING"),
                  downloadProgress: null,
                }),
              );
              setCurrentAudiobook(audiobook);

              // Find the current chapter (use mediaFileId to match since we may not have chapterId directly)
              const currentChapter = chapters.find(
                (c) => c.mediaFileId === session.mediaFileId,
              );
              if (currentChapter) {
                setCurrentChapter(currentChapter);

                // Build queue from all available chapters
                const availableChapters = chapters.filter((c) => c.mediaFileId);
                const newQueue = availableChapters.map((chapter) => ({
                  id: chapter.id,
                  mediaFileId: chapter.mediaFileId!,
                  title: chapter.title || `Chapter ${chapter.chapterNumber}`,
                  duration: chapter.durationSecs || undefined,
                  coverUrl: audiobook.coverUrl,
                  chapter,
                }));
                setQueue(newQueue);

                const idx = newQueue.findIndex(
                  (q) => q.id === currentChapter.id,
                );
                setQueueIndex(idx >= 0 ? idx : 0);

                // Set content metadata for display
                setCurrentContent({
                  contentType: "AUDIOBOOK",
                  title: audiobook.title,
                  subtitle:
                    currentChapter.title ||
                    `Chapter ${currentChapter.chapterNumber}`,
                  posterUrl: audiobook.coverUrl,
                });
              }
            }
          } catch (err) {
            console.error("Failed to fetch audiobook data:", err);
          }
        }
      } else {
        clearAllState();
      }
    } catch (err) {
      console.error("Failed to fetch playback session:", err);
    } finally {
      setIsLoading(false);
    }
  }, [clearAllState]);

  // Only fetch playback session when user is authenticated
  useEffect(() => {
    if (auth.isAuthenticated) {
      refreshSession();
    } else {
      // Not authenticated - clear state and stop loading
      setIsLoading(false);
      setSession(null);
    }
  }, [auth.isAuthenticated, refreshSession]);

  const startPlayback = useCallback(
    async (
      input: StartPlaybackInput,
      metadata?: CurrentContentMetadata,
    ): Promise<boolean> => {
      try {
        const meResult = await apolloClient.query({
          query: MeDocument,
          fetchPolicy: "network-only",
        });
        const userId = meResult.data?.me?.id;
        if (!userId) {
          console.error("Failed to start playback: no authenticated user");
          return false;
        }
        userIdRef.current = userId;

        const now = new Date().toISOString();
        let currentPosition = input.startPosition || 0;
        if (
          (input.contentType === "EPISODE" || input.contentType === "MOVIE") &&
          (!input.startPosition || input.startPosition <= 0)
        ) {
          const existingProgress = await fetchPlaybackProgress(
            userId,
            input.mediaFileId,
          );
          if (existingProgress?.id) {
            playbackProgressIdsRef.current.set(
              input.mediaFileId,
              existingProgress.id,
            );
          }
          if (
            existingProgress &&
            !existingProgress.isWatched &&
            existingProgress.currentPosition > 0
          ) {
            currentPosition = existingProgress.currentPosition;
          }
        }
        const baseInput = {
          contentType: input.contentType,
          mediaFileId: input.mediaFileId,
          currentPosition: currentPosition,
          duration: input.duration,
          volume: session?.volume ?? 1,
          isMuted: session?.isMuted ?? false,
          isPlaying: true,
          lastUpdatedAt: now,
        } as Record<string, unknown>;

        if (input.contentType === "EPISODE") {
          baseInput.episodeId = input.contentId;
          baseInput.tvShowId = input.parentId;
        } else if (input.contentType === "MOVIE") {
          baseInput.movieId = input.contentId;
        } else if (input.contentType === "TRACK") {
          baseInput.trackId = input.contentId;
          baseInput.albumId = input.parentId;
        } else if (input.contentType === "AUDIOBOOK") {
          baseInput.audiobookId = input.contentId;
        }

        let success = false;
        let errorMessage: string | undefined;
        let nextSessionNode:
          | {
              id: string;
              userId: string;
              mediaFileId?: string | null;
              currentPosition: number;
              duration?: number | null;
              volume: number;
              isMuted: boolean;
              isPlaying: boolean;
              startedAt: string;
              lastUpdatedAt: string;
              completedAt?: string | null;
              createdAt: string;
              updatedAt: string;
              contentType?: string | null;
              episodeId?: string | null;
              movieId?: string | null;
              trackId?: string | null;
              audiobookId?: string | null;
              tvShowId?: string | null;
              albumId?: string | null;
            }
          | null
          | undefined;

        if (session?.id) {
          const updateResult = await apolloClient.mutate<{
            updatePlaybackSession: {
              success: boolean;
              error?: string | null;
              playbackSession?: {
                id: string;
                userId: string;
                mediaFileId?: string | null;
                currentPosition: number;
                duration?: number | null;
                volume: number;
                isMuted: boolean;
                isPlaying: boolean;
                startedAt: string;
                lastUpdatedAt: string;
                completedAt?: string | null;
                createdAt: string;
                updatedAt: string;
                contentType?: string | null;
                episodeId?: string | null;
                movieId?: string | null;
                trackId?: string | null;
                audiobookId?: string | null;
                tvShowId?: string | null;
                albumId?: string | null;
              } | null;
            };
          }>({
            mutation: UpdatePlaybackSessionContextDocument,
            variables: {
              id: session.id,
              input: baseInput,
            },
          });
          success = Boolean(updateResult.data?.updatePlaybackSession?.success);
          errorMessage =
            updateResult.data?.updatePlaybackSession?.error || undefined;
          nextSessionNode =
            updateResult.data?.updatePlaybackSession?.playbackSession;
        } else {
          const createResult = await apolloClient.mutate<{
            createPlaybackSession: {
              success: boolean;
              error?: string | null;
              playbackSession?: {
                id: string;
                userId: string;
                mediaFileId?: string | null;
                currentPosition: number;
                duration?: number | null;
                volume: number;
                isMuted: boolean;
                isPlaying: boolean;
                startedAt: string;
                lastUpdatedAt: string;
                completedAt?: string | null;
                createdAt: string;
                updatedAt: string;
                contentType?: string | null;
                episodeId?: string | null;
                movieId?: string | null;
                trackId?: string | null;
                audiobookId?: string | null;
                tvShowId?: string | null;
                albumId?: string | null;
              } | null;
            };
          }>({
            mutation: CreatePlaybackSessionContextDocument,
            variables: {
              input: {
                ...baseInput,
                userId: userId,
                startedAt: now,
              },
            },
          });
          success = Boolean(createResult.data?.createPlaybackSession?.success);
          errorMessage =
            createResult.data?.createPlaybackSession?.error || undefined;
          nextSessionNode =
            createResult.data?.createPlaybackSession?.playbackSession;
        }

        if (!success) {
          console.error("Failed to start playback:", errorMessage);
          return false;
        }

        if (nextSessionNode) {
          setSession(mapNodeToSession(nextSessionNode));
        } else {
          await refreshSession();
        }
        if (metadata) setCurrentContent(metadata);
        lastSyncedPosition.current = currentPosition;
        setShouldExpand(true);
        return true;
      } catch (err) {
        console.error("Failed to start playback:", err);
        return false;
      }
    },
    [
      fetchPlaybackProgress,
      refreshSession,
      session?.id,
      session?.isMuted,
      session?.volume,
    ],
  );

  const startEpisodePlayback = useCallback(
    async (
      episodeId: string,
      mediaFileId: string,
      tvShowId: string,
      episode?: Episode,
      show?: TvShow,
      startPosition?: number,
      duration?: number,
    ): Promise<boolean> => {
      const input: StartPlaybackInput = {
        contentType: "EPISODE",
        contentId: episodeId,
        mediaFileId,
        parentId: tvShowId,
        startPosition,
        duration,
      };

      const metadata: CurrentContentMetadata | undefined =
        episode && show
          ? {
              contentType: "EPISODE",
              title: show.name,
              subtitle: `S${String(episode.season).padStart(2, "0")}E${String(episode.episode).padStart(2, "0")} - ${episode.title}`,
              posterUrl: show.posterUrl,
              backdropUrl: show.backdropUrl,
            }
          : undefined;

      const success = await startPlayback(input, metadata);
      if (success) {
        if (episode) setCurrentEpisode(episode);
        if (show) setCurrentShow(show);
      }
      return success;
    },
    [startPlayback],
  );

  const startMoviePlayback = useCallback(
    async (
      movieId: string,
      mediaFileId: string,
      movie?: Movie,
      startPosition?: number,
      duration?: number,
    ): Promise<boolean> => {
      const input: StartPlaybackInput = {
        contentType: "MOVIE",
        contentId: movieId,
        mediaFileId,
        startPosition,
        duration,
      };

      const metadata: CurrentContentMetadata | undefined = movie
        ? {
            contentType: "MOVIE",
            title: movie.title,
            subtitle: movie.year != null ? `${movie.year}` : undefined,
            posterUrl: movie.collectionPosterUrl ?? undefined,
            backdropUrl: undefined,
          }
        : undefined;

      const success = await startPlayback(input, metadata);
      if (success && movie) {
        setCurrentMovie(movie);
      }
      return success;
    },
    [startPlayback],
  );

  const updatePlayback = useCallback(
    async (input: UpdatePlaybackInput): Promise<boolean> => {
      if (!session?.id) return false;

      if (input.currentPosition !== undefined) {
        const diff = Math.abs(
          input.currentPosition - lastSyncedPosition.current,
        );
        if (diff < 1) {
          return true;
        }
        lastSyncedPosition.current = input.currentPosition;
      }

      // Optimistically update isPlaying state immediately for responsive UI
      if (input.isPlaying !== undefined) {
        setSession((prev) =>
          prev ? { ...prev, isPlaying: input.isPlaying! } : prev,
        );
      }

      try {
        const now = new Date().toISOString();
        const result = await apolloClient.mutate<{
          updatePlaybackSession: {
            success: boolean;
            error?: string | null;
            playbackSession?: {
              id: string;
              userId: string;
              mediaFileId?: string | null;
              currentPosition: number;
              duration?: number | null;
              volume: number;
              isMuted: boolean;
              isPlaying: boolean;
              startedAt: string;
              lastUpdatedAt: string;
              completedAt?: string | null;
              createdAt: string;
              updatedAt: string;
              contentType?: string | null;
              episodeId?: string | null;
              movieId?: string | null;
              trackId?: string | null;
              audiobookId?: string | null;
              tvShowId?: string | null;
              albumId?: string | null;
            } | null;
          };
        }>({
          mutation: UpdatePlaybackSessionContextDocument,
          variables: {
            id: session.id,
            input: {
              currentPosition: input.currentPosition,
              duration: input.duration,
              isPlaying: input.isPlaying,
              isMuted: input.isMuted,
              volume: input.volume,
              lastUpdatedAt: now,
            },
          },
        });

        if (
          result.data?.updatePlaybackSession?.success &&
          result.data.updatePlaybackSession.playbackSession
        ) {
          const updatedSession = mapNodeToSession(
            result.data.updatePlaybackSession.playbackSession,
          );
          setSession(updatedSession);
          const shouldSyncProgress = Boolean(
            session?.mediaFileId &&
            (input.currentPosition !== undefined ||
              input.duration !== undefined ||
              input.isPlaying === false),
          );
          if (shouldSyncProgress && session?.mediaFileId) {
            const currentPosition =
              input.currentPosition ?? updatedSession.currentPosition ?? 0;
            const duration =
              input.duration ?? updatedSession.duration ?? undefined;
            void upsertPlaybackProgress(
              session.mediaFileId,
              currentPosition,
              duration,
              input.isPlaying === false &&
                Boolean(
                  duration && duration > 0 && currentPosition / duration >= 0.9,
                ),
            );
          }
          return true;
        }
        if (result.data?.updatePlaybackSession?.error) {
          console.error(
            "Failed to update playback:",
            result.data.updatePlaybackSession.error,
          );
        }
        return false;
      } catch (err) {
        console.error("Failed to update playback:", err);
        return false;
      }
    },
    [session?.id, session?.mediaFileId, upsertPlaybackProgress],
  );

  const stopPlayback = useCallback(
    async (
      finalPosition?: number,
      finalDuration?: number,
    ): Promise<boolean> => {
      if (!session?.id) {
        clearAllState();
        lastSyncedPosition.current = 0;
        return true;
      }

      try {
        const now = new Date().toISOString();
        const positionToPersist = Math.max(
          0,
          finalPosition ?? session.currentPosition ?? 0,
        );
        const durationToPersist =
          finalDuration ?? session.duration ?? undefined;
        if (session.mediaFileId) {
          await upsertPlaybackProgress(
            session.mediaFileId,
            positionToPersist,
            durationToPersist,
            Boolean(
              durationToPersist &&
              durationToPersist > 0 &&
              positionToPersist / durationToPersist >= 0.9,
            ),
          );
        }
        const result = await apolloClient.mutate<{
          updatePlaybackSession: {
            success: boolean;
            error?: string | null;
          };
        }>({
          mutation: UpdatePlaybackSessionContextDocument,
          variables: {
            id: session.id,
            input: {
              currentPosition: positionToPersist,
              duration: durationToPersist,
              isPlaying: false,
              completedAt: now,
              lastUpdatedAt: now,
            },
          },
        });

        if (result.data?.updatePlaybackSession?.success) {
          clearAllState();
          lastSyncedPosition.current = 0;
          return true;
        }
        if (result.data?.updatePlaybackSession?.error) {
          console.error(
            "Failed to stop playback:",
            result.data.updatePlaybackSession.error,
          );
        }
        return false;
      } catch (err) {
        console.error("Failed to stop playback:", err);
        return false;
      }
    },
    [
      clearAllState,
      session?.currentPosition,
      session?.duration,
      session?.id,
      session?.mediaFileId,
      upsertPlaybackProgress,
    ],
  );

  // Build queue from tracks or chapters
  const buildQueueFromTracks = useCallback(
    (tracks: Track[], album: Album): QueueItem[] => {
      return tracks
        .filter((t) => t.mediaFileId) // Only include tracks with media files
        .sort((a, b) => {
          // Sort by disc then track number
          if (a.discNumber !== b.discNumber) return a.discNumber - b.discNumber;
          return a.trackNumber - b.trackNumber;
        })
        .map((track) => ({
          id: track.id,
          mediaFileId: track.mediaFileId!,
          title: track.title,
          artist: track.artistName || undefined,
          duration: track.durationSecs || undefined,
          coverUrl: album.coverUrl,
          track,
        }));
    },
    [],
  );

  const buildQueueFromChapters = useCallback(
    (chapters: AudiobookChapter[], audiobook: Audiobook): QueueItem[] => {
      return chapters
        .filter((c) => c.mediaFileId) // Only include chapters with media files
        .sort((a, b) => a.chapterNumber - b.chapterNumber)
        .map((chapter) => ({
          id: chapter.id,
          mediaFileId: chapter.mediaFileId!,
          title: chapter.title || `Chapter ${chapter.chapterNumber}`,
          duration: chapter.durationSecs || undefined,
          coverUrl: audiobook.coverUrl,
          chapter,
        }));
    },
    [],
  );

  // Start track playback with queue
  const startTrackPlayback = useCallback(
    async (
      track: Track,
      album: Album,
      allTracks: Track[],
      startPosition?: number,
    ): Promise<boolean> => {
      if (!track.mediaFileId) {
        console.error("Track has no media file");
        return false;
      }

      const input: StartPlaybackInput = {
        contentType: "TRACK",
        contentId: track.id,
        mediaFileId: track.mediaFileId,
        parentId: album.id,
        startPosition,
        duration: track.durationSecs || undefined,
      };

      const metadata: CurrentContentMetadata = {
        contentType: "TRACK",
        title: track.title,
        subtitle: track.artistName || album.name,
        posterUrl: album.coverUrl,
      };

      const success = await startPlayback(input, metadata);
      if (success) {
        setCurrentTrack(track);
        setCurrentAlbum(album);
        setCurrentAudiobook(null);
        setCurrentChapter(null);

        // Build queue from all tracks
        const newQueue = buildQueueFromTracks(allTracks, album);
        setQueue(newQueue);

        // Find current index in queue
        const idx = newQueue.findIndex((q) => q.id === track.id);
        setQueueIndex(idx >= 0 ? idx : 0);
      }
      return success;
    },
    [startPlayback, buildQueueFromTracks],
  );

  // Start audiobook playback with queue
  const startAudiobookPlayback = useCallback(
    async (
      audiobook: Audiobook,
      chapter: AudiobookChapter,
      allChapters: AudiobookChapter[],
      startPosition?: number,
    ): Promise<boolean> => {
      if (!chapter.mediaFileId) {
        console.error("Chapter has no media file");
        return false;
      }

      const input: StartPlaybackInput = {
        contentType: "AUDIOBOOK",
        contentId: audiobook.id,
        mediaFileId: chapter.mediaFileId,
        startPosition,
        duration: chapter.durationSecs || undefined,
      };

      const metadata: CurrentContentMetadata = {
        contentType: "AUDIOBOOK",
        title: audiobook.title,
        subtitle: chapter.title || `Chapter ${chapter.chapterNumber}`,
        posterUrl: audiobook.coverUrl,
      };

      const success = await startPlayback(input, metadata);
      if (success) {
        setCurrentAudiobook(audiobook);
        setCurrentChapter(chapter);
        setCurrentTrack(null);
        setCurrentAlbum(null);

        // Build queue from all chapters
        const newQueue = buildQueueFromChapters(allChapters, audiobook);
        setQueue(newQueue);

        // Find current index in queue
        const idx = newQueue.findIndex((q) => q.id === chapter.id);
        setQueueIndex(idx >= 0 ? idx : 0);
      }
      return success;
    },
    [startPlayback, buildQueueFromChapters],
  );

  // Play a specific queue item by index
  const playQueueItem = useCallback(
    async (index: number): Promise<boolean> => {
      if (index < 0 || index >= queue.length) return false;

      const item = queue[index];

      const input: StartPlaybackInput = {
        contentType: item.track ? "TRACK" : "AUDIOBOOK",
        contentId: item.track?.id || currentAudiobook?.id || item.id,
        mediaFileId: item.mediaFileId,
        parentId: item.track ? currentAlbum?.id : undefined,
        duration: item.duration,
      };

      const metadata: CurrentContentMetadata = {
        contentType: item.track ? "TRACK" : "AUDIOBOOK",
        title: item.title,
        subtitle: item.artist || currentAudiobook?.title,
        posterUrl: item.coverUrl,
      };

      const success = await startPlayback(input, metadata);
      if (success) {
        setQueueIndex(index);
        if (item.track) {
          setCurrentTrack(item.track);
        } else if (item.chapter) {
          setCurrentChapter(item.chapter);
        }
      }
      return success;
    },
    [queue, currentAlbum, currentAudiobook, startPlayback],
  );

  // Play next track/chapter in queue
  const playNext = useCallback(async (): Promise<boolean> => {
    if (queue.length === 0) return false;

    let nextIndex = queueIndex + 1;

    // Handle repeat modes
    if (nextIndex >= queue.length) {
      if (repeatMode === "all") {
        nextIndex = 0;
      } else {
        return false; // End of queue
      }
    }

    return playQueueItem(nextIndex);
  }, [queue, queueIndex, repeatMode, playQueueItem]);

  // Play previous track/chapter in queue
  const playPrevious = useCallback(async (): Promise<boolean> => {
    if (queue.length === 0) return false;

    let prevIndex = queueIndex - 1;

    // Handle repeat modes
    if (prevIndex < 0) {
      if (repeatMode === "all") {
        prevIndex = queue.length - 1;
      } else {
        prevIndex = 0; // Stay at beginning
      }
    }

    return playQueueItem(prevIndex);
  }, [queue, queueIndex, repeatMode, playQueueItem]);

  // Toggle shuffle
  const toggleShuffle = useCallback(() => {
    setShuffleEnabled((prev) => !prev);
    // Note: Actually shuffling the queue would be handled in playNext
    // For simplicity, shuffle just randomizes the next pick
  }, []);

  // Set repeat mode
  const setRepeatMode = useCallback((mode: RepeatMode) => {
    setRepeatModeState(mode);
  }, []);

  return (
    <PlaybackContext.Provider
      value={{
        session,
        isLoading,
        currentContent,
        currentEpisode,
        currentShow,
        currentMovie,
        shouldExpand,

        // Audio-specific state
        currentTrack,
        currentAlbum,
        currentAudiobook,
        currentChapter,
        queue,
        queueIndex,
        shuffleEnabled,
        repeatMode,

        // Base playback methods
        startPlayback,
        startEpisodePlayback,
        startMoviePlayback,

        // Audio-specific methods
        startTrackPlayback,
        startAudiobookPlayback,
        playNext,
        playPrevious,
        playQueueItem,
        toggleShuffle,
        setRepeatMode,

        // Common methods
        updatePlayback,
        stopPlayback,
        refreshSession,
        setCurrentEpisode,
        setCurrentShow,
        setCurrentMovie,
        clearExpandFlag,
      }}
    >
      {children}
    </PlaybackContext.Provider>
  );
}

export function usePlaybackContext(): PlaybackContextValue {
  const context = useContext(PlaybackContext);
  if (!context) {
    throw new Error(
      "usePlaybackContext must be used within a PlaybackProvider",
    );
  }
  return context;
}

export function useOptionalPlaybackContext(): PlaybackContextValue | null {
  return useContext(PlaybackContext);
}
