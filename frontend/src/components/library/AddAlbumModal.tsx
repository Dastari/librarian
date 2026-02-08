import { useState, useCallback } from 'react'
import { Button } from '@heroui/button'
import { Input } from '@heroui/input'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Card, CardBody } from '@heroui/card'
import { Image } from '@heroui/image'
import { Spinner } from '@heroui/spinner'
import { Chip } from '@heroui/chip'
import { Checkbox } from '@heroui/checkbox'
import {
  IconSearch,
  IconDisc,
  IconUser,
  IconCalendar,
  IconPlus,
} from '@tabler/icons-react'
import {
  useLazyQuery,
  useMutation,
} from '../../lib/graphql/client'
import {
  AddAlbumDocument,
  SearchAlbumsDocument,
  type SearchAlbumsQuery,
} from '../../lib/graphql/generated/graphql'

// ============================================================================
// Component Props
// ============================================================================

interface AddAlbumModalProps {
  isOpen: boolean
  onClose: () => void
  libraryId: string
  onAlbumAdded?: () => void
}

// ============================================================================
// Search Result Card
// ============================================================================

interface SearchResultCardProps {
  result: SearchAlbumsQuery['SearchAlbums'][number]
  onAdd: () => void
  isAdding: boolean
}

function SearchResultCard({ result, onAdd, isAdding }: SearchResultCardProps) {
  return (
    <Card className="bg-content2">
      <CardBody className="flex flex-row gap-4 p-3">
        {result.CoverUrl ? (
          <Image
            src={result.CoverUrl}
            alt={result.Title}
            className="w-16 h-16 object-cover flex-shrink-0"
            radius="md"
          />
        ) : (
          <div className="w-16 h-16 bg-default-100 flex items-center justify-center rounded-md flex-shrink-0">
            <IconDisc size={24} className="text-default-400" />
          </div>
        )}
        <div className="flex-1 min-w-0">
          <p className="font-semibold line-clamp-1">{result.Title}</p>
          {result.ArtistName && (
            <p className="text-sm text-default-500 flex items-center gap-1 line-clamp-1">
              <IconUser size={14} />
              {result.ArtistName}
            </p>
          )}
          <div className="flex items-center gap-2 mt-1">
            {result.Year && (
              <Chip size="sm" variant="flat">
                <span className="flex items-center gap-1">
                  <IconCalendar size={12} />
                  {result.Year}
                </span>
              </Chip>
            )}
            {result.AlbumType && (
              <Chip size="sm" variant="flat" color="secondary">
                {result.AlbumType}
              </Chip>
            )}
          </div>
        </div>
        <Button
          size="sm"
          color="primary"
          isIconOnly
          onPress={onAdd}
          isLoading={isAdding}
        >
          <IconPlus size={16} />
        </Button>
      </CardBody>
    </Card>
  )
}

// ============================================================================
// Main Component
// ============================================================================

// Filter options for album type
interface AlbumTypeFilters {
  includeEps: boolean
  includeSingles: boolean
  includeCompilations: boolean
  includeLive: boolean
  includeSoundtracks: boolean
}

export function AddAlbumModal({
  isOpen,
  onClose,
  libraryId,
  onAlbumAdded,
}: AddAlbumModalProps) {
  const [searchQuery, setSearchQuery] = useState('')
  const [searchResults, setSearchResults] = useState<SearchAlbumsQuery['SearchAlbums']>([])
  const [addingId, setAddingId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [_showFilters, _setShowFilters] = useState(false) // TODO: implement filter UI
  const [filters, setFilters] = useState<AlbumTypeFilters>({
    includeEps: false,
    includeSingles: false,
    includeCompilations: false,
    includeLive: false,
    includeSoundtracks: false,
  })
  const [searchAlbums, { loading: searching }] = useLazyQuery(SearchAlbumsDocument)
  const [addAlbum] = useMutation(AddAlbumDocument)

  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) return

    setError(null)
    setSearchResults([])

    try {
      const { data, error: queryError } = await searchAlbums({
        variables: {
          Query: searchQuery,
          IncludeEps: filters.includeEps,
          IncludeSingles: filters.includeSingles,
          IncludeCompilations: filters.includeCompilations,
          IncludeLive: filters.includeLive,
          IncludeSoundtracks: filters.includeSoundtracks,
        },
      })

      if (queryError) {
        setError(queryError.message)
      } else if (data?.SearchAlbums) {
        setSearchResults(data.SearchAlbums)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed')
    }
  }, [searchQuery, filters])

  const handleAddAlbum = useCallback(
    async (result: SearchAlbumsQuery['SearchAlbums'][number]) => {
      setAddingId(result.ProviderId)
      setError(null)

      try {
        const { data, error } = await addAlbum({
          variables: {
            Input: {
              MusicbrainzId: result.ProviderId,
              LibraryId: libraryId,
            },
          },
        })

        if (error) {
          setError(error.message)
        } else if (data?.AddAlbum.Success) {
          // Remove from search results
          setSearchResults((prev) =>
            prev.filter((r) => r.ProviderId !== result.ProviderId)
          )
          onAlbumAdded?.()
        } else if (data?.AddAlbum.Error) {
          setError(data.AddAlbum.Error)
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to add album')
      } finally {
        setAddingId(null)
      }
    },
    [addAlbum, libraryId, onAlbumAdded]
  )

  const handleClose = useCallback(() => {
    setSearchQuery('')
    setSearchResults([])
    setError(null)
    onClose()
  }, [onClose])

  return (
    <Modal isOpen={isOpen} onClose={handleClose} size="2xl" scrollBehavior="inside">
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1">
          <span>Add Album</span>
          <span className="text-sm font-normal text-default-500">
            Search MusicBrainz for albums to add to your library
          </span>
        </ModalHeader>
        <ModalBody>
          {/* Search input */}
          <form
            onSubmit={(e) => {
              e.preventDefault()
              handleSearch()
            }}
            className="flex flex-col gap-3"
          >
            <div className="flex gap-2">
              <Input
                placeholder="Album name, or 'artist: Artist - Album'"
                value={searchQuery}
                onValueChange={setSearchQuery}
                startContent={<IconSearch size={18} className="text-default-400" />}
                classNames={{
                  inputWrapper: 'flex-1',
                }}
              />
              <Button
                color="primary"
                type="submit"
                isLoading={searching}
                isDisabled={!searchQuery.trim()}
              >
                Search
              </Button>
            </div>

            {/* Type filters */}

              <div className="flex flex-wrap gap-4 p-3 bg-content1 rounded-lg">
                <span className="text-sm text-default-600 w-full">Include release types:</span>
                <Checkbox
                  size="sm"
                  isSelected={filters.includeEps}
                  onValueChange={(checked) =>
                    setFilters((f) => ({ ...f, includeEps: checked }))
                  }
                >
                  EPs
                </Checkbox>
                <Checkbox
                  size="sm"
                  isSelected={filters.includeSingles}
                  onValueChange={(checked) =>
                    setFilters((f) => ({ ...f, includeSingles: checked }))
                  }
                >
                  Singles
                </Checkbox>
                <Checkbox
                  size="sm"
                  isSelected={filters.includeCompilations}
                  onValueChange={(checked) =>
                    setFilters((f) => ({ ...f, includeCompilations: checked }))
                  }
                >
                  Compilations
                </Checkbox>
                <Checkbox
                  size="sm"
                  isSelected={filters.includeLive}
                  onValueChange={(checked) =>
                    setFilters((f) => ({ ...f, includeLive: checked }))
                  }
                >
                  Live
                </Checkbox>
                <Checkbox
                  size="sm"
                  isSelected={filters.includeSoundtracks}
                  onValueChange={(checked) =>
                    setFilters((f) => ({ ...f, includeSoundtracks: checked }))
                  }
                >
                  Soundtracks
                </Checkbox>
              </div>
   
          </form>

          {/* Error message */}
          {error && (
            <div className="p-3 rounded-lg bg-danger-50 text-danger-600 text-sm">
              {error}
            </div>
          )}

          {/* Loading state */}
          {searching && (
            <div className="flex items-center justify-center py-8">
              <Spinner size="lg" />
            </div>
          )}

          {/* Search results */}
          {!searching && searchResults.length > 0 && (
            <div className="space-y-2">
              <p className="text-sm text-default-500">
                Found {searchResults.length} results
              </p>
              {searchResults.map((result) => (
                <SearchResultCard
                  key={result.ProviderId}
                  result={result}
                  onAdd={() => handleAddAlbum(result)}
                  isAdding={addingId === result.ProviderId}
                />
              ))}
            </div>
          )}

          {/* Empty state */}
          {!searching && searchQuery && searchResults.length === 0 && (
            <div className="text-center py-8 text-default-500">
              <IconDisc size={48} className="mx-auto mb-4 text-default-300" />
              <p>No albums found for "{searchQuery}"</p>
              <p className="text-sm mt-1">Try a different search term</p>
            </div>
          )}
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={handleClose}>
            Close
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
