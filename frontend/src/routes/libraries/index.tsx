import { createFileRoute, redirect } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useRef } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { useDisclosure } from '@heroui/modal'
import { Skeleton } from '@heroui/skeleton'
import { Tooltip } from '@heroui/tooltip'
import { addToast } from '@heroui/toast'
import { ConfirmModal } from '../../components/ConfirmModal'
import { AddLibraryModal, LibraryGridCard } from '../../components/library'
import { IconPlus } from '@tabler/icons-react'
import { RouteError } from '../../components/RouteError'
import { sanitizeError } from '../../lib/format'
import {
  graphqlClient,
  LIBRARIES_QUERY,
  TV_SHOWS_QUERY,
  MOVIES_QUERY,
  ALBUMS_QUERY,
  AUDIOBOOKS_QUERY,
  CREATE_LIBRARY_MUTATION,
  SCAN_LIBRARY_MUTATION,
  DELETE_LIBRARY_MUTATION,
  LIBRARY_CHANGED_SUBSCRIPTION,
  type Library,
  type TvShow,
  type Movie,
  type Album,
  type Audiobook,
  type CreateLibraryInput,
  type LibraryChangedEvent,
} from '../../lib/graphql'

export const Route = createFileRoute('/libraries/')({
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
  component: LibrariesPage,
  errorComponent: RouteError,
})

function LibrariesPage() {
  const { isOpen: isAddOpen, onOpen: onAddOpen, onClose: onAddClose } = useDisclosure()
  const { isOpen: isConfirmOpen, onOpen: onConfirmOpen, onClose: onConfirmClose } = useDisclosure()
  const [libraries, setLibraries] = useState<Library[]>([])
  const [showsByLibrary, setShowsByLibrary] = useState<Record<string, TvShow[]>>({})
  const [moviesByLibrary, setMoviesByLibrary] = useState<Record<string, Movie[]>>({})
  const [albumsByLibrary, setAlbumsByLibrary] = useState<Record<string, Album[]>>({})
  const [audiobooksByLibrary, setAudiobooksByLibrary] = useState<Record<string, Audiobook[]>>({})
  const [libraryToDelete, setLibraryToDelete] = useState<{ id: string; name: string } | null>(null)
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState(false)

  // Track if initial load is done to avoid showing spinner on background refreshes
  const initialLoadDone = useRef(false)

  const fetchLibraries = useCallback(async (isBackgroundRefresh = false) => {
    try {
      // Only show loading spinner on initial load
      if (!isBackgroundRefresh) {
        setLoading(true)
      }
      const { data, error } = await graphqlClient
        .query<{ libraries: Library[] }>(LIBRARIES_QUERY)
        .toPromise()

      if (error) {
        console.error('Failed to fetch libraries:', error)
        if (!isBackgroundRefresh) {
          addToast({
            title: 'Error',
            description: 'Failed to load libraries',
            color: 'danger',
          })
        }
        return
      }

      if (data?.libraries) {
        setLibraries(data.libraries)

        // Fetch shows for TV libraries (for artwork)
        const tvLibraries = data.libraries.filter((l) => l.libraryType === 'TV')
        const showsPromises = tvLibraries.map(async (lib) => {
          try {
            const result = await graphqlClient
              .query<{ tvShows: TvShow[] }>(TV_SHOWS_QUERY, { libraryId: lib.id })
              .toPromise()
            return { libraryId: lib.id, shows: result.data?.tvShows || [] }
          } catch {
            return { libraryId: lib.id, shows: [] }
          }
        })

        const showsResults = await Promise.all(showsPromises)
        const showsMap: Record<string, TvShow[]> = {}
        for (const result of showsResults) {
          showsMap[result.libraryId] = result.shows
        }
        setShowsByLibrary(showsMap)

        // Fetch movies for Movie libraries (for artwork)
        const movieLibraries = data.libraries.filter((l) => l.libraryType === 'MOVIES')
        const moviesPromises = movieLibraries.map(async (lib) => {
          try {
            const result = await graphqlClient
              .query<{ movies: Movie[] }>(MOVIES_QUERY, { libraryId: lib.id })
              .toPromise()
            return { libraryId: lib.id, movies: result.data?.movies || [] }
          } catch {
            return { libraryId: lib.id, movies: [] }
          }
        })

        const moviesResults = await Promise.all(moviesPromises)
        const moviesMap: Record<string, Movie[]> = {}
        for (const result of moviesResults) {
          moviesMap[result.libraryId] = result.movies
        }
        setMoviesByLibrary(moviesMap)

        // Fetch albums for Music libraries (for artwork)
        const musicLibraries = data.libraries.filter((l) => l.libraryType === 'MUSIC')
        const albumsPromises = musicLibraries.map(async (lib) => {
          try {
            const result = await graphqlClient
              .query<{ albums: Album[] }>(ALBUMS_QUERY, { libraryId: lib.id })
              .toPromise()
            return { libraryId: lib.id, albums: result.data?.albums || [] }
          } catch {
            return { libraryId: lib.id, albums: [] }
          }
        })

        const albumsResults = await Promise.all(albumsPromises)
        const albumsMap: Record<string, Album[]> = {}
        for (const result of albumsResults) {
          albumsMap[result.libraryId] = result.albums
        }
        setAlbumsByLibrary(albumsMap)

        // Fetch audiobooks for Audiobook libraries (for artwork)
        const audiobookLibraries = data.libraries.filter((l) => l.libraryType === 'AUDIOBOOKS')
        const audiobooksPromises = audiobookLibraries.map(async (lib) => {
          try {
            const result = await graphqlClient
              .query<{ audiobooks: Audiobook[] }>(AUDIOBOOKS_QUERY, { libraryId: lib.id })
              .toPromise()
            return { libraryId: lib.id, audiobooks: result.data?.audiobooks || [] }
          } catch {
            return { libraryId: lib.id, audiobooks: [] }
          }
        })

        const audiobooksResults = await Promise.all(audiobooksPromises)
        const audiobooksMap: Record<string, Audiobook[]> = {}
        for (const result of audiobooksResults) {
          audiobooksMap[result.libraryId] = result.audiobooks
        }
        setAudiobooksByLibrary(audiobooksMap)
      }
    } catch (err) {
      console.error('Failed to fetch libraries:', err)
    } finally {
      setLoading(false)
      initialLoadDone.current = true
    }
  }, [])

  useEffect(() => {
    fetchLibraries()
  }, [fetchLibraries])

  // Subscribe to library changes for real-time updates
  useEffect(() => {
    const subscription = graphqlClient
      .subscription<{ libraryChanged: LibraryChangedEvent }>(
        LIBRARY_CHANGED_SUBSCRIPTION,
        {}
      )
      .subscribe({
        next: (result) => {
          if (result.data?.libraryChanged) {
            const event = result.data.libraryChanged

            switch (event.changeType) {
              case 'CREATED':
                // Add new library to the list
                if (event.library) {
                  setLibraries((prev) => [...prev, event.library!])
                }
                break
              case 'UPDATED':
                // Update the library in the list
                if (event.library) {
                  setLibraries((prev) =>
                    prev.map((lib) => (lib.id === event.libraryId ? event.library! : lib))
                  )
                }
                break
              case 'DELETED':
                // Remove the library from the list
                setLibraries((prev) => prev.filter((lib) => lib.id !== event.libraryId))
                break
            }
          }
        },
      })

    return () => subscription.unsubscribe()
  }, [])

  // Skeleton card for loading state
  const SkeletonLibraryCard = () => (
    <Card className="aspect-[2/3] bg-content1">
      <CardBody className="p-0 overflow-hidden">
        <Skeleton className="w-full h-full" />
      </CardBody>
    </Card>
  )

  const handleAddLibrary = async (input: CreateLibraryInput) => {
    try {
      setActionLoading(true)
      const { data, error } = await graphqlClient
        .mutation<{
          createLibrary: {
            success: boolean
            library: Library | null
            error: string | null
          }
        }>(CREATE_LIBRARY_MUTATION, { input })
        .toPromise()

      if (error || !data?.createLibrary.success) {
        const errorMsg = data?.createLibrary.error || error?.message || 'Unknown error'
        addToast({
          title: 'Error',
          description: `Failed to create library: ${errorMsg}`,
          color: 'danger',
        })
        return
      }

      addToast({
        title: 'Success',
        description: `Library "${input.name}" created`,
        color: 'success',
      })

      // Refresh libraries
      await fetchLibraries()
    } catch (err) {
      console.error('Failed to create library:', err)
      addToast({
        title: 'Error',
        description: 'Failed to create library',
        color: 'danger',
      })
    } finally {
      setActionLoading(false)
    }
  }

  const handleScan = async (libraryId: string, libraryName: string) => {
    try {
      const { data, error } = await graphqlClient
        .mutation<{
          scanLibrary: { status: string; message: string | null }
        }>(SCAN_LIBRARY_MUTATION, { id: libraryId })
        .toPromise()

      if (error) {
        addToast({
          title: 'Error',
          description: sanitizeError(error),
          color: 'danger',
        })
        return
      }

      addToast({
        title: 'Scan Started',
        description: data?.scanLibrary.message || `Scanning ${libraryName}...`,
        color: 'primary',
      })
    } catch (err) {
      console.error('Failed to scan library:', err)
    }
  }

  const handleDeleteClick = (libraryId: string, libraryName: string) => {
    setLibraryToDelete({ id: libraryId, name: libraryName })
    onConfirmOpen()
  }

  const handleDelete = async () => {
    if (!libraryToDelete) return

    try {
      const { data, error } = await graphqlClient
        .mutation<{
          deleteLibrary: { success: boolean; error: string | null }
        }>(DELETE_LIBRARY_MUTATION, { id: libraryToDelete.id })
        .toPromise()

      if (error || !data?.deleteLibrary.success) {
        addToast({
          title: 'Error',
          description: sanitizeError(data?.deleteLibrary.error || 'Failed to delete library'),
          color: 'danger',
        })
        onConfirmClose()
        return
      }

      addToast({
        title: 'Deleted',
        description: `Library "${libraryToDelete.name}" deleted`,
        color: 'success',
      })

      // Refresh libraries
      await fetchLibraries()
    } catch (err) {
      console.error('Failed to delete library:', err)
    }
    onConfirmClose()
  }

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header with title and add button */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Libraries</h1>
          <p className="text-default-500">Organize and manage your media collections</p>
        </div>
        <Tooltip content="Add Library">
          <Button isIconOnly color="primary" size="sm" onPress={onAddOpen}>
            <IconPlus size={16} />
          </Button>
        </Tooltip>
      </div>

      {/* Content */}
      {loading ? (
        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <SkeletonLibraryCard key={i} />
          ))}
        </div>
      ) : libraries.length === 0 ? (
        <Card className="bg-content1/50 border-default-300 border-dashed border-2">
          <CardBody className="py-16 text-center">
            <div className="mx-auto w-20 h-20 rounded-full bg-default-100 flex items-center justify-center mb-6">
              <span className="text-4xl">📚</span>
            </div>
            <h3 className="text-xl font-semibold mb-2">No libraries yet</h3>
            <p className="text-default-500 mb-6 max-w-md mx-auto">
              Libraries help you organize your media. Add a library to start managing your movies,
              TV shows, music, and more.
            </p>
            <Button color="primary" size="lg" onPress={onAddOpen}>
              Add Your First Library
            </Button>
          </CardBody>
        </Card>
      ) : (
        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
          {libraries.map((library) => (
            <LibraryGridCard
              key={library.id}
              library={library}
              shows={showsByLibrary[library.id] || []}
              movies={moviesByLibrary[library.id] || []}
              albums={albumsByLibrary[library.id] || []}
              audiobooks={audiobooksByLibrary[library.id] || []}
              onScan={() => handleScan(library.id, library.name)}
              onDelete={() => handleDeleteClick(library.id, library.name)}
            />
          ))}
        </div>
      )}

      {/* Confirm Delete Modal */}
      <ConfirmModal
        isOpen={isConfirmOpen}
        onClose={onConfirmClose}
        onConfirm={handleDelete}
        title="Delete Library"
        message={`Are you sure you want to delete "${libraryToDelete?.name}"?`}
        description="This will remove the library and all associated shows from your collection. Downloaded files will not be deleted."
        confirmLabel="Delete"
        confirmColor="danger"
      />

      <AddLibraryModal
        isOpen={isAddOpen}
        onClose={onAddClose}
        onAdd={handleAddLibrary}
        isLoading={actionLoading}
      />
    </div>
  )
}
