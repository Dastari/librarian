import { useState, useEffect, useMemo, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { Modal, ModalContent, ModalHeader, ModalBody } from '@heroui/modal'
import { Input } from '@heroui/input'
import { Chip } from '@heroui/chip'
import { Spinner } from '@heroui/spinner'
import { Kbd } from '@heroui/kbd'
import { Image } from '@heroui/image'
import { ScrollShadow } from '@heroui/scroll-shadow'
import {
  IconSearch,
  IconDeviceTv,
  IconMovie,
} from '@tabler/icons-react'
import {
  graphqlClient,
  ALL_TV_SHOWS_QUERY,
  ALL_MOVIES_QUERY,
  type TvShow,
  type Movie,
} from '../lib/graphql'

export interface SearchModalProps {
  isOpen: boolean
  onClose: () => void
}

interface SearchResult {
  id: string
  type: 'show' | 'movie'
  title: string
  year: number | null
  posterUrl: string | null
  status: string | null
  libraryId: string
}

export function SearchModal({ isOpen, onClose }: SearchModalProps) {
  const navigate = useNavigate()
  const [searchQuery, setSearchQuery] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [shows, setShows] = useState<TvShow[]>([])
  const [movies, setMovies] = useState<Movie[]>([])

  // Fetch all content when modal opens
  useEffect(() => {
    if (isOpen && shows.length === 0 && movies.length === 0) {
      fetchAllContent()
    }
  }, [isOpen])

  // Reset search when modal closes
  useEffect(() => {
    if (!isOpen) {
      setSearchQuery('')
    }
  }, [isOpen])

  const fetchAllContent = async () => {
    setIsLoading(true)
    try {
      const [showsResult, moviesResult] = await Promise.all([
        graphqlClient.query<{ allTvShows: TvShow[] }>(ALL_TV_SHOWS_QUERY, {}).toPromise(),
        graphqlClient.query<{ allMovies: Movie[] }>(ALL_MOVIES_QUERY, {}).toPromise(),
      ])

      setShows(showsResult.data?.allTvShows || [])
      setMovies(moviesResult.data?.allMovies || [])
    } catch (err) {
      console.error('Failed to fetch content:', err)
    } finally {
      setIsLoading(false)
    }
  }

  // Convert to search results and filter
  const searchResults = useMemo<SearchResult[]>(() => {
    const queryLower = searchQuery.toLowerCase().trim()
    const results: SearchResult[] = []

    // Filter shows
    for (const show of shows) {
      if (!queryLower || show.name.toLowerCase().includes(queryLower)) {
        results.push({
          id: show.id,
          type: 'show',
          title: show.name,
          year: show.year,
          posterUrl: show.posterUrl,
          status: show.status,
          libraryId: show.libraryId,
        })
      }
    }

    // Filter movies
    for (const movie of movies) {
      if (!queryLower || movie.title.toLowerCase().includes(queryLower)) {
        results.push({
          id: movie.id,
          type: 'movie',
          title: movie.title,
          year: movie.year,
          posterUrl: movie.posterUrl,
          status: movie.status,
          libraryId: movie.libraryId,
        })
      }
    }

    // Sort by title
    results.sort((a, b) => a.title.localeCompare(b.title))

    return results
  }, [searchQuery, shows, movies])

  // Navigate to item on click
  const handleItemClick = useCallback(
    (item: SearchResult) => {
      onClose()
      if (item.type === 'show') {
        navigate({ to: '/shows/$showId', params: { showId: item.id } })
      } else if (item.type === 'movie') {
        navigate({ to: '/movies/$movieId', params: { movieId: item.id } })
      }
    },
    [navigate, onClose]
  )

  // Navigate to hunt page
  const handleHuntClick = () => {
    onClose()
    navigate({
      to: '/hunt',
      search: { q: searchQuery, type: 'all' },
    })
  }

  const totalCount = shows.length + movies.length

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      size="2xl"
      scrollBehavior="inside"
      classNames={{
        base: 'max-h-[80vh]',
      }}
    >
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1 pb-0">
          <Input
            label="Search"
            labelPlacement="inside"
            variant="flat"
            placeholder="Search your library..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            startContent={<IconSearch size={18} className="text-default-400" />}
            endContent={
              <Kbd keys={['escape']} className="hidden sm:inline-flex">
                ESC
              </Kbd>
            }
            size="lg"
            autoFocus
            classNames={{
              inputWrapper: 'bg-default-100',
              label: 'text-sm font-medium text-primary!',
            }}
          />
          <div className="flex items-center gap-2 text-xs text-default-500 mt-2">
            <span>
              {searchQuery
                ? `${searchResults.length} result${searchResults.length !== 1 ? 's' : ''}`
                : `${totalCount} items in library`}
            </span>
            {searchQuery && searchResults.length === 0 && (
              <span
                className="text-primary cursor-pointer hover:underline"
                onClick={handleHuntClick}
              >
                Hunt for "{searchQuery}" online →
              </span>
            )}
          </div>
        </ModalHeader>

        <ModalBody className="px-2 pb-4">
          {isLoading ? (
            <div className="flex justify-center items-center py-12">
              <Spinner size="lg" />
              <span className="ml-3 text-default-500">Loading library...</span>
            </div>
          ) : searchResults.length > 0 ? (
            <ScrollShadow className="max-h-[50vh]">
              <div className="space-y-1">
                {searchResults.slice(0, 50).map((result) => (
                  <div
                    key={`${result.type}-${result.id}`}
                    className="flex items-center gap-3 p-2 rounded-lg hover:bg-content2 cursor-pointer transition-colors"
                    onClick={() => handleItemClick(result)}
                  >
                    <div className="w-10 h-14 rounded overflow-hidden bg-default-200 flex items-center justify-center shrink-0">
                      {result.posterUrl ? (
                        <Image
                          src={result.posterUrl}
                          alt={result.title}
                          className="w-full h-full object-cover"
                          removeWrapper
                        />
                      ) : result.type === 'show' ? (
                        <IconDeviceTv size={20} className="text-blue-400" />
                      ) : (
                        <IconMovie size={20} className="text-purple-400" />
                      )}
                    </div>
                    <div className="flex flex-col min-w-0 flex-1">
                      <span className="font-medium truncate">{result.title}</span>
                      <div className="flex items-center gap-2 text-xs text-default-500">
                        <Chip
                          size="sm"
                          variant="flat"
                          color={result.type === 'show' ? 'primary' : 'secondary'}
                          className="h-5"
                        >
                          {result.type === 'show' ? 'TV Show' : 'Movie'}
                        </Chip>
                        {result.year && <span>{result.year}</span>}
                        {result.status && (
                          <span className="capitalize">{result.status.toLowerCase()}</span>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
                {searchResults.length > 50 && (
                  <div className="text-center py-2 text-sm text-default-500">
                    Showing 50 of {searchResults.length} results
                  </div>
                )}
              </div>
            </ScrollShadow>
          ) : searchQuery ? (
            <div className="text-center py-12">
              <IconSearch size={48} className="mx-auto mb-4 text-default-400" />
              <h3 className="text-lg font-semibold mb-2">No results found</h3>
              <p className="text-default-500 mb-4">
                No content in your library matches "{searchQuery}"
              </p>
              <span
                className="text-primary cursor-pointer hover:underline"
                onClick={handleHuntClick}
              >
                Hunt for "{searchQuery}" online →
              </span>
            </div>
          ) : (
            <div className="text-center py-12">
              <IconSearch size={48} className="mx-auto mb-4 text-default-400" />
              <p className="text-default-500">
                Start typing to search your library
              </p>
            </div>
          )}
        </ModalBody>
      </ModalContent>
    </Modal>
  )
}
