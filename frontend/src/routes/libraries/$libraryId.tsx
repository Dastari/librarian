import { createFileRoute, Link, redirect, Outlet, useLocation } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useRef, createContext, useContext } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { useDisclosure } from '@heroui/modal'
import { Skeleton } from '@heroui/skeleton'
import { Chip } from '@heroui/chip'
import { addToast } from '@heroui/toast'
import { Breadcrumbs, BreadcrumbItem } from '@heroui/breadcrumbs'
import { ConfirmModal } from '../../components/ConfirmModal'
import { useDataReactivity } from '../../hooks/useSubscription'
import { RouteError } from '../../components/RouteError'
import {
  AddShowModal,
  LibraryLayout,
  type LibraryTab,
} from '../../components/library'
import { sanitizeError } from '../../lib/format'
import {
  graphqlClient,
  LIBRARY_QUERY,
  TV_SHOWS_QUERY,
  DELETE_TV_SHOW_MUTATION,
  UPDATE_LIBRARY_MUTATION,
  SCAN_LIBRARY_MUTATION,
  getLibraryTypeInfo,
  type Library,
  type TvShow,
  type UpdateLibraryInput,
} from '../../lib/graphql'
import { formatBytes } from '../../lib/format'

// Context for sharing library data with subroutes
export interface LibraryContextValue {
  library: Library
  tvShows: TvShow[]
  fetchData: (isBackgroundRefresh?: boolean) => Promise<void>
  actionLoading: boolean
  handleDeleteShowClick: (showId: string, showName: string) => void
  handleUpdateLibrary: (input: UpdateLibraryInput) => Promise<void>
  onOpenAddShow: () => void
}

export const LibraryContext = createContext<LibraryContextValue | null>(null)

export function useLibraryContext() {
  return useContext(LibraryContext)
}

export const Route = createFileRoute('/libraries/$libraryId')({
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
  component: LibraryDetailLayout,
  errorComponent: RouteError,
})


function LibraryDetailLayout() {
  const { libraryId } = Route.useParams()
  const location = useLocation()
  const { isOpen, onOpen, onClose } = useDisclosure()
  const { isOpen: isConfirmOpen, onOpen: onConfirmOpen, onClose: onConfirmClose } = useDisclosure()
  const [library, setLibrary] = useState<Library | null>(null)
  const [tvShows, setTvShows] = useState<TvShow[]>([])
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState(false)
  const [showToDelete, setShowToDelete] = useState<{ id: string; name: string } | null>(null)

  // Determine active tab from current URL
  const getActiveTab = (): LibraryTab => {
    const path = location.pathname
    if (path.endsWith('/unmatched')) return 'unmatched'
    if (path.endsWith('/browser')) return 'browser'
    if (path.endsWith('/settings')) return 'settings'
    return 'shows' // default
  }

  // Track if initial load is done to avoid showing spinner on background refreshes
  const initialLoadDone = useRef(false)

  const fetchData = useCallback(async (isBackgroundRefresh = false) => {
    try {
      // Only show loading spinner on initial load
      if (!isBackgroundRefresh) {
        setLoading(true)
      }

      // Fetch library and TV shows in parallel
      const [libraryResult, showsResult] = await Promise.all([
        graphqlClient
          .query<{ library: Library | null }>(LIBRARY_QUERY, { id: libraryId })
          .toPromise(),
        graphqlClient
          .query<{ tvShows: TvShow[] }>(TV_SHOWS_QUERY, { libraryId })
          .toPromise(),
      ])

      if (libraryResult.data?.library) {
        setLibrary(libraryResult.data.library)
      }
      if (showsResult.data?.tvShows) {
        setTvShows(showsResult.data.tvShows)
      }
    } catch (err) {
      console.error('Failed to fetch data:', err)
    } finally {
      setLoading(false)
      initialLoadDone.current = true
    }
  }, [libraryId])

  useEffect(() => {
    fetchData()
  }, [fetchData])

  // Update page title when library data is loaded
  useEffect(() => {
    if (library) {
      document.title = `Librarian - ${library.name}`
    }
    return () => {
      document.title = 'Librarian'
    }
  }, [library])

  // Subscribe to data changes for live updates
  useDataReactivity(
    () => {
      if (initialLoadDone.current) {
        fetchData(true)
      }
    },
    { onTorrentComplete: true, periodicInterval: 30000, onFocus: true }
  )

  const handleDeleteShowClick = (showId: string, showName: string) => {
    setShowToDelete({ id: showId, name: showName })
    onConfirmOpen()
  }

  const handleDeleteShow = async () => {
    if (!showToDelete) return

    try {
      const { data, error } = await graphqlClient
        .mutation<{ deleteTvShow: { success: boolean; error: string | null } }>(
          DELETE_TV_SHOW_MUTATION,
          { id: showToDelete.id }
        )
        .toPromise()

      if (error || !data?.deleteTvShow.success) {
        addToast({
          title: 'Error',
          description: sanitizeError(data?.deleteTvShow.error || 'Failed to delete show'),
          color: 'danger',
        })
        onConfirmClose()
        return
      }

      addToast({
        title: 'Deleted',
        description: `"${showToDelete.name}" removed from library`,
        color: 'success',
      })

      await fetchData()
    } catch (err) {
      console.error('Failed to delete show:', err)
    }
    onConfirmClose()
  }

  const handleUpdateLibrary = async (input: UpdateLibraryInput) => {
    if (!library) return

    try {
      setActionLoading(true)
      const { data, error } = await graphqlClient
        .mutation<{
          updateLibrary: {
            success: boolean
            library: Library | null
            error: string | null
          }
        }>(UPDATE_LIBRARY_MUTATION, { id: library.id, input })
        .toPromise()

      if (error || !data?.updateLibrary.success) {
        const errorMsg = data?.updateLibrary.error || error?.message || 'Unknown error'
        addToast({
          title: 'Error',
          description: `Failed to update library: ${errorMsg}`,
          color: 'danger',
        })
        return
      }

      addToast({
        title: 'Success',
        description: 'Library settings saved',
        color: 'success',
      })

      // Refresh library data
      await fetchData()
    } catch (err) {
      console.error('Failed to update library:', err)
      addToast({
        title: 'Error',
        description: 'Failed to update library',
        color: 'danger',
      })
    } finally {
      setActionLoading(false)
    }
  }

  const handleScanLibrary = async () => {
    if (!library) return

    try {
      const { data, error } = await graphqlClient
        .mutation<{
          scanLibrary: { status: string; message: string | null }
        }>(SCAN_LIBRARY_MUTATION, { id: library.id })
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
        description: data?.scanLibrary.message || `Scanning ${library.name}...`,
        color: 'primary',
      })
    } catch (err) {
      console.error('Failed to scan library:', err)
    }
  }

  // Loading skeleton for library detail page
  if (loading) {
    return (
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col grow">
        {/* Header Skeleton */}
        <div className="mb-6">
          <Skeleton className="w-48 h-4 rounded mb-4" />
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-4">
              <Skeleton className="w-12 h-12 rounded" />
              <div>
                <Skeleton className="w-48 h-7 rounded mb-2" />
                <div className="flex gap-2">
                  <Skeleton className="w-20 h-4 rounded" />
                  <Skeleton className="w-16 h-4 rounded" />
                  <Skeleton className="w-32 h-4 rounded" />
                </div>
              </div>
            </div>
            <div className="flex gap-2">
              <Skeleton className="w-16 h-6 rounded-full" />
              <Skeleton className="w-20 h-6 rounded-full" />
              <Skeleton className="w-24 h-9 rounded-lg" />
            </div>
          </div>
        </div>

        {/* Tabs Skeleton */}
        <div className="flex gap-2 mb-6">
          <Skeleton className="w-20 h-9 rounded-lg" />
          <Skeleton className="w-28 h-9 rounded-lg" />
          <Skeleton className="w-20 h-9 rounded-lg" />
          <Skeleton className="w-24 h-9 rounded-lg" />
        </div>

        {/* Content Skeleton */}
        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
          {Array.from({ length: 12 }).map((_, i) => (
            <Skeleton key={i} className="aspect-[2/3] rounded-lg" />
          ))}
        </div>
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

  const typeInfo = getLibraryTypeInfo(library.libraryType)

  const contextValue: LibraryContextValue = {
    library,
    tvShows,
    fetchData,
    actionLoading,
    handleDeleteShowClick,
    handleUpdateLibrary,
    onOpenAddShow: onOpen,
  }

  return (
    <LibraryContext.Provider value={contextValue}>
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col grow">
        {/* Header */}
        <div className="mb-6">
          {/* Breadcrumb */}
          <Breadcrumbs className="mb-2">
            <BreadcrumbItem href="/libraries">Libraries</BreadcrumbItem>
            <BreadcrumbItem isCurrent>{library.name}</BreadcrumbItem>
          </Breadcrumbs>

          {/* Title and Stats */}
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-4">
              <typeInfo.Icon className="w-10 h-10" />
              <div>
                <h1 className="text-2xl font-bold">{library.name}</h1>
                <div className="flex items-center gap-3 text-sm text-default-500 mt-1">
                  <span>{tvShows.length} shows</span>
                  <span>•</span>
                  <span>{formatBytes(library.totalSizeBytes)}</span>
                  <span>•</span>
                  <span className="font-mono text-xs">{library.path}</span>
                </div>
              </div>
            </div>

            <div className="flex items-center gap-2">
              {/* Quality Settings Chip */}
              {(() => {
                const resolutions = library.allowedResolutions || []
                const codecs = library.allowedVideoCodecs || []
                const requireHdr = library.requireHdr || false
                
                // Build summary
                const parts: string[] = []
                if (resolutions.length > 0) {
                  if (resolutions.includes('2160p')) parts.push('4K')
                  else if (resolutions.includes('1080p')) parts.push('1080p')
                  else if (resolutions.includes('720p')) parts.push('720p')
                  else parts.push(resolutions.join('/'))
                }
                if (codecs.length > 0) {
                  parts.push(codecs.map(c => c.toUpperCase()).join('/'))
                }
                if (requireHdr) {
                  parts.push('HDR')
                }
                
                const label = parts.length > 0 ? parts.join(' • ') : 'Any Quality'
                
                return (
                  <Chip size="sm" variant="flat" color="primary">
                    {label}
                  </Chip>
                )
              })()}
              {library.watchForChanges && (
                <Chip size="sm" color="secondary" variant="flat">
                  Watching
                </Chip>
              )}
              <Chip
                size="sm"
                color={library.autoScan ? 'success' : 'default'}
                variant="flat"
              >
                {library.autoScan ? 'Auto-scan' : 'Manual'}
              </Chip>
            <Button
              color="primary"
              variant="flat"
              size="sm"
              onPress={handleScanLibrary}
              isLoading={library.scanning}
              isDisabled={library.scanning}
            >
              {library.scanning ? 'Scanning...' : 'Scan Now'}
            </Button>
            </div>
          </div>
        </div>

        {/* Tabbed Content with Outlet for subroutes */}
        <LibraryLayout activeTab={getActiveTab()} libraryId={libraryId}>
          <Outlet />
        </LibraryLayout>

        {/* Add Show Modal */}
        <AddShowModal
          isOpen={isOpen}
          onClose={onClose}
          libraryId={libraryId}
          onAdded={fetchData}
        />

        {/* Confirm Delete Modal */}
        <ConfirmModal
          isOpen={isConfirmOpen}
          onClose={onConfirmClose}
          onConfirm={handleDeleteShow}
          title="Delete Show"
          message={`Are you sure you want to delete "${showToDelete?.name}"?`}
          description="This will remove the show from your library. Downloaded files will not be deleted."
          confirmLabel="Delete"
          confirmColor="danger"
        />
      </div>
    </LibraryContext.Provider>
  )
}
