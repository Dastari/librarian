import { createFileRoute, Link, redirect } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import {
  Button,
  Card,
  CardBody,
  useDisclosure,
  Spinner,
  addToast,
} from '@heroui/react'
import { useAuth } from '../../hooks/useAuth'
import { TvShowCard, AddShowModal } from '../../components/library'
import {
  graphqlClient,
  LIBRARY_QUERY,
  TV_SHOWS_QUERY,
  DELETE_TV_SHOW_MUTATION,
  QUALITY_PROFILES_QUERY,
  type Library,
  type TvShow,
  type QualityProfile,
} from '../../lib/graphql'

export const Route = createFileRoute('/libraries/$libraryId')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/auth/login' })
    }
  },
  component: LibraryDetailPage,
})

function formatBytes(bytes: number | null): string {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let unitIndex = 0
  let size = bytes
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }
  return `${size.toFixed(1)} ${units[unitIndex]}`
}

function LibraryDetailPage() {
  const { libraryId } = Route.useParams()
  const { user, loading: authLoading } = useAuth()
  const { isOpen, onOpen, onClose } = useDisclosure()
  const [library, setLibrary] = useState<Library | null>(null)
  const [tvShows, setTvShows] = useState<TvShow[]>([])
  const [qualityProfiles, setQualityProfiles] = useState<QualityProfile[]>([])
  const [loading, setLoading] = useState(true)

  const fetchData = useCallback(async () => {
    try {
      setLoading(true)

      // Fetch library, TV shows, and quality profiles in parallel
      const [libraryResult, showsResult, profilesResult] = await Promise.all([
        graphqlClient
          .query<{ library: Library | null }>(LIBRARY_QUERY, { id: libraryId })
          .toPromise(),
        graphqlClient
          .query<{ tvShows: TvShow[] }>(TV_SHOWS_QUERY, { libraryId })
          .toPromise(),
        graphqlClient
          .query<{ qualityProfiles: QualityProfile[] }>(QUALITY_PROFILES_QUERY)
          .toPromise(),
      ])

      if (libraryResult.data?.library) {
        setLibrary(libraryResult.data.library)
      }
      if (showsResult.data?.tvShows) {
        setTvShows(showsResult.data.tvShows)
      }
      if (profilesResult.data?.qualityProfiles) {
        setQualityProfiles(profilesResult.data.qualityProfiles)
      }
    } catch (err) {
      console.error('Failed to fetch data:', err)
    } finally {
      setLoading(false)
    }
  }, [libraryId])

  useEffect(() => {
    if (user) {
      fetchData()
    }
  }, [user, fetchData])

  const handleDeleteShow = async (showId: string, showName: string) => {
    if (
      !confirm(
        `Are you sure you want to delete "${showName}"? This will not delete files.`
      )
    ) {
      return
    }

    try {
      const { data, error } = await graphqlClient
        .mutation<{ deleteTvShow: { success: boolean; error: string | null } }>(
          DELETE_TV_SHOW_MUTATION,
          { id: showId }
        )
        .toPromise()

      if (error || !data?.deleteTvShow.success) {
        addToast({
          title: 'Error',
          description: data?.deleteTvShow.error || 'Failed to delete show',
          color: 'danger',
        })
        return
      }

      addToast({
        title: 'Deleted',
        description: `"${showName}" removed from library`,
        color: 'success',
      })

      await fetchData()
    } catch (err) {
      console.error('Failed to delete show:', err)
    }
  }

  if (authLoading || loading) {
    return (
      <div className="flex items-center justify-center h-[calc(100vh-4rem)]">
        <Spinner size="lg" color="primary" />
      </div>
    )
  }

  if (!library) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <Card className="bg-content1">
          <CardBody className="text-center py-12">
            <h2 className="text-xl font-semibold mb-4">Library not found</h2>
            <Link to="/libraries">
              <Button color="primary">Back to Libraries</Button>
            </Link>
          </CardBody>
        </Card>
      </div>
    )
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-2 mb-1">
            <Link to="/libraries" className="text-default-500 hover:text-default-700">
              Libraries
            </Link>
            <span className="text-default-400">/</span>
            <span>{library.name}</span>
          </div>
          <h1 className="text-2xl font-bold">{library.name}</h1>
          <p className="text-default-500">
            {tvShows.length} shows • {formatBytes(library.totalSizeBytes)}
          </p>
        </div>
        <Button color="primary" onPress={onOpen}>
          + Add Show
        </Button>
      </div>

      {/* TV Shows Grid */}
      {tvShows.length === 0 ? (
        <Card className="bg-content1/50 border-default-300 border-dashed border-2">
          <CardBody className="py-12 text-center">
            <span className="text-5xl mb-4 block">📺</span>
            <h3 className="text-lg font-semibold mb-2">No shows yet</h3>
            <p className="text-default-500 mb-4">
              Add TV shows to start tracking episodes.
            </p>
            <Button color="primary" onPress={onOpen}>
              Add Your First Show
            </Button>
          </CardBody>
        </Card>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          {tvShows.map((show) => (
            <TvShowCard
              key={show.id}
              show={show}
              onDelete={() => handleDeleteShow(show.id, show.name)}
            />
          ))}
        </div>
      )}

      <AddShowModal
        isOpen={isOpen}
        onClose={onClose}
        libraryId={libraryId}
        qualityProfiles={qualityProfiles}
        onAdded={fetchData}
      />
    </div>
  )
}
