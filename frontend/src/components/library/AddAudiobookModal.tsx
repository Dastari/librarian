import { useState, useCallback } from 'react'
import { Button } from '@heroui/button'
import { Input } from '@heroui/input'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Card, CardBody } from '@heroui/card'
import { Image } from '@heroui/image'
import { Spinner } from '@heroui/spinner'
import {
  IconSearch,
  IconBook,
  IconUser,
  IconCalendar,
  IconPlus,
} from '@tabler/icons-react'
import {
  useLazyQuery,
  useMutation,
} from '../../lib/graphql/client'
import {
  AddAudiobookDocument,
  SearchAudiobooksDocument,
  type SearchAudiobooksQuery,
} from '../../lib/graphql/generated/graphql'

// ============================================================================
// Component Props
// ============================================================================

interface AddAudiobookModalProps {
  isOpen: boolean
  onClose: () => void
  libraryId: string
  onAudiobookAdded?: () => void
}

// ============================================================================
// Search Result Card
// ============================================================================

interface SearchResultCardProps {
  result: SearchAudiobooksQuery['SearchAudiobooks'][number]
  onAdd: () => void
  isAdding: boolean
}

function SearchResultCard({ result, onAdd, isAdding }: SearchResultCardProps) {
  return (
    <Card>
      <CardBody className="flex flex-row gap-4 p-3">
        {result.CoverUrl ? (
          <Image
            src={result.CoverUrl}
            alt={result.Title}
            className="w-16 h-24 object-cover flex-shrink-0"
            radius="md"
          />
        ) : (
          <div className="w-16 h-24 bg-default-100 flex items-center justify-center rounded-md flex-shrink-0">
            <IconBook size={24} className="text-default-400" />
          </div>
        )}
        <div className="flex-1 min-w-0">
          <p className="font-semibold line-clamp-2">{result.Title}</p>
          {result.AuthorName && (
            <p className="text-sm text-default-500 flex items-center gap-1 line-clamp-1">
              <IconUser size={14} />
              {result.AuthorName}
            </p>
          )}
          {result.Year && (
            <p className="text-xs text-default-400 flex items-center gap-1 mt-1">
              <IconCalendar size={12} />
              {result.Year}
            </p>
          )}
          {result.Description && (
            <p className="text-xs text-default-400 mt-1 line-clamp-2">
              {result.Description}
            </p>
          )}
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

export function AddAudiobookModal({
  isOpen,
  onClose,
  libraryId,
  onAudiobookAdded,
}: AddAudiobookModalProps) {
  const [searchQuery, setSearchQuery] = useState('')
  const [searchResults, setSearchResults] = useState<SearchAudiobooksQuery['SearchAudiobooks']>([])
  const [addingId, setAddingId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [searchAudiobooks, { loading: searching }] = useLazyQuery(SearchAudiobooksDocument)
  const [addAudiobook] = useMutation(AddAudiobookDocument)

  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) return

    setError(null)
    setSearchResults([])

    try {
      const { data, error: queryError } = await searchAudiobooks({
        variables: {
          Query: searchQuery,
        },
      })

      if (queryError) {
        setError(queryError.message)
      } else if (data?.SearchAudiobooks) {
        setSearchResults(data.SearchAudiobooks)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed')
    }
  }, [searchAudiobooks, searchQuery])

  const handleAddAudiobook = useCallback(
    async (result: SearchAudiobooksQuery['SearchAudiobooks'][number]) => {
      setAddingId(result.ProviderId)
      setError(null)

      try {
        const { data, error } = await addAudiobook({
          variables: {
            Input: {
              OpenlibraryId: result.ProviderId,
              LibraryId: libraryId,
            },
          },
        })

        if (error) {
          setError(error.message)
        } else if (data?.AddAudiobook.Success) {
          // Remove from search results
          setSearchResults((prev) =>
            prev.filter((r) => r.ProviderId !== result.ProviderId)
          )
          onAudiobookAdded?.()
        } else if (data?.AddAudiobook.Error) {
          setError(data.AddAudiobook.Error)
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to add audiobook')
      } finally {
        setAddingId(null)
      }
    },
    [addAudiobook, libraryId, onAudiobookAdded]
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
          <span>Add Audiobook</span>
          <span className="text-sm font-normal text-default-500">
            Search OpenLibrary for audiobooks to add to your library
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
              placeholder="Search for book title or author..."
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
                  key={result.ProviderId}
                  result={result}
                  onAdd={() => handleAddAudiobook(result)}
                  isAdding={addingId === result.ProviderId}
                />
              ))}
            </div>
          )}

          {/* Empty state */}
          {!searching && searchQuery && searchResults.length === 0 && (
            <div className="text-center py-8 text-default-500">
              <IconBook size={48} className="mx-auto mb-4 text-default-300" />
              <p>No audiobooks found for "{searchQuery}"</p>
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
