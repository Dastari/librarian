import { useState, useEffect, useMemo } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Spinner } from '@heroui/spinner'
import { addToast } from '@heroui/toast'
import { Select, SelectItem } from '@heroui/select'
import {
  IconDeviceTv,
  IconMovie,
  IconMusic,
  IconHeadphones,
  IconFolder,
  IconCheck,
  IconDisc,
} from '@tabler/icons-react'
import {
  graphqlClient,
  LIBRARIES_QUERY,
  ORGANIZE_TORRENT_MUTATION,
  type Library,
  type Torrent,
  type OrganizeTorrentResult,
  type Album,
} from '../../lib/graphql'

// Query to get albums for a library
const ALBUMS_FOR_LIBRARY_QUERY = `
  query AlbumsForLibrary($libraryId: String!) {
    albums(libraryId: $libraryId) {
      id
      name
      year
      coverUrl
    }
  }
`

export interface LinkToLibraryModalProps {
  isOpen: boolean
  onClose: () => void
  torrent: Torrent | null
  onLinked: () => void
}

// Get icon for library type
function getLibraryIcon(type: string) {
  switch (type) {
    case 'TV':
      return IconDeviceTv
    case 'MOVIES':
      return IconMovie
    case 'MUSIC':
      return IconMusic
    case 'AUDIOBOOKS':
      return IconHeadphones
    default:
      return IconFolder
  }
}

// Get color for library type
function getLibraryColor(type: string) {
  switch (type) {
    case 'TV':
      return 'text-blue-400'
    case 'MOVIES':
      return 'text-purple-400'
    case 'MUSIC':
      return 'text-green-400'
    case 'AUDIOBOOKS':
      return 'text-orange-400'
    default:
      return 'text-default-400'
  }
}

// Detect media type from torrent name
function detectMediaType(name: string): 'TV' | 'MOVIES' | 'MUSIC' | 'AUDIOBOOKS' | null {
  const cleaned = name.replace(/\./g, ' ').replace(/_/g, ' ')
  
  // TV patterns
  const tvPatterns = [
    /[Ss](\d{1,2})[Ee](\d{1,2})/,
    /(\d{1,2})x(\d{2})/,
    /[Ss]eason\s*(\d+)/i,
  ]
  for (const pattern of tvPatterns) {
    if (pattern.test(cleaned)) return 'TV'
  }
  
  // Music patterns
  const musicPatterns = [
    /\b(FLAC|MP3|320kbps|V0|ALAC)\b/i,
    /\b(Discography|Album|EP|Single)\b/i,
  ]
  for (const pattern of musicPatterns) {
    if (pattern.test(cleaned)) return 'MUSIC'
  }
  
  // Audiobook patterns
  const audiobookPatterns = [
    /\b(Audiobook|M4B|Audible)\b/i,
  ]
  for (const pattern of audiobookPatterns) {
    if (pattern.test(cleaned)) return 'AUDIOBOOKS'
  }
  
  // Movie patterns - if it has year and quality but no season/episode
  const moviePattern = /[\s\.\(]*((?:19|20)\d{2})[\s\)\]\.]/
  if (moviePattern.test(cleaned)) return 'MOVIES'
  
  return null
}

export function LinkToLibraryModal({
  isOpen,
  onClose,
  torrent,
  onLinked,
}: LinkToLibraryModalProps) {
  const [libraries, setLibraries] = useState<Library[]>([])
  const [albums, setAlbums] = useState<Album[]>([])
  const [loading, setLoading] = useState(true)
  const [loadingAlbums, setLoadingAlbums] = useState(false)
  const [organizing, setOrganizing] = useState(false)
  const [selectedLibraryId, setSelectedLibraryId] = useState<string | null>(null)
  const [selectedAlbumId, setSelectedAlbumId] = useState<string | null>(null)

  // Get the selected library
  const selectedLibrary = useMemo(() => {
    return libraries.find(l => l.id === selectedLibraryId)
  }, [libraries, selectedLibraryId])

  // Check if selected library is music
  const isMusicLibrary = selectedLibrary?.libraryType === 'MUSIC'

  // Fetch libraries
  useEffect(() => {
    if (isOpen) {
      setLoading(true)
      setSelectedLibraryId(null)
      setSelectedAlbumId(null)
      setAlbums([])
      graphqlClient
        .query<{ libraries: Library[] }>(LIBRARIES_QUERY)
        .toPromise()
        .then((result) => {
          if (result.data?.libraries) {
            setLibraries(result.data.libraries)
          }
        })
        .finally(() => setLoading(false))
    }
  }, [isOpen])

  // Fetch albums when a music library is selected
  useEffect(() => {
    if (selectedLibraryId && isMusicLibrary) {
      setLoadingAlbums(true)
      setSelectedAlbumId(null)
      graphqlClient
        .query<{ albums: Album[] }>(ALBUMS_FOR_LIBRARY_QUERY, { libraryId: selectedLibraryId })
        .toPromise()
        .then((result) => {
          if (result.data?.albums) {
            setAlbums(result.data.albums)
          }
        })
        .finally(() => setLoadingAlbums(false))
    } else {
      setAlbums([])
      setSelectedAlbumId(null)
    }
  }, [selectedLibraryId, isMusicLibrary])

  // Detect recommended library type
  const recommendedType = useMemo(() => {
    if (!torrent) return null
    return detectMediaType(torrent.name)
  }, [torrent])

  // Sort libraries - recommended type first
  const sortedLibraries = useMemo(() => {
    if (!recommendedType) return libraries
    return [...libraries].sort((a, b) => {
      if (a.libraryType === recommendedType && b.libraryType !== recommendedType) return -1
      if (b.libraryType === recommendedType && a.libraryType !== recommendedType) return 1
      return 0
    })
  }, [libraries, recommendedType])

  const handleLink = async () => {
    if (!torrent || !selectedLibraryId) return
    // For music libraries, require album selection
    if (isMusicLibrary && !selectedAlbumId) {
      addToast({
        title: 'Select Album',
        description: 'Please select an album to link this music to',
        color: 'warning',
      })
      return
    }

    setOrganizing(true)
    try {
      const result = await graphqlClient
        .mutation<{ organizeTorrent: OrganizeTorrentResult }>(ORGANIZE_TORRENT_MUTATION, {
          id: torrent.id,
          libraryId: selectedLibraryId,
          albumId: selectedAlbumId,
        })
        .toPromise()

      if (result.data?.organizeTorrent) {
        const { success, messages } = result.data.organizeTorrent

        if (success) {
          addToast({
            title: 'Linked Successfully',
            description: messages[0] || 'Torrent linked to library',
            color: 'success',
          })
          onLinked()
          onClose()
        } else {
          // Show messages even on "failure" - they may be informational
          addToast({
            title: 'Link Result',
            description: messages[0] || 'Linking completed with notes',
            color: messages.length > 0 ? 'warning' : 'success',
          })
          onLinked()
          onClose()
        }
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: 'Failed to link torrent to library',
        color: 'danger',
      })
    } finally {
      setOrganizing(false)
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="lg">
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1">
          <div>Link to Library</div>
          {torrent && (
            <div className="text-sm font-normal text-default-500 line-clamp-1">
              {torrent.name}
            </div>
          )}
        </ModalHeader>
        <ModalBody>
          {loading ? (
            <div className="flex justify-center py-8">
              <Spinner size="lg" />
            </div>
          ) : libraries.length === 0 ? (
            <div className="text-center py-8 text-default-500">
              <IconFolder size={48} className="mx-auto mb-4 text-default-400" />
              <p>No libraries found. Create a library first.</p>
            </div>
          ) : (
            <div className="flex flex-col gap-2">
              <p className="text-sm text-default-500 mb-2">
                Select a library to link this torrent to. The files will be matched and organized
                based on the library settings.
              </p>
              {sortedLibraries.map((library) => {
                const Icon = getLibraryIcon(library.libraryType)
                const isSelected = selectedLibraryId === library.id
                const isRecommended = library.libraryType === recommendedType

                return (
                  <Card
                    key={library.id}
                    isPressable
                    className={`transition-all ${
                      isSelected
                        ? 'ring-2 ring-primary bg-primary/10'
                        : 'hover:bg-content2'
                    }`}
                    onPress={() => setSelectedLibraryId(library.id)}
                  >
                    <CardBody className="flex-row items-center gap-3 py-3">
                      <Icon size={24} className={getLibraryColor(library.libraryType)} />
                      <div className="flex-1">
                        <div className="flex items-center gap-2">
                          <span className="font-medium">{library.name}</span>
                          {isRecommended && (
                            <span className="text-xs px-1.5 py-0.5 rounded bg-primary/20 text-primary">
                              Recommended
                            </span>
                          )}
                        </div>
                        <div className="text-xs text-default-500">
                          {library.libraryType} • {library.path}
                        </div>
                      </div>
                      {isSelected && (
                        <IconCheck size={20} className="text-primary" />
                      )}
                    </CardBody>
                  </Card>
                )
              })}

              {/* Album selection for music libraries */}
              {isMusicLibrary && selectedLibraryId && (
                <div className="mt-4 pt-4 border-t border-default-200">
                  <div className="flex items-center gap-2 mb-2">
                    <IconDisc size={20} className="text-green-400" />
                    <span className="text-sm font-medium">Select Album</span>
                  </div>
                  {loadingAlbums ? (
                    <div className="flex justify-center py-4">
                      <Spinner size="sm" />
                    </div>
                  ) : albums.length === 0 ? (
                    <div className="text-sm text-default-500 py-2">
                      No albums found in this library. Add an album first from the library page.
                    </div>
                  ) : (
                    <Select
                      label="Album"
                      placeholder="Select an album"
                      selectedKeys={selectedAlbumId ? [selectedAlbumId] : []}
                      onSelectionChange={(keys) => {
                        const selected = Array.from(keys)[0] as string
                        setSelectedAlbumId(selected || null)
                      }}
                      classNames={{
                        trigger: 'bg-content2',
                      }}
                    >
                      {albums.map((album) => (
                        <SelectItem key={album.id} textValue={album.name}>
                          <div className="flex flex-col">
                            <span>{album.name}</span>
                            {album.year && (
                              <span className="text-xs text-default-500">
                                {album.year}
                              </span>
                            )}
                          </div>
                        </SelectItem>
                      ))}
                    </Select>
                  )}
                </div>
              )}
            </div>
          )}
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            onPress={handleLink}
            isLoading={organizing}
            isDisabled={!selectedLibraryId || organizing || (isMusicLibrary && !selectedAlbumId)}
          >
            Link & Organize
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
