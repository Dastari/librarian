import { useState } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Image } from '@heroui/image'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Input } from '@heroui/input'
import { Chip } from '@heroui/chip'
import { Switch } from '@heroui/switch'
import { Spinner } from '@heroui/spinner'
import { addToast } from '@heroui/toast'
import {
  graphqlClient,
  SEARCH_MOVIES_QUERY,
  ADD_MOVIE_MUTATION,
  type Movie,
  type MovieSearchResult,
} from '../../lib/graphql'
import { IconMovie, IconStar } from '@tabler/icons-react'
import { sanitizeError } from '../../lib/format'


export interface AddMovieModalProps {
  isOpen: boolean
  onClose: () => void
  libraryId: string
  onAdded: () => void
}

export function AddMovieModal({
  isOpen,
  onClose,
  libraryId,
  onAdded,
}: AddMovieModalProps) {
  const [searchQuery, setSearchQuery] = useState('')
  const [searchResults, setSearchResults] = useState<MovieSearchResult[]>([])
  const [searching, setSearching] = useState(false)
  const [adding, setAdding] = useState(false)
  const [selectedMovie, setSelectedMovie] = useState<MovieSearchResult | null>(null)
  const [monitored, setMonitored] = useState(true)

  const handleSearch = async () => {
    if (!searchQuery.trim()) return

    try {
      setSearching(true)
      const { data, error } = await graphqlClient
        .query<{ searchMovies: MovieSearchResult[] }>(SEARCH_MOVIES_QUERY, {
          query: searchQuery,
        })
        .toPromise()

      if (error) {
        addToast({
          title: 'Error',
          description: sanitizeError(error),
          color: 'danger',
        })
        return
      }

      setSearchResults(data?.searchMovies || [])
    } catch (err) {
      console.error('Search failed:', err)
    } finally {
      setSearching(false)
    }
  }

  const handleAdd = async () => {
    if (!selectedMovie) return

    try {
      setAdding(true)
      const { data, error } = await graphqlClient
        .mutation<{
          addMovie: {
            success: boolean
            movie: Movie | null
            error: string | null
          }
        }>(ADD_MOVIE_MUTATION, {
          libraryId,
          input: {
            tmdbId: selectedMovie.providerId,
            monitored,
          },
        })
        .toPromise()

      if (error || !data?.addMovie.success) {
        addToast({
          title: 'Error',
          description: sanitizeError(data?.addMovie.error || 'Failed to add movie'),
          color: 'danger',
        })
        return
      }

      addToast({
        title: 'Success',
        description: `Added "${selectedMovie.title}" to library`,
        color: 'success',
      })

      handleReset()
      onClose()
      onAdded()
    } catch (err) {
      console.error('Failed to add movie:', err)
    } finally {
      setAdding(false)
    }
  }

  const handleReset = () => {
    setSearchQuery('')
    setSearchResults([])
    setSelectedMovie(null)
    setMonitored(true)
  }

  const handleClose = () => {
    handleReset()
    onClose()
  }

  return (
    <Modal isOpen={isOpen} onClose={handleClose} size="2xl">
      <ModalContent>
        <ModalHeader>Add Movie</ModalHeader>
        <ModalBody>
          {!selectedMovie ? (
            <div className="space-y-4">
              <Input
                label="Search Movies"
                labelPlacement="inside"
                variant="flat"
                placeholder="Search for a movie..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
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
                  >
                    Search
                  </Button>
                }
              />

              {searching ? (
                <div className="flex justify-center py-8">
                  <Spinner size="lg" />
                </div>
              ) : searchResults.length > 0 ? (
                <div className="space-y-2 max-h-96 overflow-auto">
                  {searchResults.map((result) => (
                    <Card
                      key={`${result.provider}-${result.providerId}`}
                      isPressable
                      className="bg-content2 w-full hover:bg-content3"
                      onPress={() => setSelectedMovie(result)}
                    >
                      <CardBody className="flex flex-row gap-3 p-2">
                        <div className="shrink-0 w-10">
                          {result.posterUrl ? (
                            <Image
                              src={result.posterUrl}
                              alt={result.title}
                              classNames={{
                                wrapper: "w-full",
                                img: "w-full aspect-[2/3] object-cover"
                              }}
                              radius="sm"
                            />
                          ) : (
                            <div className="w-full aspect-[2/3] bg-default-200 flex items-center justify-center rounded-sm">
                              <IconMovie size={16} className="text-purple-400" />
                            </div>
                          )}
                        </div>
                        <div className="flex-1 min-w-0">
                          <h4 className="font-medium">
                            {result.title}
                            {result.year && (
                              <span className="text-default-500 ml-1">
                                ({result.year})
                              </span>
                            )}
                          </h4>
                          <p className="text-xs text-default-500 line-clamp-2">
                            {result.overview || 'No description available'}
                          </p>
                        </div>
                        <div className="flex items-center gap-2">
                          {result.voteAverage && result.voteAverage > 0 && (
                            <Chip
                              size="sm"
                              variant="flat"
                              color={result.voteAverage >= 7 ? 'success' : result.voteAverage >= 5 ? 'warning' : 'danger'}
                              startContent={<IconStar size={10} />}
                            >
                              {result.voteAverage.toFixed(1)}
                            </Chip>
                          )}
                        </div>
                      </CardBody>
                    </Card>
                  ))}
                </div>
              ) : searchQuery && !searching ? (
                <p className="text-center text-default-500 py-8">
                  No results found
                </p>
              ) : null}
            </div>
          ) : (
            <div className="space-y-4">
              <Card className="bg-content2">
                <CardBody className="flex flex-row gap-4 p-3">
                  <div className="flex-shrink-0 w-24">
                    {selectedMovie.posterUrl ? (
                      <Image
                        src={selectedMovie.posterUrl}
                        alt={selectedMovie.title}
                        classNames={{
                          wrapper: "w-full",
                          img: "w-full aspect-[2/3] object-cover"
                        }}
                        radius="md"
                      />
                    ) : (
                      <div className="w-full aspect-[2/3] bg-default-200 flex items-center justify-center rounded-md">
                        <IconMovie size={32} className="text-purple-400" />
                      </div>
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <h4 className="font-semibold text-lg">
                      {selectedMovie.title}
                      {selectedMovie.year && (
                        <span className="text-default-500 ml-1">
                          ({selectedMovie.year})
                        </span>
                      )}
                    </h4>
                    {selectedMovie.voteAverage && selectedMovie.voteAverage > 0 && (
                      <div className="flex items-center gap-1 text-sm text-default-500 mt-1">
                        <IconStar size={14} className="text-yellow-400" />
                        <span>{selectedMovie.voteAverage.toFixed(1)}</span>
                      </div>
                    )}
                    {selectedMovie.overview && (
                      <p className="text-sm text-default-400 mt-2 line-clamp-4">
                        {selectedMovie.overview}
                      </p>
                    )}
                  </div>
                </CardBody>
              </Card>

              <div className="flex items-center justify-between p-3 bg-content2 rounded-lg">
                <div>
                  <p className="font-medium">Monitor Movie</p>
                  <p className="text-xs text-default-500">
                    Automatically hunt for and download this movie
                  </p>
                </div>
                <Switch
                  isSelected={monitored}
                  onValueChange={setMonitored}
                />
              </div>

              <p className="text-xs text-default-400">
                Quality settings will be inherited from the library. You can customize them after adding the movie.
              </p>

              <Button
                variant="flat"
                onPress={() => {
                  setSelectedMovie(null)
                  setSearchResults([])
                }}
              >
                Back to Search
              </Button>
            </div>
          )}
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={handleClose}>
            Cancel
          </Button>
          {selectedMovie && (
            <Button color="primary" onPress={handleAdd} isLoading={adding}>
              Add Movie
            </Button>
          )}
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
