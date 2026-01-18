import { createFileRoute, redirect, Link, useNavigate } from '@tanstack/react-router'
import { useState, useEffect, useMemo } from 'react'
import { useQueryState, parseAsString } from 'nuqs'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Input } from '@heroui/input'
import { Image } from '@heroui/image'
import { Chip } from '@heroui/chip'
import { Spinner } from '@heroui/spinner'
import { Tabs, Tab } from '@heroui/tabs'
import {
  IconSearch,
  IconDeviceTv,
  IconMovie,
  IconMusic,
  IconHeadphones,
  IconFolder,
} from '@tabler/icons-react'
import {
  graphqlClient,
  TV_SHOWS_QUERY,
  MOVIES_QUERY,
  type TvShow,
  type Movie,
} from '../lib/graphql'
import { RouteError } from '../components/RouteError'

export const Route = createFileRoute('/search')({
  beforeLoad: ({ context, location }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({
        to: '/',
        search: {
          signin: true,
          redirect: location.href,
        },
      })
    }
  },
  component: LibrarySearchPage,
  errorComponent: RouteError,
})

type MediaType = 'all' | 'shows' | 'movies' | 'music' | 'audiobooks'

interface SearchResult {
  id: string
  type: 'show' | 'movie' | 'album' | 'audiobook'
  title: string
  year?: number
  posterUrl?: string
  status?: string
  libraryId: string
  libraryName?: string
}

function LibrarySearchPage() {
  const navigate = useNavigate()
  const [query, setQuery] = useQueryState('q', parseAsString.withDefault(''))
  const [searchInput, setSearchInput] = useState(query)
  const [mediaType, setMediaType] = useState<MediaType>('all')
  const [isSearching, setIsSearching] = useState(false)
  const [shows, setShows] = useState<TvShow[]>([])
  const [movies, setMovies] = useState<Movie[]>([])

  // Sync search input with query param
  useEffect(() => {
    setSearchInput(query)
  }, [query])

  // Fetch all content on mount
  useEffect(() => {
    fetchAllContent()
  }, [])

  const fetchAllContent = async () => {
    setIsSearching(true)
    try {
      const [showsResult, moviesResult] = await Promise.all([
        graphqlClient.query<{ tvShows: TvShow[] }>(TV_SHOWS_QUERY, {}).toPromise(),
        graphqlClient.query<{ movies: Movie[] }>(MOVIES_QUERY, {}).toPromise(),
      ])

      setShows(showsResult.data?.tvShows || [])
      setMovies(moviesResult.data?.movies || [])
    } catch (err) {
      console.error('Failed to fetch content:', err)
    } finally {
      setIsSearching(false)
    }
  }

  // Filter and convert to search results
  const searchResults = useMemo<SearchResult[]>(() => {
    const queryLower = query.toLowerCase().trim()
    const results: SearchResult[] = []

    // Filter shows
    if (mediaType === 'all' || mediaType === 'shows') {
      for (const show of shows) {
        if (!queryLower || show.name.toLowerCase().includes(queryLower)) {
          results.push({
            id: show.id,
            type: 'show',
            title: show.name,
            year: show.year ?? undefined,
            posterUrl: show.posterUrl ?? undefined,
            status: show.status ?? undefined,
            libraryId: show.libraryId,
          })
        }
      }
    }

    // Filter movies
    if (mediaType === 'all' || mediaType === 'movies') {
      for (const movie of movies) {
        if (!queryLower || movie.title.toLowerCase().includes(queryLower)) {
          results.push({
            id: movie.id,
            type: 'movie',
            title: movie.title,
            year: movie.year ?? undefined,
            posterUrl: movie.posterUrl ?? undefined,
            status: movie.status ?? undefined,
            libraryId: movie.libraryId,
          })
        }
      }
    }

    // Sort by title
    results.sort((a, b) => a.title.localeCompare(b.title))

    return results
  }, [query, mediaType, shows, movies])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      setQuery(searchInput)
    }
  }

  const getResultLink = (result: SearchResult): string => {
    if (result.type === 'show') return `/shows/${result.id}`
    if (result.type === 'movie') return `/movies/${result.id}`
    return `/libraries/${result.libraryId}`
  }

  const showsCount = shows.filter(s => !query || s.name.toLowerCase().includes(query.toLowerCase())).length
  const moviesCount = movies.filter(m => !query || m.title.toLowerCase().includes(query.toLowerCase())).length

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 min-w-0 grow flex flex-col gap-4">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold">Search Library</h1>
        <p className="text-default-500 text-sm">
          Find content in your local libraries
        </p>
      </div>

      {/* Search Bar */}
      <Card>
        <CardBody className="flex flex-row items-center gap-4">
          <Input
            label="Search"
            labelPlacement="inside"
            variant="flat"
            placeholder="Search your library..."
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            onKeyDown={handleKeyDown}
            onBlur={() => setQuery(searchInput)}
            startContent={<IconSearch size={18} className="text-default-400" />}
            className="flex-1"
            classNames={{
              label: 'text-sm font-medium text-primary!',
            }}
            size="lg"
            autoFocus
          />
        </CardBody>
      </Card>

      {/* Type Tabs */}
      <Tabs
        selectedKey={mediaType}
        onSelectionChange={(key) => setMediaType(key as MediaType)}
        variant="underlined"
        classNames={{
          tabList: 'gap-4',
        }}
      >
        <Tab
          key="all"
          title={
            <div className="flex items-center gap-2">
              <IconFolder size={16} />
              <span>All</span>
              <Chip size="sm" variant="flat">{searchResults.length}</Chip>
            </div>
          }
        />
        <Tab
          key="shows"
          title={
            <div className="flex items-center gap-2">
              <IconDeviceTv size={16} className="text-blue-400" />
              <span>TV Shows</span>
              <Chip size="sm" variant="flat">{showsCount}</Chip>
            </div>
          }
        />
        <Tab
          key="movies"
          title={
            <div className="flex items-center gap-2">
              <IconMovie size={16} className="text-purple-400" />
              <span>Movies</span>
              <Chip size="sm" variant="flat">{moviesCount}</Chip>
            </div>
          }
        />
        <Tab
          key="music"
          title={
            <div className="flex items-center gap-2">
              <IconMusic size={16} className="text-green-400" />
              <span>Music</span>
              <Chip size="sm" variant="flat">0</Chip>
            </div>
          }
        />
        <Tab
          key="audiobooks"
          title={
            <div className="flex items-center gap-2">
              <IconHeadphones size={16} className="text-amber-400" />
              <span>Audiobooks</span>
              <Chip size="sm" variant="flat">0</Chip>
            </div>
          }
        />
      </Tabs>

      {/* Results */}
      {isSearching ? (
        <div className="flex justify-center items-center py-12">
          <Spinner size="lg" />
          <span className="ml-3 text-default-500">Loading library...</span>
        </div>
      ) : searchResults.length > 0 ? (
        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
          {searchResults.map((result) => (
            <Link key={`${result.type}-${result.id}`} to={getResultLink(result)}>
              <Card isPressable className="h-full">
                <CardBody className="p-0 overflow-hidden">
                  {result.posterUrl ? (
                    <Image
                      src={result.posterUrl}
                      alt={result.title}
                      className="w-full aspect-[2/3] object-cover"
                      removeWrapper
                    />
                  ) : (
                    <div className="w-full aspect-[2/3] bg-default-200 flex items-center justify-center">
                      {result.type === 'show' ? (
                        <IconDeviceTv size={48} className="text-blue-400" />
                      ) : (
                        <IconMovie size={48} className="text-purple-400" />
                      )}
                    </div>
                  )}
                </CardBody>
                <div className="p-2">
                  <p className="font-medium text-sm line-clamp-1" title={result.title}>
                    {result.title}
                  </p>
                  <div className="flex items-center gap-1 text-xs text-default-500">
                    {result.year && <span>{result.year}</span>}
                    {result.status && (
                      <>
                        <span>•</span>
                        <span className="capitalize">{result.status.toLowerCase()}</span>
                      </>
                    )}
                  </div>
                </div>
              </Card>
            </Link>
          ))}
        </div>
      ) : query ? (
        <Card className="bg-content1/50 border-default-300 border-dashed border-2">
          <CardBody className="py-12 text-center">
            <IconSearch size={48} className="mx-auto mb-4 text-default-400" />
            <h3 className="text-lg font-semibold mb-2">No results found</h3>
            <p className="text-default-500 mb-4">
              No content in your library matches "{query}"
            </p>
            <Button
              color="primary"
              onPress={() => navigate({
                to: '/hunt',
                search: { q: query, type: 'all' },
              })}
            >
              Hunt for "{query}" online
            </Button>
          </CardBody>
        </Card>
      ) : (
        <Card className="bg-content1/50 border-default-300 border-dashed border-2">
          <CardBody className="py-12 text-center">
            <IconSearch size={48} className="mx-auto mb-4 text-primary-400" />
            <h3 className="text-lg font-semibold mb-2">Search your library</h3>
            <p className="text-default-500">
              Type above to find TV shows, movies, music, and audiobooks in your library.
            </p>
          </CardBody>
        </Card>
      )}
    </div>
  )
}
