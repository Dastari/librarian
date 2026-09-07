import { useState, useEffect, useMemo } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { Spinner } from "@heroui/spinner";
import { Select, SelectItem } from "@heroui/select";
import { Chip } from "@heroui/chip";
import { Input } from "@heroui/input";
import { addToast } from "@heroui/toast";
import {
  IconDeviceTv,
  IconMovie,
  IconMusic,
  IconHeadphones,
  IconLink,
  IconSearch,
  IconFile,
  IconAlertCircle,
} from "@tabler/icons-react";
import { apolloClient, useMutation } from "../../lib/graphql/client";
import { formatBytes } from "../../lib/format";
import {
  type LibraryUnmatchedMediaFilesTabQuery,
  ManualMatchAlbumsByLibraryDocument,
  ManualMatchAudiobooksByLibraryDocument,
  ManualMatchFileDocument,
  ManualMatchMoviesByLibraryDocument,
  ManualMatchShowsByLibraryDocument,
  type ManualMatchAlbumsByLibraryQuery,
  type ManualMatchAudiobooksByLibraryQuery,
  type ManualMatchFileMutation,
  type ManualMatchFileMutationVariables,
  type ManualMatchMoviesByLibraryQuery,
  type ManualMatchShowsByLibraryQuery,
} from "../../lib/graphql/generated/graphql";

interface TvShow {
  id: string;
  name: string;
  year: number | null;
  seasons: Season[];
}

interface Season {
  id: string;
  seasonNumber: number;
  episodeCount: number;
  episodes: Episode[];
}

interface Episode {
  id: string;
  episodeNumber: number;
  name: string | null;
}

type Movie = ManualMatchMoviesByLibraryQuery["movies"]["edges"][number]["node"];

interface Album {
  id: string;
  name: string;
  year: number | null;
  artist: string | null;
  tracks: Track[];
}

type AlbumTrackNode =
  ManualMatchAlbumsByLibraryQuery["tracks"]["edges"][number]["node"];

interface Track {
  id: string;
  trackNumber: number | null;
  title: string | null;
}

interface Audiobook {
  id: string;
  title: string;
  author: string | null;
  chapters: Chapter[];
}

interface Chapter {
  id: string;
  chapterNumber: number | null;
  title: string | null;
}

type MatchableMediaFile =
  LibraryUnmatchedMediaFilesTabQuery["mediaFiles"]["edges"][number]["node"];

export interface ManualMatchModalProps {
  isOpen: boolean;
  onClose: () => void;
  mediaFile: MatchableMediaFile | null;
  libraryId: string;
  libraryType: string;
  onMatched: () => void;
}

export function ManualMatchModal({
  isOpen,
  onClose,
  mediaFile,
  libraryId,
  libraryType,
  onMatched,
}: ManualMatchModalProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  // Library items
  const [tvShows, setTvShows] = useState<TvShow[]>([]);
  const [movies, setMovies] = useState<Movie[]>([]);
  const [albums, setAlbums] = useState<Album[]>([]);
  const [audiobooks, setAudiobooks] = useState<Audiobook[]>([]);

  // Selection state
  const [selectedShowId, setSelectedShowId] = useState<string>("");
  const [selectedSeasonNumber, setSelectedSeasonNumber] = useState<string>("");
  const [selectedEpisodeId, setSelectedEpisodeId] = useState<string>("");
  const [selectedMovieId, setSelectedMovieId] = useState<string>("");
  const [selectedAlbumId, setSelectedAlbumId] = useState<string>("");
  const [selectedTrackId, setSelectedTrackId] = useState<string>("");
  const [selectedAudiobookId, setSelectedAudiobookId] = useState<string>("");
  const [selectedChapterId, setSelectedChapterId] = useState<string>("");
  const [manualMatchFile] = useMutation<
    ManualMatchFileMutation,
    ManualMatchFileMutationVariables
  >(ManualMatchFileDocument);

  // Normalize library type
  const normalizedType = libraryType.toUpperCase();

  // Fetch library items when modal opens
  useEffect(() => {
    if (!isOpen || !libraryId) return;

    const fetchItems = async () => {
      setIsLoading(true);
      setError(null);

      try {
        if (normalizedType === "TV") {
          const result =
            await apolloClient.query<ManualMatchShowsByLibraryQuery>({
              query: ManualMatchShowsByLibraryDocument,
              variables: { libraryId: libraryId },
              fetchPolicy: "network-only",
            });

          if (result.data?.shows?.edges) {
            const list: TvShow[] = result.data.shows.edges.map((e) => {
              const n = e.node;
              const bySeason = new Map<number, Episode[]>();
              for (const episodeEdge of n.episodes?.edges ?? []) {
                const ep = episodeEdge.node;
                const list = bySeason.get(ep.season) ?? [];
                list.push({
                  id: ep.id,
                  episodeNumber: ep.episode,
                  name: ep.title ?? null,
                });
                bySeason.set(ep.season, list);
              }
              const seasons: Season[] = Array.from(bySeason.entries())
                .sort((a, b) => a[0] - b[0])
                .map(([seasonNumber, episodes]) => ({
                  id: `s${seasonNumber}`,
                  seasonNumber,
                  episodeCount: episodes.length,
                  episodes: episodes.sort(
                    (a, b) => a.episodeNumber - b.episodeNumber,
                  ),
                }));
              return {
                id: n.id,
                name: n.name,
                year: n.year ?? null,
                seasons,
              };
            });
            setTvShows(list);
          }
        } else if (normalizedType === "MOVIES") {
          const result =
            await apolloClient.query<ManualMatchMoviesByLibraryQuery>({
              query: ManualMatchMoviesByLibraryDocument,
              variables: { libraryId: libraryId },
              fetchPolicy: "network-only",
            });

          if (result.data?.movies?.edges) {
            setMovies(result.data.movies.edges.map((e) => e.node));
          }
        } else if (normalizedType === "MUSIC") {
          const result =
            await apolloClient.query<ManualMatchAlbumsByLibraryQuery>({
              query: ManualMatchAlbumsByLibraryDocument,
              variables: { libraryId: libraryId },
              fetchPolicy: "network-only",
            });

          if (result.data?.albums?.edges) {
            const tracksByAlbum = new Map<string, AlbumTrackNode[]>();
            for (const edge of result.data.tracks?.edges ?? []) {
              const track = edge.node;
              const tracks = tracksByAlbum.get(track.albumId) ?? [];
              tracks.push(track);
              tracksByAlbum.set(track.albumId, tracks);
            }
            setAlbums(
              result.data.albums.edges.map((edge) => ({
                id: edge.node.id,
                name: edge.node.name,
                year: edge.node.year ?? null,
                artist:
                  tracksByAlbum
                    .get(edge.node.id)
                    ?.find((t) => Boolean(t.artistName))?.artistName ?? null,
                tracks: (tracksByAlbum.get(edge.node.id) ?? []).map(
                  (track) => ({
                    id: track.id,
                    trackNumber: track.trackNumber,
                    title: track.title ?? null,
                  }),
                ),
              })),
            );
          }
        } else if (normalizedType === "AUDIOBOOKS") {
          const result =
            await apolloClient.query<ManualMatchAudiobooksByLibraryQuery>({
              query: ManualMatchAudiobooksByLibraryDocument,
              variables: { libraryId: libraryId },
              fetchPolicy: "network-only",
            });

          if (result.data?.audiobooks?.edges) {
            setAudiobooks(
              result.data.audiobooks.edges.map((edge) => ({
                id: edge.node.id,
                title: edge.node.title,
                author: edge.node.authorName ?? null,
                chapters: (edge.node.chapters?.edges ?? []).map((ch) => ({
                  id: ch.node.id,
                  chapterNumber: ch.node.chapterNumber,
                  title: ch.node.title ?? null,
                })),
              })),
            );
          }
        }
      } catch (err) {
        setError(
          err instanceof Error ? err.message : "Failed to load library items",
        );
      } finally {
        setIsLoading(false);
      }
    };

    fetchItems();
  }, [isOpen, libraryId, normalizedType]);

  // Reset selection when modal closes
  useEffect(() => {
    if (!isOpen) {
      setSelectedShowId("");
      setSelectedSeasonNumber("");
      setSelectedEpisodeId("");
      setSelectedMovieId("");
      setSelectedAlbumId("");
      setSelectedTrackId("");
      setSelectedAudiobookId("");
      setSelectedChapterId("");
      setSearchQuery("");
    }
  }, [isOpen]);

  // Get selected show/album/audiobook details
  const selectedShow = useMemo(
    () => tvShows.find((s) => s.id === selectedShowId),
    [tvShows, selectedShowId],
  );
  const selectedSeason = useMemo(
    () =>
      selectedShow?.seasons.find(
        (s) => s.seasonNumber.toString() === selectedSeasonNumber,
      ),
    [selectedShow, selectedSeasonNumber],
  );
  const selectedAlbum = useMemo(
    () => albums.find((a) => a.id === selectedAlbumId),
    [albums, selectedAlbumId],
  );
  const selectedAudiobook = useMemo(
    () => audiobooks.find((a) => a.id === selectedAudiobookId),
    [audiobooks, selectedAudiobookId],
  );

  // Filter items by search query
  const filteredShows = useMemo(() => {
    if (!searchQuery) return tvShows;
    const q = searchQuery.toLowerCase();
    return tvShows.filter((s) => s.name.toLowerCase().includes(q));
  }, [tvShows, searchQuery]);

  const filteredMovies = useMemo(() => {
    if (!searchQuery) return movies;
    const q = searchQuery.toLowerCase();
    return movies.filter((m) => m.title.toLowerCase().includes(q));
  }, [movies, searchQuery]);

  const filteredAlbums = useMemo(() => {
    if (!searchQuery) return albums;
    const q = searchQuery.toLowerCase();
    return albums.filter(
      (a) =>
        a.name.toLowerCase().includes(q) || a.artist?.toLowerCase().includes(q),
    );
  }, [albums, searchQuery]);

  const filteredAudiobooks = useMemo(() => {
    if (!searchQuery) return audiobooks;
    const q = searchQuery.toLowerCase();
    return audiobooks.filter(
      (a) =>
        a.title.toLowerCase().includes(q) ||
        a.author?.toLowerCase().includes(q),
    );
  }, [audiobooks, searchQuery]);

  // Check if we have a valid selection
  const hasValidSelection = useMemo(() => {
    if (normalizedType === "TV") return !!selectedEpisodeId;
    if (normalizedType === "MOVIES") return !!selectedMovieId;
    if (normalizedType === "MUSIC") return !!selectedTrackId;
    if (normalizedType === "AUDIOBOOKS") return !!selectedChapterId;
    return false;
  }, [
    normalizedType,
    selectedEpisodeId,
    selectedMovieId,
    selectedTrackId,
    selectedChapterId,
  ]);

  const handleMatch = async () => {
    if (!mediaFile || !hasValidSelection) return;

    setIsSubmitting(true);
    setError(null);

    try {
      const result = await manualMatchFile({
        variables: {
          input: {
            mediaFileId: mediaFile.id,
            libraryId: libraryId || undefined,
            episodeId: selectedEpisodeId || undefined,
            movieId: selectedMovieId || undefined,
            trackId: selectedTrackId || undefined,
            chapterId: selectedChapterId || undefined,
          },
        },
      });

      if (result.data?.matchMediaFile?.success) {
        addToast({
          title: "File Matched",
          description:
            "The file has been manually matched to the selected item",
          color: "success",
        });
        onMatched();
        onClose();
      } else {
        setError(result.data?.matchMediaFile?.reason || "Failed to match file");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "An error occurred");
    } finally {
      setIsSubmitting(false);
    }
  };

  const getFileName = (path: string) => {
    const parts = path.split("/");
    return parts[parts.length - 1];
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="2xl" scrollBehavior="inside">
      <ModalContent>
        <ModalHeader className="flex items-center gap-2">
          <IconLink size={20} className="text-primary" />
          Manual Match
        </ModalHeader>
        <ModalBody>
          {/* File info */}
          {mediaFile && (
            <Card className="mb-4">
              <CardBody className="py-3">
                <div className="flex items-start gap-3">
                  <IconFile
                    size={24}
                    className="text-default-400 mt-1 flex-shrink-0"
                  />
                  <div className="min-w-0">
                    <p className="font-medium truncate">
                      {getFileName(mediaFile.path)}
                    </p>
                    <p className="text-sm text-default-500 truncate">
                      {mediaFile.path}
                    </p>
                    <div className="flex items-center gap-2 mt-1">
                      <Chip size="sm" variant="flat">
                        {formatBytes(mediaFile.size)}
                      </Chip>
                      {mediaFile.resolution && (
                        <Chip size="sm" variant="flat" color="primary">
                          {mediaFile.resolution}
                        </Chip>
                      )}
                      {mediaFile.videoCodec && (
                        <Chip size="sm" variant="flat">
                          {mediaFile.videoCodec}
                        </Chip>
                      )}
                    </div>
                  </div>
                </div>
              </CardBody>
            </Card>
          )}

          {/* Search input */}
          <Input
            placeholder="Search..."
            value={searchQuery}
            onValueChange={setSearchQuery}
            startContent={<IconSearch size={16} className="text-default-400" />}
            className="mb-4"
          />

          {/* Error display */}
          {error && (
            <Card className="mb-4 bg-danger-50 dark:bg-danger-900/20">
              <CardBody className="py-3">
                <div className="flex items-center gap-2 text-danger">
                  <IconAlertCircle size={20} />
                  <span>{error}</span>
                </div>
              </CardBody>
            </Card>
          )}

          {/* Loading state */}
          {isLoading ? (
            <div className="flex justify-center py-8">
              <Spinner size="lg" />
            </div>
          ) : (
            <div className="space-y-4">
              {/* TV Shows selection */}
              {normalizedType === "TV" && (
                <>
                  <Select
                    label="Select Show"
                    placeholder="Choose a TV show"
                    selectedKeys={selectedShowId ? [selectedShowId] : []}
                    onSelectionChange={(keys) => {
                      const key = Array.from(keys)[0]?.toString() || "";
                      setSelectedShowId(key);
                      setSelectedSeasonNumber("");
                      setSelectedEpisodeId("");
                    }}
                    startContent={
                      <IconDeviceTv size={16} className="text-blue-400" />
                    }
                  >
                    {filteredShows.map((show) => (
                      <SelectItem key={show.id} textValue={show.name}>
                        {show.name} {show.year && `(${show.year})`}
                      </SelectItem>
                    ))}
                  </Select>

                  {selectedShow && (
                    <Select
                      label="Select Season"
                      placeholder="Choose a season"
                      selectedKeys={
                        selectedSeasonNumber ? [selectedSeasonNumber] : []
                      }
                      onSelectionChange={(keys) => {
                        const key = Array.from(keys)[0]?.toString() || "";
                        setSelectedSeasonNumber(key);
                        setSelectedEpisodeId("");
                      }}
                    >
                      {selectedShow.seasons.map((season) => (
                        <SelectItem
                          key={season.seasonNumber.toString()}
                          textValue={`Season ${season.seasonNumber}`}
                        >
                          Season {season.seasonNumber} ({season.episodeCount}{" "}
                          episodes)
                        </SelectItem>
                      ))}
                    </Select>
                  )}

                  {selectedSeason && (
                    <Select
                      label="Select Episode"
                      placeholder="Choose an episode"
                      selectedKeys={
                        selectedEpisodeId ? [selectedEpisodeId] : []
                      }
                      onSelectionChange={(keys) => {
                        const key = Array.from(keys)[0]?.toString() || "";
                        setSelectedEpisodeId(key);
                      }}
                    >
                      {selectedSeason.episodes.map((ep) => (
                        <SelectItem
                          key={ep.id}
                          textValue={`Episode ${ep.episodeNumber}`}
                        >
                          Episode {ep.episodeNumber}
                          {ep.name && `: ${ep.name}`}
                        </SelectItem>
                      ))}
                    </Select>
                  )}
                </>
              )}

              {/* Movies selection */}
              {normalizedType === "MOVIES" && (
                <Select
                  label="Select Movie"
                  placeholder="Choose a movie"
                  selectedKeys={selectedMovieId ? [selectedMovieId] : []}
                  onSelectionChange={(keys) => {
                    const key = Array.from(keys)[0]?.toString() || "";
                    setSelectedMovieId(key);
                  }}
                  startContent={
                    <IconMovie size={16} className="text-purple-400" />
                  }
                >
                  {filteredMovies.map((movie) => (
                    <SelectItem key={movie.id} textValue={movie.title}>
                      {movie.title} {movie.year && `(${movie.year})`}
                    </SelectItem>
                  ))}
                </Select>
              )}

              {/* Music selection */}
              {normalizedType === "MUSIC" && (
                <>
                  <Select
                    label="Select Album"
                    placeholder="Choose an album"
                    selectedKeys={selectedAlbumId ? [selectedAlbumId] : []}
                    onSelectionChange={(keys) => {
                      const key = Array.from(keys)[0]?.toString() || "";
                      setSelectedAlbumId(key);
                      setSelectedTrackId("");
                    }}
                    startContent={
                      <IconMusic size={16} className="text-green-400" />
                    }
                  >
                    {filteredAlbums.map((album) => (
                      <SelectItem key={album.id} textValue={album.name}>
                        {album.name} {album.artist && `- ${album.artist}`}{" "}
                        {album.year && `(${album.year})`}
                      </SelectItem>
                    ))}
                  </Select>

                  {selectedAlbum && (
                    <Select
                      label="Select Track"
                      placeholder="Choose a track"
                      selectedKeys={selectedTrackId ? [selectedTrackId] : []}
                      onSelectionChange={(keys) => {
                        const key = Array.from(keys)[0]?.toString() || "";
                        setSelectedTrackId(key);
                      }}
                    >
                      {selectedAlbum.tracks.map((track) => (
                        <SelectItem
                          key={track.id}
                          textValue={`Track ${track.trackNumber}`}
                        >
                          {track.trackNumber}. {track.title || "Untitled"}
                        </SelectItem>
                      ))}
                    </Select>
                  )}
                </>
              )}

              {/* Audiobooks selection */}
              {normalizedType === "AUDIOBOOKS" && (
                <>
                  <Select
                    label="Select Audiobook"
                    placeholder="Choose an audiobook"
                    selectedKeys={
                      selectedAudiobookId ? [selectedAudiobookId] : []
                    }
                    onSelectionChange={(keys) => {
                      const key = Array.from(keys)[0]?.toString() || "";
                      setSelectedAudiobookId(key);
                      setSelectedChapterId("");
                    }}
                    startContent={
                      <IconHeadphones size={16} className="text-orange-400" />
                    }
                  >
                    {filteredAudiobooks.map((book) => (
                      <SelectItem key={book.id} textValue={book.title}>
                        {book.title} {book.author && `- ${book.author}`}
                      </SelectItem>
                    ))}
                  </Select>

                  {selectedAudiobook &&
                    selectedAudiobook.chapters.length > 0 && (
                      <Select
                        label="Select Chapter (Optional)"
                        placeholder="Choose a chapter"
                        selectedKeys={
                          selectedChapterId ? [selectedChapterId] : []
                        }
                        onSelectionChange={(keys) => {
                          const key = Array.from(keys)[0]?.toString() || "";
                          setSelectedChapterId(key);
                        }}
                      >
                        {selectedAudiobook.chapters.map((chapter) => (
                          <SelectItem
                            key={chapter.id}
                            textValue={`Chapter ${chapter.chapterNumber}`}
                          >
                            {chapter.chapterNumber}.{" "}
                            {chapter.title || "Untitled"}
                          </SelectItem>
                        ))}
                      </Select>
                    )}
                </>
              )}
            </div>
          )}

          {/* Warning about manual matches */}
          <Card className="mt-4 bg-warning-50 dark:bg-warning-900/20">
            <CardBody className="py-3">
              <p className="text-sm text-warning-700 dark:text-warning-300">
                <strong>Note:</strong> Manual matches will never be overwritten
                by automatic scanning or matching. To change this match later,
                you'll need to unmatch and re-match manually.
              </p>
            </CardBody>
          </Card>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            onPress={handleMatch}
            isLoading={isSubmitting}
            isDisabled={!hasValidSelection || isLoading}
            startContent={<IconLink size={16} />}
          >
            Match File
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
