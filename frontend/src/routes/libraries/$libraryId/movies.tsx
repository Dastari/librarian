import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useMemo } from 'react'
import { Spinner } from '@heroui/spinner'
import { Card, CardBody } from '@heroui/card'
import { Button, ButtonGroup } from '@heroui/button'
import { Image } from '@heroui/image'
import { Chip } from '@heroui/chip'
import { Input } from '@heroui/input'
import { useDisclosure } from '@heroui/modal'
import { Link } from '@tanstack/react-router'
import { IconMovie, IconPlus, IconSearch, IconCalendar, IconClock } from '@tabler/icons-react'
import { useLibraryContext } from '../$libraryId'
import {
  graphqlClient,
  MOVIES_QUERY,
  type Movie,
} from '../../../lib/graphql'

export const Route = createFileRoute('/libraries/$libraryId/movies')({
  component: MoviesPage,
})

// Alphabet for A-Z navigation
const ALPHABET = '#ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('')

function getFirstLetter(title: string): string {
  // Skip common articles for sorting
  const titleLower = title.toLowerCase()
  let sortTitle = title
  for (const article of ['the ', 'a ', 'an ']) {
    if (titleLower.startsWith(article)) {
      sortTitle = title.slice(article.length)
      break
    }
  }
  const firstChar = sortTitle.charAt(0).toUpperCase()
  return /[A-Z]/.test(firstChar) ? firstChar : '#'
}

function MoviesPage() {
  const ctx = useLibraryContext()
  const { onOpen: onAddOpen } = useDisclosure()
  const [movies, setMovies] = useState<Movie[]>([])
  const [loading, setLoading] = useState(true)
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedLetter, setSelectedLetter] = useState<string | null>(null)

  const fetchMovies = async () => {
    if (!ctx?.library) return
    
    try {
      setLoading(true)
      const result = await graphqlClient
        .query<{ movies: Movie[] }>(MOVIES_QUERY, { libraryId: ctx.library.id })
        .toPromise()
      
      if (result.data?.movies) {
        setMovies(result.data.movies)
      }
    } catch (err) {
      console.error('Failed to fetch movies:', err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    if (ctx?.library) {
      fetchMovies()
    }
  }, [ctx?.library?.id])

  // Get letters that have movies
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    movies.forEach((movie) => {
      letters.add(getFirstLetter(movie.title))
    })
    return letters
  }, [movies])

  // Handle letter click - toggle filter
  const handleLetterClick = (letter: string) => {
    setSelectedLetter((prev) => (prev === letter ? null : letter))
  }

  // Filter movies by search query and selected letter
  const filteredMovies = useMemo(() => {
    let result = movies

    // Filter by letter first
    if (selectedLetter) {
      result = result.filter((m) => getFirstLetter(m.title) === selectedLetter)
    }

    // Then filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase()
      result = result.filter(
        (m) =>
          m.title.toLowerCase().includes(query) ||
          m.director?.toLowerCase().includes(query) ||
          m.genres.some((g) => g.toLowerCase().includes(query))
      )
    }

    return result
  }, [movies, searchQuery, selectedLetter])

  if (!ctx) {
    return (
      <div className="flex justify-center items-center py-12">
        <Spinner size="lg" />
      </div>
    )
  }

  if (loading) {
    return (
      <div className="flex justify-center items-center py-12">
        <Spinner size="lg" />
      </div>
    )
  }

  return (
    <div className="flex flex-col w-full h-full overflow-hidden">
      {/* A-Z Navigation */}
      <div className="flex items-center p-2 bg-content2 rounded-lg overflow-x-auto shrink-0 mb-4">
        <ButtonGroup size="sm" variant="flat">
          <Button
            variant={selectedLetter === null ? 'solid' : 'flat'}
            color={selectedLetter === null ? 'primary' : 'default'}
            onPress={() => setSelectedLetter(null)}
            className="min-w-8 px-2"
          >
            All
          </Button>
          {ALPHABET.map((letter) => {
            const hasMovies = availableLetters.has(letter)
            const isSelected = selectedLetter === letter
            return (
              <Button
                key={letter}
                variant={isSelected ? 'solid' : 'flat'}
                color={isSelected ? 'primary' : 'default'}
                onPress={() => hasMovies && handleLetterClick(letter)}
                isDisabled={!hasMovies}
                className="w-4 min-w-4 lg:w-6 lg:min-w-6 p-0 text-xs font-medium xl:min-w-7 xl:w-7"
              >
                {letter}
              </Button>
            )
          })}
        </ButtonGroup>
      </div>

      {/* Toolbar */}
      <div className="flex items-center justify-between gap-4 mb-4 shrink-0">
        <div className="flex items-center gap-3">
          <Input
            label="Search"
            labelPlacement="inside"
            variant="flat"
            placeholder="Search movies..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            startContent={<IconSearch size={18} className="text-default-400" />}
            className="max-w-xs"
            size="sm"
            classNames={{
              label: 'text-sm font-medium text-primary!',
            }}
          />
          {filteredMovies.length > 0 && (
            <span className="text-sm text-default-500">
              {filteredMovies.length} {filteredMovies.length === 1 ? 'movie' : 'movies'}
            </span>
          )}
        </div>
        <Button
          color="primary"
          startContent={<IconPlus size={16} />}
          onPress={onAddOpen}
        >
          Add Movie
        </Button>
      </div>

      {/* Movies Grid */}
      {filteredMovies.length === 0 ? (
        <Card className="bg-content1/50 border-default-300 border-dashed border-2">
          <CardBody className="py-12 text-center">
            <IconMovie size={48} className="mx-auto mb-4 text-purple-400" />
            <h3 className="text-lg font-semibold mb-2">No movies yet</h3>
            <p className="text-default-500 mb-4">
              Add movies to this library to start building your collection.
            </p>
            <Button color="primary" onPress={onAddOpen}>
              Add Movie
            </Button>
          </CardBody>
        </Card>
      ) : (
        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4 overflow-auto pb-4">
          {filteredMovies.map((movie) => (
            <MovieCard key={movie.id} movie={movie} />
          ))}
        </div>
      )}

      {/* TODO: Add Movie Modal - similar to AddShowModal */}
    </div>
  )
}

interface MovieCardProps {
  movie: Movie
}

function MovieCard({ movie }: MovieCardProps) {
  return (
    <Link
      to="/movies/$movieId"
      params={{ movieId: movie.id }}
      className="block group"
    >
      <Card
        isPressable
        className="bg-content1 overflow-hidden hover:scale-[1.02] transition-transform"
      >
        {/* Poster */}
        <div className="relative aspect-[2/3]">
          {movie.posterUrl ? (
            <Image
              src={movie.posterUrl}
              alt={movie.title}
              className="w-full h-full object-cover"
              removeWrapper
            />
          ) : (
            <div className="w-full h-full bg-default-200 flex items-center justify-center">
              <IconMovie size={48} className="text-purple-400" />
            </div>
          )}
          
          {/* Status overlay */}
          <div className="absolute top-2 right-2 flex flex-col gap-1">
            {movie.hasFile && (
              <Chip size="sm" color="success" variant="solid" className="text-xs">
                Downloaded
              </Chip>
            )}
            {movie.certification && (
              <Chip size="sm" variant="flat" className="text-xs bg-black/60">
                {movie.certification}
              </Chip>
            )}
          </div>

          {/* Rating badge */}
          {movie.tmdbRating && movie.tmdbRating > 0 && (
            <div className="absolute bottom-2 left-2">
              <Chip
                size="sm"
                variant="solid"
                className={`text-xs font-semibold ${
                  movie.tmdbRating >= 7
                    ? 'bg-success'
                    : movie.tmdbRating >= 5
                    ? 'bg-warning'
                    : 'bg-danger'
                }`}
              >
                {movie.tmdbRating.toFixed(1)}
              </Chip>
            </div>
          )}
        </div>

        {/* Info */}
        <CardBody className="p-3">
          <h4 className="font-semibold text-sm line-clamp-2 mb-1 group-hover:text-primary">
            {movie.title}
          </h4>
          <div className="flex items-center gap-2 text-xs text-default-500">
            {movie.year && (
              <span className="flex items-center gap-1">
                <IconCalendar size={12} />
                {movie.year}
              </span>
            )}
            {movie.runtime && (
              <span className="flex items-center gap-1">
                <IconClock size={12} />
                {movie.runtime}m
              </span>
            )}
          </div>
          {movie.genres.length > 0 && (
            <p className="text-xs text-default-400 mt-1 line-clamp-1">
              {movie.genres.slice(0, 2).join(' • ')}
            </p>
          )}
        </CardBody>
      </Card>
    </Link>
  )
}
