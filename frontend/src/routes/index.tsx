import { createFileRoute, Link, redirect } from '@tanstack/react-router'
import { Button, Card, CardBody, Spinner } from '@heroui/react'
import { useAuth } from '../hooks/useAuth'
import { MediaCard } from '../components/MediaCard'
import { LIBRARY_TYPES, type MediaItem, type Library } from '../lib/api'

export const Route = createFileRoute('/')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/auth/login' })
    }
  },
  component: HomePage,
})

// Mock data for initial display
const mockMedia: MediaItem[] = []

const mockLibraries: Library[] = [
  {
    id: '1',
    name: 'Movies',
    path: '/data/media/Movies',
    library_type: 'movies',
    icon: 'film',
    color: 'purple',
    auto_scan: true,
    scan_interval_hours: 24,
    last_scanned_at: null,
    file_count: 142,
    total_size_bytes: 856000000000,
  },
  {
    id: '2',
    name: 'TV Shows',
    path: '/data/media/TV',
    library_type: 'tv',
    icon: 'tv',
    color: 'blue',
    auto_scan: true,
    scan_interval_hours: 6,
    last_scanned_at: '2026-01-08T12:00:00Z',
    file_count: 1247,
    total_size_bytes: 2340000000000,
  },
]

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

function HomePage() {
  const { user, loading } = useAuth()

  if (loading) {
    return (
      <div className="flex items-center justify-center h-[calc(100vh-4rem)]">
        <Spinner size="lg" color="primary" />
      </div>
    )
  }

  if (!user) {
    return (
      <div className="flex flex-col items-center justify-center h-[calc(100vh-4rem)] px-4">
        <h1 className="text-4xl font-bold mb-4 text-center">
          Welcome to Librarian
        </h1>
        <p className="text-default-500 text-lg mb-8 text-center max-w-md">
          Your local-first, privacy-preserving media library. Sign in to access
          your collection.
        </p>
        <Link to="/auth/login">
          <Button color="primary" size="lg">
            Get Started
          </Button>
        </Link>
      </div>
    )
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Hero section with backdrop */}
      <Card className="mb-8 overflow-hidden">
        <div className="relative h-48 md:h-64 bg-gradient-to-r from-blue-900 to-purple-900">
          <div className="absolute inset-0 bg-gradient-to-t from-background via-transparent to-transparent" />
          <div className="absolute bottom-0 left-0 p-6 md:p-8">
            <h1 className="text-2xl md:text-3xl font-bold mb-1">
              Welcome back!
            </h1>
            <p className="text-default-400">Your media library at a glance</p>
          </div>
        </div>
      </Card>

      {/* Libraries overview */}
      <section className="mb-8">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-semibold">Your Libraries</h2>
          <Link to="/libraries">
            <Button variant="light" color="primary" size="sm">
              Manage Libraries →
            </Button>
          </Link>
        </div>

        {mockLibraries.length === 0 ? (
          <Card className="border-dashed border-2 border-default-300 bg-content1/50">
            <CardBody className="py-8 text-center">
              <span className="text-4xl mb-3 block">📚</span>
              <h3 className="font-semibold mb-2">No libraries yet</h3>
              <p className="text-default-500 text-sm mb-4">
                Add a library to start organizing your media.
              </p>
              <Link to="/libraries">
                <Button color="primary">Add Library</Button>
              </Link>
            </CardBody>
          </Card>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {mockLibraries.map((library) => {
              const typeInfo =
                LIBRARY_TYPES.find((t) => t.value === library.library_type) ||
                LIBRARY_TYPES[4]
              return (
                <Link key={library.id} to="/libraries">
                  <Card
                    isPressable
                    className="bg-content1 hover:bg-content2 transition-colors"
                  >
                    <CardBody>
                      <div className="flex items-center gap-3 mb-3">
                        <span className="text-2xl">{typeInfo.icon}</span>
                        <div>
                          <h3 className="font-semibold">{library.name}</h3>
                          <p className="text-default-500 text-xs">
                            {typeInfo.label}
                          </p>
                        </div>
                      </div>
                      <div className="flex justify-between text-sm text-default-500">
                        <span>{library.file_count ?? 0} files</span>
                        <span>{formatBytes(library.total_size_bytes)}</span>
                      </div>
                    </CardBody>
                  </Card>
                </Link>
              )
            })}
          </div>
        )}
      </section>

      {/* Quick actions */}
      <section className="mb-8">
        <h2 className="text-xl font-semibold mb-4">Quick Actions</h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <Link to="/downloads">
            <Card isPressable className="bg-content1 hover:bg-content2 transition-colors">
              <CardBody>
                <span className="text-2xl mb-2 block">⬇️</span>
                <h3 className="font-semibold">Downloads</h3>
                <p className="text-default-500 text-sm">
                  Manage active downloads
                </p>
              </CardBody>
            </Card>
          </Link>
          <Link to="/subscriptions">
            <Card isPressable className="bg-content1 hover:bg-content2 transition-colors">
              <CardBody>
                <span className="text-2xl mb-2 block">📺</span>
                <h3 className="font-semibold">Subscriptions</h3>
                <p className="text-default-500 text-sm">
                  Track your favorite shows
                </p>
              </CardBody>
            </Card>
          </Link>
          <Link to="/libraries">
            <Card isPressable className="bg-content1 hover:bg-content2 transition-colors">
              <CardBody>
                <span className="text-2xl mb-2 block">📚</span>
                <h3 className="font-semibold">Libraries</h3>
                <p className="text-default-500 text-sm">
                  Configure media folders
                </p>
              </CardBody>
            </Card>
          </Link>
        </div>
      </section>

      {/* Recently Added */}
      {mockMedia.length > 0 && (
        <section className="mb-8">
          <h2 className="text-xl font-semibold mb-4">Recently Added</h2>
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
            {mockMedia.map((media) => (
              <MediaCard key={media.id} media={media} />
            ))}
          </div>
        </section>
      )}

      {/* Empty state when no content */}
      {mockMedia.length === 0 && mockLibraries.length > 0 && (
        <section className="mb-8">
          <h2 className="text-xl font-semibold mb-4">Recently Added</h2>
          <Card className="bg-content1/50">
            <CardBody className="py-8 text-center">
              <p className="text-lg text-default-600 mb-2">No media found yet</p>
              <p className="text-sm text-default-500">
                Scan your libraries or add some downloads to populate your
                collection.
              </p>
            </CardBody>
          </Card>
        </section>
      )}
    </div>
  )
}
