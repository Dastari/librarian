import { createFileRoute, Link, redirect } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import {
  Button,
  Card,
  CardBody,
  CardHeader,
  useDisclosure,
  Spinner,
  Divider,
  addToast,
} from '@heroui/react'
import { useAuth } from '../../hooks/useAuth'
import { LibraryCard, AddLibraryModal } from '../../components/library'
import {
  graphqlClient,
  LIBRARIES_QUERY,
  CREATE_LIBRARY_MUTATION,
  SCAN_LIBRARY_MUTATION,
  DELETE_LIBRARY_MUTATION,
  type Library,
  type CreateLibraryInput,
} from '../../lib/graphql'

export const Route = createFileRoute('/libraries/')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/auth/login' })
    }
  },
  component: LibrariesPage,
})

function LibrariesPage() {
  const { user, loading: authLoading } = useAuth()
  const { isOpen, onOpen, onClose } = useDisclosure()
  const [libraries, setLibraries] = useState<Library[]>([])
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState(false)

  const fetchLibraries = useCallback(async () => {
    try {
      setLoading(true)
      const { data, error } = await graphqlClient
        .query<{ libraries: Library[] }>(LIBRARIES_QUERY)
        .toPromise()

      if (error) {
        console.error('Failed to fetch libraries:', error)
        addToast({
          title: 'Error',
          description: 'Failed to load libraries',
          color: 'danger',
        })
        return
      }

      if (data?.libraries) {
        setLibraries(data.libraries)
      }
    } catch (err) {
      console.error('Failed to fetch libraries:', err)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (user) {
      fetchLibraries()
    }
  }, [user, fetchLibraries])

  if (authLoading || loading) {
    return (
      <div className="flex items-center justify-center h-[calc(100vh-4rem)]">
        <Spinner size="lg" color="primary" />
      </div>
    )
  }

  if (!user) {
    return (
      <div className="flex flex-col items-center justify-center h-[calc(100vh-4rem)] px-4">
        <h1 className="text-2xl font-bold mb-4">Sign in to manage libraries</h1>
        <Link to="/auth/login">
          <Button color="primary">Sign In</Button>
        </Link>
      </div>
    )
  }

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
          description: `Failed to start scan: ${error.message}`,
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

  const handleDelete = async (libraryId: string, libraryName: string) => {
    if (!confirm(`Are you sure you want to delete "${libraryName}"? This cannot be undone.`)) {
      return
    }

    try {
      const { data, error } = await graphqlClient
        .mutation<{
          deleteLibrary: { success: boolean; error: string | null }
        }>(DELETE_LIBRARY_MUTATION, { id: libraryId })
        .toPromise()

      if (error || !data?.deleteLibrary.success) {
        addToast({
          title: 'Error',
          description: data?.deleteLibrary.error || 'Failed to delete library',
          color: 'danger',
        })
        return
      }

      addToast({
        title: 'Deleted',
        description: `Library "${libraryName}" deleted`,
        color: 'success',
      })

      // Refresh libraries
      await fetchLibraries()
    } catch (err) {
      console.error('Failed to delete library:', err)
    }
  }

  const handleEdit = (libraryId: string) => {
    // TODO: Open edit modal
    console.log('Editing library:', libraryId)
    addToast({
      title: 'Coming Soon',
      description: 'Library settings editor coming soon!',
      color: 'warning',
    })
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Libraries</h1>
          <p className="text-default-500">Manage your media libraries</p>
        </div>
        <Button color="primary" onPress={onOpen}>
          + Add Library
        </Button>
      </div>

      {libraries.length === 0 ? (
        <Card className="bg-content1/50 border-default-300 border-dashed border-2">
          <CardBody className="py-12 text-center">
            <span className="text-5xl mb-4 block">📚</span>
            <h3 className="text-lg font-semibold mb-2">No libraries yet</h3>
            <p className="text-default-500 mb-4">
              Add a library to start organizing your media collection.
            </p>
            <Button color="primary" onPress={onOpen}>
              Add Your First Library
            </Button>
          </CardBody>
        </Card>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {libraries.map((library) => (
            <LibraryCard
              key={library.id}
              library={library}
              onScan={() => handleScan(library.id, library.name)}
              onEdit={() => handleEdit(library.id)}
              onDelete={() => handleDelete(library.id, library.name)}
            />
          ))}
        </div>
      )}

      <AddLibraryModal
        isOpen={isOpen}
        onClose={onClose}
        onAdd={handleAddLibrary}
        isLoading={actionLoading}
      />

      {/* Info section */}
      <Card className="mt-8 bg-content2">
        <CardHeader>
          <h3 className="font-semibold">About Libraries</h3>
        </CardHeader>
        <Divider />
        <CardBody>
          <ul className="text-default-500 text-sm space-y-1">
            <li>
              • <strong>TV Shows</strong> - Series organized by show, season, and
              episode with automatic episode tracking
            </li>
            <li>
              • <strong>Movies</strong> - Feature films, organized by title and
              year
            </li>
            <li>
              • <strong>Music</strong> - Albums and tracks (coming soon)
            </li>
            <li>
              • <strong>Audiobooks</strong> - Audio books organized by author and
              title (coming soon)
            </li>
            <li>• Each library can have its own scan schedule and settings</li>
            <li>
              • TV libraries support automatic episode tracking via metadata
              providers
            </li>
          </ul>
        </CardBody>
      </Card>
    </div>
  )
}
