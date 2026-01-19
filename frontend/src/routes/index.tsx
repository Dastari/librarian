import { createFileRoute, Link, useNavigate, useSearch } from '@tanstack/react-router'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Chip } from '@heroui/chip'
import { Image } from '@heroui/image'
import { Spinner } from '@heroui/spinner'
import { useDisclosure } from '@heroui/modal'
import { useEffect } from 'react'
import { useAuth } from '../hooks/useAuth'
import { useDashboardCache } from '../hooks/useDashboardCache'
import { useDataReactivity } from '../hooks/useSubscription'
import { AlbumArtCarousel } from '../components/AlbumArtCarousel'
import { SignInModal } from '../components/SignInModal'
import { IconDeviceTv } from '@tabler/icons-react'

// Format air date to a readable format
function formatAirDate(dateStr: string): string {
  const date = new Date(dateStr)
  const today = new Date()
  const tomorrow = new Date(today)
  tomorrow.setDate(tomorrow.getDate() + 1)
  
  const isToday = date.toDateString() === today.toDateString()
  const isTomorrow = date.toDateString() === tomorrow.toDateString()
  
  if (isToday) return 'Today'
  if (isTomorrow) return 'Tomorrow'
  
  return date.toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' })
}

// Search params for the home page
interface HomeSearchParams {
  signin?: boolean
  redirect?: string
}

// The index route is public - unauthenticated users see the landing page
// Authenticated users see the dashboard
export const Route = createFileRoute('/')({
  validateSearch: (search: Record<string, unknown>): HomeSearchParams => {
    return {
      signin: search.signin === true || search.signin === 'true',
      redirect: typeof search.redirect === 'string' ? search.redirect : undefined,
    }
  },
  component: HomePage,
})


function HomePage() {
  const { user, loading: authLoading } = useAuth()
  const navigate = useNavigate()
  const { signin, redirect: redirectUrl } = useSearch({ from: '/' })
  const { isOpen, onOpen, onClose } = useDisclosure()
  
  // Dashboard data with caching - loads instantly from cache while fetching fresh data
  const { 
    data: { libraries, recentShows, libraryUpcoming, globalUpcoming },
    isLoading,
    isStale,
    isFetching,
    refetch 
  } = useDashboardCache(user?.id ?? null)

  // Refresh on window focus (subscriptions in useDashboardCache handle real-time updates)
  useDataReactivity(
    () => {
      if (user && !isLoading) {
        refetch()
      }
    },
    { onTorrentComplete: false, periodicInterval: false, onFocus: true }
  )

  // Open sign-in modal if signin search param is true
  useEffect(() => {
    if (signin && !user && !authLoading) {
      onOpen()
    }
  }, [signin, user, authLoading, onOpen])

  // Handle modal close - clear the search params
  const handleModalClose = () => {
    onClose()
    // Clear the signin param from URL
    navigate({ to: '/', search: {}, replace: true })
  }

  // Handle successful sign in
  const handleSignInSuccess = () => {
    // If there's a redirect URL, the modal will handle it
    // Otherwise just close the modal
    if (!redirectUrl) {
      handleModalClose()
    }
  }

  if (authLoading) {
    return (
      <div className="flex items-center justify-center grow w-full h-full">
        <Spinner size="lg" color="primary" />
      </div>
    )
  }

  if (!user) {
    return (
      <div className="relative h-[calc(100vh-4rem)] overflow-hidden w-full">
        {/* 3D Album Art Carousel Background */}
        <div className="absolute inset-0 bg-gradient-to-br from-blue-400 via-purple-500 to-indigo-600 dark:from-blue-950 dark:via-purple-950 dark:to-slate-950">
          <AlbumArtCarousel />
        </div>

        {/* Content overlay */}
        <div className="absolute inset-0 bg-gradient-to-t from-background via-background/60 to-transparent" />

        {/* Content */}
        <div className="relative flex flex-col items-center justify-center h-full px-4">
          {/* Large logo */}
          <img src="/logo.svg" alt="" className="h-24 w-24 md:h-32 md:w-32 mb-6 drop-shadow-2xl" />
          
          <h1 className="text-5xl md:text-6xl font-bold mb-4 text-center drop-shadow-4xl">
            Welcome to <span style={{ fontFamily: '"Playwrite Australia SA", cursive' }}>Librarian</span>
          </h1>
          <p className="text-default-600 text-xl mb-8 text-center max-w-md drop-shadow-lg">
            Self-hosted media automation: discover, download, organize, and stream your collection.
          </p>
          <Button color="primary" size="lg" className="shadow-2xl" onPress={onOpen}>
            Get Started
          </Button>
        </div>

        {/* Sign In Modal */}
        <SignInModal
          isOpen={isOpen}
          onClose={handleModalClose}
          onSuccess={handleSignInSuccess}
          redirectUrl={redirectUrl}
        />
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Hero section with 3D album art carousel */}
      <Card className="mb-8 overflow-hidden">
        <div className="relative h-56 md:h-72 bg-gradient-to-br from-blue-400 via-purple-500 to-indigo-600 dark:from-blue-950 dark:via-purple-950 dark:to-slate-950">
          {/* 3D Album Art Carousel */}
          <AlbumArtCarousel />

          {/* Content overlay - fade to primary-900 for better text contrast */}
          <div className="absolute inset-0 bg-gradient-to-t from-primary-900/95 via-primary-900/40 to-transparent pointer-events-none" />

          {/* Text content */}
          <div className="absolute bottom-0 left-0 p-6 md:p-8 z-10">
            <h1 className="text-2xl md:text-4xl font-bold mb-2 drop-shadow-lg text-white">
              Welcome back!
            </h1>
            <p className="text-white/70 drop-shadow-md">
              Your media library at a glance
              {isFetching && isStale && (
                <span className="ml-2 text-xs text-primary-200 animate-pulse">• Refreshing...</span>
              )}
            </p>
          </div>
        </div>
      </Card>

      {/* Airing Soon in Your Library */}
      {libraryUpcoming.length > 0 && (
        <section className="mb-8">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h2 className="text-xl font-semibold">Airing Soon in Your Library</h2>
              <p className="text-default-500 text-sm">Episodes from shows you're tracking</p>
            </div>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {libraryUpcoming.slice(0, 8).map((ep) => (
              <Link key={ep.id} to="/shows/$showId" params={{ showId: ep.show.id }}>
                <Card 
                  isPressable 
                  className="bg-content1 hover:bg-content2 transition-colors overflow-hidden w-full"
                >
                  <div className="flex gap-3 p-3">
                    {/* Show poster */}
                    <div className="w-16 h-24 shrink-0 rounded-md overflow-hidden bg-default-200">
                      {ep.show.posterUrl ? (
                        <Image
                          src={ep.show.posterUrl}
                          alt={ep.show.name}
                          classNames={{
                            wrapper: "w-full h-full",
                            img: "w-full h-full object-cover"
                          }}
                          radius="none"
                          removeWrapper={false}
                        />
                      ) : (
                        <div className="w-full h-full flex items-center justify-center">
                          <IconDeviceTv size={32} className="text-blue-400" />
                        </div>
                      )}
                    </div>
                    {/* Episode info */}
                    <div className='flex-1 min-w-0 text-left flex flex-col'>
                      <p className="font-semibold truncate">{ep.show.name}</p>
                      <p className="text-sm text-default-500 grow">
                        S{ep.season.toString().padStart(2, '0')}E{ep.episode.toString().padStart(2, '0')}
                        {ep.name && `: ${ep.name}`}
                      </p>
                      <div className="flex items-center gap-2">
                        <Chip size="sm" variant="flat" color="primary">
                          {formatAirDate(ep.airDate)}
                        </Chip>
                        {ep.show.network && (
                          <span className="text-xs text-default-400">{ep.show.network}</span>
                        )}
                      </div>
                    </div>
                  </div>
                </Card>
              </Link>
            ))}
          </div>
        </section>
      )}

      {/* Recently Added Shows */}
      {recentShows.length > 0 && (
        <section className="mb-8">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xl font-semibold">Recently Added Shows</h2>
            <Link to="/libraries">
              <Button variant="light" color="primary" size="sm">
                View All →
              </Button>
            </Link>
          </div>
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
            {recentShows.map((show) => (
              <Link key={show.id} to="/shows/$showId" params={{ showId: show.id }}>
                <Card isPressable isHoverable className="bg-content1 overflow-hidden">
                  <div className="aspect-[2/3] relative">
                    {show.posterUrl ? (
                      <Image
                        src={show.posterUrl}
                        alt={show.name}
                        classNames={{
                          wrapper: "w-full h-full !max-w-full",
                          img: "w-full h-full object-cover"
                        }}
                        radius="none"
                      />
                    ) : (
                      <div className="w-full h-full bg-default-200 flex items-center justify-center">
                        <IconDeviceTv size={40} className="text-blue-400" />
                      </div>
                    )}
                  </div>
                  <CardBody className="p-2">
                    <p className="text-sm font-medium truncate">{show.name}</p>
                    <p className="text-xs text-default-500">
                      {show.episodeFileCount ?? 0} / {show.episodeCount ?? 0} episodes
                    </p>
                  </CardBody>
                </Card>
              </Link>
            ))}
          </div>
        </section>
      )}

      {/* Airing Soon (Global) */}
      {globalUpcoming.length > 0 && (
        <section className="mb-8">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h2 className="text-xl font-semibold">Airing Soon</h2>
              <p className="text-default-500 text-sm">Popular shows airing this week</p>
            </div>
          </div>
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
            {globalUpcoming.map((ep) => (
              <Card 
                key={`${ep.show.tvmazeId}-${ep.season}-${ep.episode}`}
                isHoverable
                className="bg-content1 overflow-hidden"
              >
                <div className="aspect-[2/3] relative">
                  {ep.show.posterUrl ? (
                    <Image
                      src={ep.show.posterUrl}
                      alt={ep.show.name}
                      classNames={{
                        wrapper: "w-full h-full !max-w-full",
                        img: "w-full h-full object-cover"
                      }}
                      radius="none"
                    />
                  ) : (
                    <div className="w-full h-full bg-default-200 flex items-center justify-center">
                      <IconDeviceTv size={40} className="text-blue-400" />
                    </div>
                  )}
                  {/* Air date badge */}
                  <div className="absolute top-2 right-2 z-10">
                    <Chip size="sm" variant="solid" className="bg-black/70">
                      {formatAirDate(ep.airDate)}
                    </Chip>
                  </div>
                </div>
                <CardBody className="p-2">
                  <p className="text-sm font-medium truncate">{ep.show.name}</p>
                  <p className="text-xs text-default-500">
                    S{ep.season.toString().padStart(2, '0')}E{ep.episode.toString().padStart(2, '0')}
                    {ep.show.network && ` • ${ep.show.network}`}
                  </p>
                </CardBody>
              </Card>
            ))}
          </div>
        </section>
      )}

      {/* Empty state when no recent content but has libraries */}
      {recentShows.length === 0 && libraryUpcoming.length === 0 && libraries.length > 0 && !isLoading && (
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
