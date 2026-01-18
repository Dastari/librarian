import { useState, useEffect, useMemo } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Image } from '@heroui/image'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Input } from '@heroui/input'
import { Select, SelectItem } from '@heroui/select'
import { Chip } from '@heroui/chip'
import { Spinner } from '@heroui/spinner'
import { Switch } from '@heroui/switch'
import { Divider } from '@heroui/divider'
import { addToast } from '@heroui/toast'
import {
  IconDeviceTv,
  IconMovie,
  IconMusic,
  IconHeadphones,
  IconCategory,
  IconSearch,
  IconDownload,
  IconCheck,
} from '@tabler/icons-react'
import {
  graphqlClient,
  LIBRARIES_QUERY,
  SEARCH_TV_SHOWS_QUERY,
  SEARCH_MOVIES_QUERY,
  ADD_TV_SHOW_MUTATION,
  ADD_MOVIE_MUTATION,
  type Library,
  type LibraryType,
  type TvShowSearchResult,
  type MovieSearchResult,
  type TorrentRelease,
} from '../../lib/graphql'
// import { sanitizeError } from '../../lib/format'

export interface AddToLibraryModalProps {
  isOpen: boolean
  onClose: () => void
  release: TorrentRelease | null
  onAdded: () => void
}

type DetectedType = 'tv' | 'movies' | 'music' | 'audiobooks' | 'unknown'

interface ParsedInfo {
  type: DetectedType
  title: string
  year?: number
  season?: number
  episode?: number
}

// Parse torrent name to detect media type and extract info
function parseTorrentName(title: string): ParsedInfo {
  const cleaned = title.replace(/\./g, ' ').replace(/_/g, ' ')
  
  // Check for TV patterns: S01E01, 1x01, Season 1, etc.
  const tvPatterns = [
    /[Ss](\d{1,2})[Ee](\d{1,2})/,
    /(\d{1,2})x(\d{2})/,
    /[Ss]eason\s*(\d+)/i,
    /[Ee]pisode\s*(\d+)/i,
    /\b(HDTV|WEB-?DL|WEBRip)\b.*?(720p|1080p|2160p)/i,
  ]
  
  for (const pattern of tvPatterns) {
    if (pattern.test(cleaned)) {
      // Extract show name (everything before the season/episode marker)
      const match = cleaned.match(/^(.+?)\s*[Ss]\d|^(.+?)\s*\d{1,2}x\d{2}|^(.+?)\s*[Ss]eason/i)
      const showName = (match?.[1] || match?.[2] || match?.[3] || title.split(/[Ss]\d/)[0]).trim()
      
      const seasonMatch = cleaned.match(/[Ss](\d{1,2})|(\d{1,2})x\d{2}|[Ss]eason\s*(\d+)/i)
      const episodeMatch = cleaned.match(/[Ee](\d{1,2})|x(\d{2})|[Ee]pisode\s*(\d+)/i)
      
      return {
        type: 'tv',
        title: showName,
        season: parseInt(seasonMatch?.[1] || seasonMatch?.[2] || seasonMatch?.[3] || '0'),
        episode: parseInt(episodeMatch?.[1] || episodeMatch?.[2] || episodeMatch?.[3] || '0'),
      }
    }
  }
  
  // Check for movie patterns: Title (Year) or Title.Year
  const moviePattern = /^(.+?)[\s\.\(]*((?:19|20)\d{2})[\s\)\]\.]/
  const movieMatch = cleaned.match(moviePattern)
  if (movieMatch) {
    return {
      type: 'movies',
      title: movieMatch[1].trim(),
      year: parseInt(movieMatch[2]),
    }
  }
  
  // Check for music patterns: Artist - Album, FLAC, MP3, etc.
  const musicPatterns = [
    /\b(FLAC|MP3|320kbps|V0|ALAC)\b/i,
    /\b(Discography|Album|EP|Single)\b/i,
  ]
  for (const pattern of musicPatterns) {
    if (pattern.test(cleaned)) {
      return {
        type: 'music',
        title: cleaned.split(/\b(FLAC|MP3|320)/i)[0].trim(),
      }
    }
  }
  
  // Check for audiobook patterns
  const audiobookPatterns = [
    /\b(Audiobook|Audio\s*Book|M4B|Audible)\b/i,
    /\bnarrated\s*by\b/i,
  ]
  for (const pattern of audiobookPatterns) {
    if (pattern.test(cleaned)) {
      return {
        type: 'audiobooks',
        title: cleaned.split(/\b(Audiobook|M4B)/i)[0].trim(),
      }
    }
  }
  
  // Default: treat as movie (most common for releases without patterns)
  const yearMatch = cleaned.match(/((?:19|20)\d{2})/)
  return {
    type: 'movies',
    title: yearMatch ? cleaned.split(yearMatch[1])[0].trim() : cleaned.split(/\d{3,4}p/i)[0].trim(),
    year: yearMatch ? parseInt(yearMatch[1]) : undefined,
  }
}

// Map detected type to library type
const TYPE_TO_LIBRARY_TYPE: Record<DetectedType, LibraryType | null> = {
  tv: 'TV',
  movies: 'MOVIES',
  music: 'MUSIC',
  audiobooks: 'AUDIOBOOKS',
  unknown: null,
}

const TYPE_ICONS: Record<DetectedType, typeof IconDeviceTv> = {
  tv: IconDeviceTv,
  movies: IconMovie,
  music: IconMusic,
  audiobooks: IconHeadphones,
  unknown: IconCategory,
}

// Type guards for search results
function isTvShowSearchResult(item: TvShowSearchResult | MovieSearchResult): item is TvShowSearchResult {
  return 'name' in item && !('title' in item)
}

function isMovieSearchResult(item: TvShowSearchResult | MovieSearchResult): item is MovieSearchResult {
  return 'title' in item
}

// Get display name from either type
function getItemDisplayName(item: TvShowSearchResult | MovieSearchResult): string {
  if (isTvShowSearchResult(item)) {
    return item.name
  }
  return item.title
}

export function AddToLibraryModal({
  isOpen,
  onClose,
  release,
  onAdded,
}: AddToLibraryModalProps) {
  // Parsed info from torrent name
  const [parsedInfo, setParsedInfo] = useState<ParsedInfo | null>(null)
  const [selectedType, setSelectedType] = useState<DetectedType>('unknown')
  
  // Libraries
  const [libraries, setLibraries] = useState<Library[]>([])
  const [selectedLibraryId, setSelectedLibraryId] = useState<string>('')
  const [loadingLibraries, setLoadingLibraries] = useState(false)
  
  // Search for existing items
  const [searchQuery, setSearchQuery] = useState('')
  const [searchResults, setSearchResults] = useState<(TvShowSearchResult | MovieSearchResult)[]>([])
  const [searching, setSearching] = useState(false)
  const [selectedItem, setSelectedItem] = useState<TvShowSearchResult | MovieSearchResult | null>(null)
  
  // Options
  const [startDownload, setStartDownload] = useState(true)
  const [creating, setCreating] = useState(false)
  
  // Parse release name when modal opens
  useEffect(() => {
    if (isOpen && release) {
      const parsed = parseTorrentName(release.title)
      setParsedInfo(parsed)
      setSelectedType(parsed.type)
      setSearchQuery(parsed.title)
      setSelectedItem(null)
      setSearchResults([])
    }
  }, [isOpen, release])
  
  // Fetch libraries
  useEffect(() => {
    if (isOpen) {
      setLoadingLibraries(true)
      graphqlClient
        .query<{ libraries: Library[] }>(LIBRARIES_QUERY, {})
        .toPromise()
        .then(({ data }) => {
          setLibraries(data?.libraries || [])
        })
        .finally(() => setLoadingLibraries(false))
    }
  }, [isOpen])
  
  // Filter libraries by selected type
  const filteredLibraries = useMemo(() => {
    const targetType = TYPE_TO_LIBRARY_TYPE[selectedType]
    if (!targetType) return libraries
    return libraries.filter((lib) => lib.libraryType === targetType)
  }, [libraries, selectedType])
  
  // Auto-select first matching library
  useEffect(() => {
    if (filteredLibraries.length > 0 && !selectedLibraryId) {
      setSelectedLibraryId(filteredLibraries[0].id)
    } else if (filteredLibraries.length === 0) {
      setSelectedLibraryId('')
    }
  }, [filteredLibraries, selectedLibraryId])
  
  // Search for matching items
  const handleSearch = async () => {
    if (!searchQuery.trim() || !selectedType) return
    
    setSearching(true)
    setSelectedItem(null)
    
    try {
      if (selectedType === 'tv') {
        const { data } = await graphqlClient
          .query<{ searchTvShows: TvShowSearchResult[] }>(SEARCH_TV_SHOWS_QUERY, { query: searchQuery })
          .toPromise()
        setSearchResults(data?.searchTvShows || [])
      } else if (selectedType === 'movies') {
        const { data } = await graphqlClient
          .query<{ searchMovies: MovieSearchResult[] }>(SEARCH_MOVIES_QUERY, { query: searchQuery })
          .toPromise()
        setSearchResults(data?.searchMovies || [])
      } else {
        // Music and audiobooks - no search yet
        setSearchResults([])
      }
    } catch (err) {
      console.error('Search failed:', err)
    } finally {
      setSearching(false)
    }
  }
  
  // Add item and start download
  const handleConfirm = async () => {
    if (!release || !selectedLibraryId) {
      addToast({
        title: 'Error',
        description: 'Please select a library',
        color: 'danger',
      })
      return
    }
    
    setCreating(true)
    
    try {
      // Create the library item if we have a selected metadata item
      if (selectedItem && (selectedType === 'tv' || selectedType === 'movies')) {
        if (selectedType === 'tv') {
          const tvItem = selectedItem as TvShowSearchResult
          const { data, error } = await graphqlClient
            .mutation<{ addTvShow: { success: boolean; tvShow: { id: string } | null; error: string | null } }>(
              ADD_TV_SHOW_MUTATION,
              {
                libraryId: selectedLibraryId,
                input: {
                  provider: tvItem.provider,
                  providerId: tvItem.providerId,
                  monitorType: 'ALL',
                },
              }
            )
            .toPromise()
          
          if (error || !data?.addTvShow.success) {
            throw new Error(data?.addTvShow.error || 'Failed to add TV show')
          }
          // TODO: Use data.addTvShow.tvShow?.id to link torrent to show
        } else if (selectedType === 'movies') {
          const movieItem = selectedItem as MovieSearchResult
          const { data, error } = await graphqlClient
            .mutation<{ addMovie: { success: boolean; movie: { id: string } | null; error: string | null } }>(
              ADD_MOVIE_MUTATION,
              {
                libraryId: selectedLibraryId,
                input: {
                  provider: movieItem.provider,
                  providerId: String(movieItem.providerId),
                },
              }
            )
            .toPromise()
          
          if (error || !data?.addMovie.success) {
            throw new Error(data?.addMovie.error || 'Failed to add movie')
          }
          // TODO: Use data.addMovie.movie?.id to link torrent to movie
        }
      }
      
      // Start the download
      if (startDownload) {
        // Prefer magnet link, fall back to torrent file URL
        const magnetUri = release.magnetUri
        const torrentUrl = release.link
        
        if (!magnetUri && !torrentUrl) {
          throw new Error('No download link available')
        }
        
        const ADD_TORRENT = `
          mutation AddTorrentToLibrary($input: AddTorrentInput!) {
            addTorrent(input: $input) {
              success
              torrent { id name }
              error
            }
          }
        `
        
        interface AddTorrentResponse {
          addTorrent: {
            success: boolean
            torrent: { id: string; name: string } | null
            error: string | null
          }
        }
        
        // Use magnet field for magnet links, url field for .torrent file URLs
        const isMagnet = magnetUri?.startsWith('magnet:')
        
        const { data, error } = await graphqlClient
          .mutation<AddTorrentResponse>(ADD_TORRENT, {
            input: {
              magnet: isMagnet ? magnetUri : undefined,
              url: !isMagnet ? (magnetUri || torrentUrl) : undefined,
              libraryId: selectedLibraryId || undefined,
              // Pass indexer ID for authenticated .torrent downloads
              indexerId: !isMagnet && release.indexerId ? release.indexerId : undefined,
              // Link to specific item if selected (providerId is the external ID)
              movieId: selectedType === 'movies' && selectedItem && isMovieSearchResult(selectedItem) ? String(selectedItem.providerId) : undefined,
              episodeId: selectedType === 'tv' && selectedItem && isTvShowSearchResult(selectedItem) ? String(selectedItem.providerId) : undefined,
            },
          })
          .toPromise()
        
        if (error || !data?.addTorrent?.success) {
          throw new Error(data?.addTorrent?.error || 'Failed to add torrent')
        }
      }
      
      addToast({
        title: 'Success',
        description: selectedItem
          ? `Added "${getItemDisplayName(selectedItem)}" and started download`
          : 'Download started',
        color: 'success',
      })
      
      onAdded()
      onClose()
    } catch (err) {
      console.error('Failed to add to library:', err)
      addToast({
        title: 'Error',
        description: err instanceof Error ? err.message : 'Failed to add to library',
        color: 'danger',
      })
    } finally {
      setCreating(false)
    }
  }
  
  const TypeIcon = TYPE_ICONS[selectedType]
  
  return (
    <Modal isOpen={isOpen} onClose={onClose} size="2xl" scrollBehavior="inside">
      <ModalContent>
        <ModalHeader className="flex items-center gap-3">
          <IconDownload size={20} />
          Add to Library
        </ModalHeader>
        
        <ModalBody className="gap-4">
          {/* Release Info */}
          {release && (
            <Card className="bg-content2">
              <CardBody className="py-3">
                <p className="text-sm font-medium line-clamp-2">{release.title}</p>
                <div className="flex items-center gap-2 mt-1 text-xs text-default-500">
                  <span>{release.sizeFormatted}</span>
                  <span>•</span>
                  <span className="text-success">{release.seeders} seeders</span>
                  <span>•</span>
                  <span>{release.indexerName}</span>
                </div>
              </CardBody>
            </Card>
          )}
          
          {/* Detected Type */}
          <div className="space-y-2">
            <label className="text-sm font-medium">Detected Type</label>
            <div className="flex items-center gap-2">
              <Select
                selectedKeys={[selectedType]}
                onChange={(e) => setSelectedType(e.target.value as DetectedType)}
                className="flex-1"
                startContent={<TypeIcon size={16} />}
              >
                <SelectItem key="tv" textValue="TV Show">
                  <div className="flex items-center gap-2">
                    <IconDeviceTv size={16} />
                    TV Show
                  </div>
                </SelectItem>
                <SelectItem key="movies" textValue="Movie">
                  <div className="flex items-center gap-2">
                    <IconMovie size={16} />
                    Movie
                  </div>
                </SelectItem>
                <SelectItem key="music" textValue="Music">
                  <div className="flex items-center gap-2">
                    <IconMusic size={16} />
                    Music
                  </div>
                </SelectItem>
                <SelectItem key="audiobooks" textValue="Audiobook">
                  <div className="flex items-center gap-2">
                    <IconHeadphones size={16} />
                    Audiobook
                  </div>
                </SelectItem>
              </Select>
              
              {parsedInfo && (
                <Chip size="sm" variant="flat" color="primary">
                  Detected: {parsedInfo.title}
                  {parsedInfo.year && ` (${parsedInfo.year})`}
                  {parsedInfo.season && ` S${parsedInfo.season}`}
                  {parsedInfo.episode && `E${parsedInfo.episode}`}
                </Chip>
              )}
            </div>
          </div>
          
          {/* Library Selection */}
          <div className="space-y-2">
            <label className="text-sm font-medium">Target Library</label>
            {loadingLibraries ? (
              <Spinner size="sm" />
            ) : filteredLibraries.length === 0 ? (
              <p className="text-sm text-warning">
                No {selectedType} libraries found. Please create one first.
              </p>
            ) : (
              <Select
                selectedKeys={selectedLibraryId ? [selectedLibraryId] : []}
                onChange={(e) => setSelectedLibraryId(e.target.value)}
                placeholder="Select a library"
              >
                {filteredLibraries.map((lib) => (
                  <SelectItem key={lib.id} textValue={lib.name}>
                    {lib.name}
                  </SelectItem>
                ))}
              </Select>
            )}
          </div>
          
          <Divider />
          
          {/* Search for Metadata */}
          {(selectedType === 'tv' || selectedType === 'movies') && (
            <div className="space-y-3">
              <label className="text-sm font-medium">
                Link to {selectedType === 'tv' ? 'TV Show' : 'Movie'} (Optional)
              </label>
              
              <Input
                label={`Search ${selectedType === 'tv' ? 'TV Shows' : 'Movies'}`}
                labelPlacement="inside"
                variant="flat"
                placeholder={`Search for ${selectedType === 'tv' ? 'TV shows' : 'movies'}...`}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                startContent={<IconSearch size={16} className="text-default-400" />}
                className="flex-1"
                classNames={{
                  label: 'text-sm font-medium text-primary!',
                }}
                endContent={
                  <Button
                    size="sm"
                    variant="light"
                    color="primary"
                    className="font-semibold"
                    onPress={handleSearch}
                    isLoading={searching}
                    isDisabled={!searchQuery.trim()}
                  >
                    Search
                  </Button>
                }
              />
              
              {/* Search Results */}
              {searchResults.length > 0 && (
                <div className="max-h-48 overflow-y-auto space-y-2">
                  {searchResults.map((item) => {
                    const isTv = 'name' in item && !('title' in item)
                    const id = isTv ? (item as TvShowSearchResult).providerId : String((item as MovieSearchResult).providerId)
                    const name = isTv ? (item as TvShowSearchResult).name : (item as MovieSearchResult).title
                    const year = isTv ? (item as TvShowSearchResult).year : (item as MovieSearchResult).year
                    const poster = isTv ? (item as TvShowSearchResult).posterUrl : (item as MovieSearchResult).posterUrl
                    const isSelected = selectedItem === item
                    
                    return (
                      <Card
                        key={id}
                        isPressable
                        onPress={() => setSelectedItem(isSelected ? null : item)}
                        className={`${isSelected ? 'ring-2 ring-primary' : ''}`}
                      >
                        <CardBody className="flex-row items-center gap-3 py-2">
                          {poster ? (
                            <Image
                              src={poster}
                              alt={name}
                              className="w-10 h-14 object-cover rounded"
                            />
                          ) : (
                            <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
                              {isTv ? <IconDeviceTv size={20} /> : <IconMovie size={20} />}
                            </div>
                          )}
                          <div className="flex-1">
                            <p className="font-medium text-sm">{name}</p>
                            <p className="text-xs text-default-500">
                              {year || 'Unknown year'}
                            </p>
                          </div>
                          {isSelected && (
                            <IconCheck size={20} className="text-primary" />
                          )}
                        </CardBody>
                      </Card>
                    )
                  })}
                </div>
              )}
              
              {!selectedItem && searchResults.length === 0 && !searching && (
                <p className="text-xs text-default-400">
                  Search to link this download to a {selectedType === 'tv' ? 'show' : 'movie'} in your library.
                  If not linked, the download will still be added to the selected library.
                </p>
              )}
            </div>
          )}
          
          <Divider />
          
          {/* Options */}
          <div className="space-y-3">
            <Switch isSelected={startDownload} onValueChange={setStartDownload}>
              <span className="text-sm">Start download immediately</span>
            </Switch>
          </div>
        </ModalBody>
        
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            onPress={handleConfirm}
            isLoading={creating}
            isDisabled={!selectedLibraryId}
            startContent={<IconDownload size={16} />}
          >
            {selectedItem ? 'Add & Download' : 'Download to Library'}
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
