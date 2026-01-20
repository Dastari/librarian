import { useState, useCallback } from 'react'
import { Button } from '@heroui/button'
import { Input } from '@heroui/input'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Card, CardBody } from '@heroui/card'
import { Image } from '@heroui/image'
import { Spinner } from '@heroui/spinner'
import { Chip } from '@heroui/chip'
import {
  IconSearch,
  IconDisc,
  IconUser,
  IconCalendar,
  IconPlus,
} from '@tabler/icons-react'
import {
  graphqlClient,
  SEARCH_ALBUMS_QUERY,
  ADD_ALBUM_MUTATION,
  type AlbumSearchResult,
  type AlbumResult,
} from '../../lib/graphql'

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
  result: AlbumSearchResult
  onAdd: () => void
  isAdding: boolean
}

function SearchResultCard({ result, onAdd, isAdding }: SearchResultCardProps) {
  return (
    <Card>
      <CardBody className="flex flex-row gap-4 p-3">
        {result.coverUrl ? (
          <Image
            src={result.coverUrl}
            alt={result.title}
            className="w-16 h-16 object-cover flex-shrink-0"
            radius="md"
          />
        ) : (
          <div className="w-16 h-16 bg-default-100 flex items-center justify-center rounded-md flex-shrink-0">
            <IconDisc size={24} className="text-default-400" />
          </div>
        )}
        <div className="flex-1 min-w-0">
          <p className="font-semibold line-clamp-1">{result.title}</p>
          {result.artistName && (
            <p className="text-sm text-default-500 flex items-center gap-1 line-clamp-1">
              <IconUser size={14} />
              {result.artistName}
            </p>
          )}
          <div className="flex items-center gap-2 mt-1">
            {result.year && (
              <Chip size="sm" variant="flat">
                <span className="flex items-center gap-1">
                  <IconCalendar size={12} />
                  {result.year}
                </span>
              </Chip>
            )}
            {result.albumType && (
              <Chip size="sm" variant="flat" color="secondary">
                {result.albumType}
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

export function AddAlbumModal({
  isOpen,
  onClose,
  libraryId,
  onAlbumAdded,
}: AddAlbumModalProps) {
  const [searchQuery, setSearchQuery] = useState('')
  const [searching, setSearching] = useState(false)
  const [searchResults, setSearchResults] = useState<AlbumSearchResult[]>([])
  const [addingId, setAddingId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) return

    setSearching(true)
    setError(null)
    setSearchResults([])

    try {
      const result = await graphqlClient
        .query<{ searchAlbums: AlbumSearchResult[] }>(SEARCH_ALBUMS_QUERY, {
          query: searchQuery,
        })
        .toPromise()

      if (result.error) {
        setError(result.error.message)
      } else if (result.data?.searchAlbums) {
        setSearchResults(result.data.searchAlbums)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed')
    } finally {
      setSearching(false)
    }
  }, [searchQuery])

  const handleAddAlbum = useCallback(
    async (result: AlbumSearchResult) => {
      setAddingId(result.providerId)
      setError(null)

      try {
        const mutationResult = await graphqlClient
          .mutation<{ addAlbum: AlbumResult }>(ADD_ALBUM_MUTATION, {
            input: {
              musicbrainzId: result.providerId,
              libraryId,
            },
          })
          .toPromise()

        if (mutationResult.error) {
          setError(mutationResult.error.message)
        } else if (mutationResult.data?.addAlbum.success) {
          // Remove from search results
          setSearchResults((prev) =>
            prev.filter((r) => r.providerId !== result.providerId)
          )
          onAlbumAdded?.()
        } else if (mutationResult.data?.addAlbum.error) {
          setError(mutationResult.data.addAlbum.error)
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to add album')
      } finally {
        setAddingId(null)
      }
    },
    [libraryId, onAlbumAdded]
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
            className="flex gap-2"
          >
            <Input
              placeholder="Search for album or artist..."
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
                  key={result.providerId}
                  result={result}
                  onAdd={() => handleAddAlbum(result)}
                  isAdding={addingId === result.providerId}
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
