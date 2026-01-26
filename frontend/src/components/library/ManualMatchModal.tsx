import { useState, useEffect, useMemo } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Spinner } from '@heroui/spinner'
import { Select, SelectItem } from '@heroui/select'
import { Chip } from '@heroui/chip'
import { Input } from '@heroui/input'
import { addToast } from '@heroui/toast'
import {
  IconDeviceTv,
  IconMovie,
  IconMusic,
  IconHeadphones,
  IconLink,
  IconSearch,
  IconFile,
  IconAlertCircle,
} from '@tabler/icons-react'
import {
  graphqlClient,
  MANUAL_MATCH_MUTATION,
  type MediaFile,
  type ManualMatchResult,
} from '../../lib/graphql'

// Queries for fetching library items (PascalCase schema)
const TV_SHOWS_QUERY = `
  query TvShows($libraryId: String!) {
    Shows(Where: { LibraryId: { Eq: $libraryId } }) {
      Edges {
        Node {
          Id
          Name
          Year
          Episodes {
            Id
            Season
            Episode
            Title
          }
        }
      }
    }
  }
`

const MOVIES_QUERY = `
  query Movies($libraryId: String!) {
    Movies(Where: { LibraryId: { Eq: $libraryId } }) {
      Edges {
        Node {
          Id
          Title
          Year
        }
      }
    }
  }
`

const ALBUMS_QUERY = `
  query Albums($libraryId: String!) {
    albums(libraryId: $libraryId) {
      id
      name
      year
      artist
      tracks {
        id
        trackNumber
        title
      }
    }
  }
`

const AUDIOBOOKS_QUERY = `
  query Audiobooks($libraryId: String!) {
    audiobooks(libraryId: $libraryId) {
      id
      title
      author
      chapters {
        id
        chapterNumber
        title
      }
    }
  }
`

interface TvShow {
  id: string
  name: string
  year: number | null
  seasons: Season[]
}

/** Raw show node from GraphQL (Shows.Edges[].Node with Episodes) */
interface ShowNode {
  Id: string
  Name: string
  Year: number | null
  Episodes: Array< { Id: string; Season: number; Episode: number; Title: string | null } >
}

interface Season {
  id: string
  seasonNumber: number
  episodeCount: number
  episodes: Episode[]
}

interface Episode {
  id: string
  episodeNumber: number
  name: string | null
}

interface Movie {
  Id: string
  Title: string
  Year: number | null
}

interface Album {
  id: string
  name: string
  year: number | null
  artist: string | null
  tracks: Track[]
}

interface Track {
  id: string
  trackNumber: number | null
  title: string | null
}

interface Audiobook {
  id: string
  title: string
  author: string | null
  chapters: Chapter[]
}

interface Chapter {
  id: string
  chapterNumber: number | null
  title: string | null
}

export interface ManualMatchModalProps {
  isOpen: boolean
  onClose: () => void
  mediaFile: MediaFile | null
  libraryId: string
  libraryType: string
  onMatched: () => void
}

export function ManualMatchModal({
  isOpen,
  onClose,
  mediaFile,
  libraryId,
  libraryType,
  onMatched,
}: ManualMatchModalProps) {
  const [isLoading, setIsLoading] = useState(false)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState('')

  // Library items
  const [tvShows, setTvShows] = useState<TvShow[]>([])
  const [movies, setMovies] = useState<Movie[]>([])
  const [albums, setAlbums] = useState<Album[]>([])
  const [audiobooks, setAudiobooks] = useState<Audiobook[]>([])

  // Selection state
  const [selectedShowId, setSelectedShowId] = useState<string>('')
  const [selectedSeasonNumber, setSelectedSeasonNumber] = useState<string>('')
  const [selectedEpisodeId, setSelectedEpisodeId] = useState<string>('')
  const [selectedMovieId, setSelectedMovieId] = useState<string>('')
  const [selectedAlbumId, setSelectedAlbumId] = useState<string>('')
  const [selectedTrackId, setSelectedTrackId] = useState<string>('')
  const [selectedAudiobookId, setSelectedAudiobookId] = useState<string>('')
  const [selectedChapterId, setSelectedChapterId] = useState<string>('')

  // Normalize library type
  const normalizedType = libraryType.toUpperCase()

  // Fetch library items when modal opens
  useEffect(() => {
    if (!isOpen || !libraryId) return

    const fetchItems = async () => {
      setIsLoading(true)
      setError(null)

      try {
        if (normalizedType === 'TV') {
          const result = await graphqlClient
            .query<{ Shows: { Edges: Array<{ Node: ShowNode }> } }>(TV_SHOWS_QUERY, { libraryId })
            .toPromise()
          if (result.data?.Shows?.Edges) {
            const list: TvShow[] = result.data.Shows.Edges.map((e) => {
              const n = e.Node
              const bySeason = new Map<number, Episode[]>()
              for (const ep of n.Episodes) {
                const list = bySeason.get(ep.Season) ?? []
                list.push({
                  id: ep.Id,
                  episodeNumber: ep.Episode,
                  name: ep.Title ?? null,
                })
                bySeason.set(ep.Season, list)
              }
              const seasons: Season[] = Array.from(bySeason.entries())
                .sort((a, b) => a[0] - b[0])
                .map(([seasonNumber, episodes]) => ({
                  id: `s${seasonNumber}`,
                  seasonNumber,
                  episodeCount: episodes.length,
                  episodes: episodes.sort((a, b) => a.episodeNumber - b.episodeNumber),
                }))
              return {
                id: n.Id,
                name: n.Name,
                year: n.Year ?? null,
                seasons,
              }
            })
            setTvShows(list)
          }
        } else if (normalizedType === 'MOVIES') {
          const result = await graphqlClient
            .query<{ Movies: { Edges: Array<{ Node: Movie }> } }>(MOVIES_QUERY, { libraryId })
            .toPromise()
          if (result.data?.Movies?.Edges) {
            setMovies(result.data.Movies.Edges.map((e) => e.Node))
          }
        } else if (normalizedType === 'MUSIC') {
          const result = await graphqlClient.query<{ albums: Album[] }>(ALBUMS_QUERY, { libraryId }).toPromise()
          if (result.data?.albums) {
            setAlbums(result.data.albums)
          }
        } else if (normalizedType === 'AUDIOBOOKS') {
          const result = await graphqlClient.query<{ audiobooks: Audiobook[] }>(AUDIOBOOKS_QUERY, { libraryId }).toPromise()
          if (result.data?.audiobooks) {
            setAudiobooks(result.data.audiobooks)
          }
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load library items')
      } finally {
        setIsLoading(false)
      }
    }

    fetchItems()
  }, [isOpen, libraryId, normalizedType])

  // Reset selection when modal closes
  useEffect(() => {
    if (!isOpen) {
      setSelectedShowId('')
      setSelectedSeasonNumber('')
      setSelectedEpisodeId('')
      setSelectedMovieId('')
      setSelectedAlbumId('')
      setSelectedTrackId('')
      setSelectedAudiobookId('')
      setSelectedChapterId('')
      setSearchQuery('')
    }
  }, [isOpen])

  // Get selected show/album/audiobook details
  const selectedShow = useMemo(() => tvShows.find(s => s.id === selectedShowId), [tvShows, selectedShowId])
  const selectedSeason = useMemo(() => selectedShow?.seasons.find(s => s.seasonNumber.toString() === selectedSeasonNumber), [selectedShow, selectedSeasonNumber])
  const selectedAlbum = useMemo(() => albums.find(a => a.id === selectedAlbumId), [albums, selectedAlbumId])
  const selectedAudiobook = useMemo(() => audiobooks.find(a => a.id === selectedAudiobookId), [audiobooks, selectedAudiobookId])

  // Filter items by search query
  const filteredShows = useMemo(() => {
    if (!searchQuery) return tvShows
    const q = searchQuery.toLowerCase()
    return tvShows.filter(s => s.name.toLowerCase().includes(q))
  }, [tvShows, searchQuery])

  const filteredMovies = useMemo(() => {
    if (!searchQuery) return movies
    const q = searchQuery.toLowerCase()
    return movies.filter(m => m.Title.toLowerCase().includes(q))
  }, [movies, searchQuery])

  const filteredAlbums = useMemo(() => {
    if (!searchQuery) return albums
    const q = searchQuery.toLowerCase()
    return albums.filter(a => a.name.toLowerCase().includes(q) || a.artist?.toLowerCase().includes(q))
  }, [albums, searchQuery])

  const filteredAudiobooks = useMemo(() => {
    if (!searchQuery) return audiobooks
    const q = searchQuery.toLowerCase()
    return audiobooks.filter(a => a.title.toLowerCase().includes(q) || a.author?.toLowerCase().includes(q))
  }, [audiobooks, searchQuery])

  // Check if we have a valid selection
  const hasValidSelection = useMemo(() => {
    if (normalizedType === 'TV') return !!selectedEpisodeId
    if (normalizedType === 'MOVIES') return !!selectedMovieId
    if (normalizedType === 'MUSIC') return !!selectedTrackId
    if (normalizedType === 'AUDIOBOOKS') return !!selectedChapterId || !!selectedAudiobookId
    return false
  }, [normalizedType, selectedEpisodeId, selectedMovieId, selectedTrackId, selectedChapterId, selectedAudiobookId])

  const handleMatch = async () => {
    if (!mediaFile || !hasValidSelection) return

    setIsSubmitting(true)
    setError(null)

    try {
      const result = await graphqlClient
        .mutation<{ manualMatch: ManualMatchResult }>(MANUAL_MATCH_MUTATION, {
          mediaFileId: mediaFile.id,
          episodeId: selectedEpisodeId || null,
          movieId: selectedMovieId || null,
          trackId: selectedTrackId || null,
          albumId: selectedAlbumId || null,
          audiobookId: selectedAudiobookId || null,
          chapterId: selectedChapterId || null,
        })
        .toPromise()

      if (result.error) {
        setError(result.error.message)
        return
      }

      if (result.data?.manualMatch.success) {
        addToast({
          title: 'File Matched',
          description: 'The file has been manually matched to the selected item',
          color: 'success',
        })
        onMatched()
        onClose()
      } else {
        setError(result.data?.manualMatch.error || 'Failed to match file')
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred')
    } finally {
      setIsSubmitting(false)
    }
  }

  const getFileName = (path: string) => {
    const parts = path.split('/')
    return parts[parts.length - 1]
  }

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
                  <IconFile size={24} className="text-default-400 mt-1 flex-shrink-0" />
                  <div className="min-w-0">
                    <p className="font-medium truncate">{getFileName(mediaFile.path)}</p>
                    <p className="text-sm text-default-500 truncate">{mediaFile.path}</p>
                    <div className="flex items-center gap-2 mt-1">
                      <Chip size="sm" variant="flat">{mediaFile.sizeFormatted}</Chip>
                      {mediaFile.resolution && <Chip size="sm" variant="flat" color="primary">{mediaFile.resolution}</Chip>}
                      {mediaFile.videoCodec && <Chip size="sm" variant="flat">{mediaFile.videoCodec}</Chip>}
                      {mediaFile.isManualMatch && (
                        <Chip size="sm" variant="flat" color="warning">Currently Manual Match</Chip>
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
              {normalizedType === 'TV' && (
                <>
                  <Select
                    label="Select Show"
                    placeholder="Choose a TV show"
                    selectedKeys={selectedShowId ? [selectedShowId] : []}
                    onSelectionChange={(keys) => {
                      const key = Array.from(keys)[0]?.toString() || ''
                      setSelectedShowId(key)
                      setSelectedSeasonNumber('')
                      setSelectedEpisodeId('')
                    }}
                    startContent={<IconDeviceTv size={16} className="text-blue-400" />}
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
                      selectedKeys={selectedSeasonNumber ? [selectedSeasonNumber] : []}
                      onSelectionChange={(keys) => {
                        const key = Array.from(keys)[0]?.toString() || ''
                        setSelectedSeasonNumber(key)
                        setSelectedEpisodeId('')
                      }}
                    >
                      {selectedShow.seasons.map((season) => (
                        <SelectItem key={season.seasonNumber.toString()} textValue={`Season ${season.seasonNumber}`}>
                          Season {season.seasonNumber} ({season.episodeCount} episodes)
                        </SelectItem>
                      ))}
                    </Select>
                  )}

                  {selectedSeason && (
                    <Select
                      label="Select Episode"
                      placeholder="Choose an episode"
                      selectedKeys={selectedEpisodeId ? [selectedEpisodeId] : []}
                      onSelectionChange={(keys) => {
                        const key = Array.from(keys)[0]?.toString() || ''
                        setSelectedEpisodeId(key)
                      }}
                    >
                      {selectedSeason.episodes.map((ep) => (
                        <SelectItem key={ep.id} textValue={`Episode ${ep.episodeNumber}`}>
                          Episode {ep.episodeNumber}{ep.name && `: ${ep.name}`}
                        </SelectItem>
                      ))}
                    </Select>
                  )}
                </>
              )}

              {/* Movies selection */}
              {normalizedType === 'MOVIES' && (
                <Select
                  label="Select Movie"
                  placeholder="Choose a movie"
                  selectedKeys={selectedMovieId ? [selectedMovieId] : []}
                  onSelectionChange={(keys) => {
                    const key = Array.from(keys)[0]?.toString() || ''
                    setSelectedMovieId(key)
                  }}
                  startContent={<IconMovie size={16} className="text-purple-400" />}
                >
                  {filteredMovies.map((movie) => (
                    <SelectItem key={movie.Id} textValue={movie.Title}>
                      {movie.Title} {movie.Year && `(${movie.Year})`}
                    </SelectItem>
                  ))}
                </Select>
              )}

              {/* Music selection */}
              {normalizedType === 'MUSIC' && (
                <>
                  <Select
                    label="Select Album"
                    placeholder="Choose an album"
                    selectedKeys={selectedAlbumId ? [selectedAlbumId] : []}
                    onSelectionChange={(keys) => {
                      const key = Array.from(keys)[0]?.toString() || ''
                      setSelectedAlbumId(key)
                      setSelectedTrackId('')
                    }}
                    startContent={<IconMusic size={16} className="text-green-400" />}
                  >
                    {filteredAlbums.map((album) => (
                      <SelectItem key={album.id} textValue={album.name}>
                        {album.name} {album.artist && `- ${album.artist}`} {album.year && `(${album.year})`}
                      </SelectItem>
                    ))}
                  </Select>

                  {selectedAlbum && (
                    <Select
                      label="Select Track"
                      placeholder="Choose a track"
                      selectedKeys={selectedTrackId ? [selectedTrackId] : []}
                      onSelectionChange={(keys) => {
                        const key = Array.from(keys)[0]?.toString() || ''
                        setSelectedTrackId(key)
                      }}
                    >
                      {selectedAlbum.tracks.map((track) => (
                        <SelectItem key={track.id} textValue={`Track ${track.trackNumber}`}>
                          {track.trackNumber}. {track.title || 'Untitled'}
                        </SelectItem>
                      ))}
                    </Select>
                  )}
                </>
              )}

              {/* Audiobooks selection */}
              {normalizedType === 'AUDIOBOOKS' && (
                <>
                  <Select
                    label="Select Audiobook"
                    placeholder="Choose an audiobook"
                    selectedKeys={selectedAudiobookId ? [selectedAudiobookId] : []}
                    onSelectionChange={(keys) => {
                      const key = Array.from(keys)[0]?.toString() || ''
                      setSelectedAudiobookId(key)
                      setSelectedChapterId('')
                    }}
                    startContent={<IconHeadphones size={16} className="text-orange-400" />}
                  >
                    {filteredAudiobooks.map((book) => (
                      <SelectItem key={book.id} textValue={book.title}>
                        {book.title} {book.author && `- ${book.author}`}
                      </SelectItem>
                    ))}
                  </Select>

                  {selectedAudiobook && selectedAudiobook.chapters.length > 0 && (
                    <Select
                      label="Select Chapter (Optional)"
                      placeholder="Choose a chapter"
                      selectedKeys={selectedChapterId ? [selectedChapterId] : []}
                      onSelectionChange={(keys) => {
                        const key = Array.from(keys)[0]?.toString() || ''
                        setSelectedChapterId(key)
                      }}
                    >
                      {selectedAudiobook.chapters.map((chapter) => (
                        <SelectItem key={chapter.id} textValue={`Chapter ${chapter.chapterNumber}`}>
                          {chapter.chapterNumber}. {chapter.title || 'Untitled'}
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
                <strong>Note:</strong> Manual matches will never be overwritten by automatic scanning or matching.
                To change this match later, you'll need to unmatch and re-match manually.
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
  )
}
