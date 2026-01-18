import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useRef } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { useDisclosure } from '@heroui/modal'
import { Chip } from '@heroui/chip'
import { Tooltip } from '@heroui/tooltip'
import { Skeleton } from '@heroui/skeleton'
import { Table, TableHeader, TableColumn, TableBody, TableRow, TableCell } from '@heroui/table'
import { Dropdown, DropdownTrigger, DropdownMenu, DropdownItem } from '@heroui/dropdown'
import { addToast } from '@heroui/toast'
import { ConfirmModal } from '../../components/ConfirmModal'
import {
  graphqlClient,
  RSS_FEEDS_QUERY,
  CREATE_RSS_FEED_MUTATION,
  UPDATE_RSS_FEED_MUTATION,
  DELETE_RSS_FEED_MUTATION,
  TEST_RSS_FEED_MUTATION,
  POLL_RSS_FEED_MUTATION,
  type RssFeed,
  type RssFeedResult,
  type RssFeedTestResult,
} from '../../lib/graphql'
import { usePeriodicRefresh, useFocusRefresh } from '../../hooks/useSubscription'
import { IconRefresh, IconPlayerPause, IconPlayerPlay, IconPencil, IconTrash, IconTestPipe, IconBulb } from '@tabler/icons-react'
import { sanitizeError } from '../../lib/format'
import {
  AddRssFeedModal,
  EditRssFeedModal,
  TestRssFeedModal,
  type RssFeedFormData,
} from '../../components/settings'
import { formatDateTime } from '../../lib/format'

export const Route = createFileRoute('/settings/rss')({
  component: RssSettingsPage,
})

interface MutationResult {
  success: boolean
  error: string | null
}

function RssSettingsPage() {
  const [feeds, setFeeds] = useState<RssFeed[]>([])
  const [isLoading, setIsLoading] = useState(true)

  // Modal states
  const addModal = useDisclosure()
  const editModal = useDisclosure()
  const testModal = useDisclosure()
  const confirmModal = useDisclosure()
  const [editingFeed, setEditingFeed] = useState<RssFeed | null>(null)
  const [feedToDelete, setFeedToDelete] = useState<RssFeed | null>(null)
  const [testUrl, setTestUrl] = useState('')
  const [isAdding, setIsAdding] = useState(false)
  const [isEditing, setIsEditing] = useState(false)

  // Polling state
  const [pollingFeedId, setPollingFeedId] = useState<string | null>(null)

  // Track if initial load is done
  const initialLoadDone = useRef(false)

  const fetchFeeds = useCallback(async (isBackgroundRefresh = false) => {
    try {
      const result = await graphqlClient
        .query<{ rssFeeds: RssFeed[] }>(RSS_FEEDS_QUERY, {})
        .toPromise()
      if (result.data?.rssFeeds) {
        setFeeds(result.data.rssFeeds)
      }
      if (result.error && !isBackgroundRefresh) {
        addToast({
          title: 'Error',
          description: sanitizeError(result.error),
          color: 'danger',
        })
      }
    } catch (e) {
      if (!isBackgroundRefresh) {
        addToast({
          title: 'Error',
          description: sanitizeError(e),
          color: 'danger',
        })
      }
    } finally {
      setIsLoading(false)
      initialLoadDone.current = true
    }
  }, [])

  useEffect(() => {
    fetchFeeds()
  }, [fetchFeeds])

  // Subscribe to periodic updates (RSS feeds poll in background)
  usePeriodicRefresh(
    () => {
      if (initialLoadDone.current) {
        fetchFeeds(true)
      }
    },
    30000, // Refresh every 30 seconds
    true
  )

  useFocusRefresh(
    () => {
      if (initialLoadDone.current) {
        fetchFeeds(true)
      }
    },
    true
  )

  const handleAddFeed = async (data: RssFeedFormData) => {
    setIsAdding(true)

    try {
      const result = await graphqlClient
        .mutation<{ createRssFeed: RssFeedResult }>(CREATE_RSS_FEED_MUTATION, {
          input: {
            name: data.name,
            url: data.url,
            enabled: data.enabled,
            pollIntervalMinutes: data.pollIntervalMinutes,
          },
        })
        .toPromise()

      if (result.data?.createRssFeed.success) {
        addToast({
          title: 'Feed Added',
          description: 'RSS feed added successfully',
          color: 'success',
        })
        addModal.onClose()
        fetchFeeds()
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(result.data?.createRssFeed.error || 'Failed to add feed'),
          color: 'danger',
        })
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    } finally {
      setIsAdding(false)
    }
  }

  const handleEditFeed = async (data: RssFeedFormData) => {
    if (!editingFeed) return
    setIsEditing(true)

    try {
      const result = await graphqlClient
        .mutation<{ updateRssFeed: RssFeedResult }>(UPDATE_RSS_FEED_MUTATION, {
          id: editingFeed.id,
          input: {
            name: data.name,
            url: data.url,
            enabled: data.enabled,
            pollIntervalMinutes: data.pollIntervalMinutes,
          },
        })
        .toPromise()

      if (result.data?.updateRssFeed.success) {
        addToast({
          title: 'Feed Updated',
          description: 'RSS feed updated successfully',
          color: 'success',
        })
        editModal.onClose()
        setEditingFeed(null)
        fetchFeeds()
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(result.data?.updateRssFeed.error || 'Failed to update feed'),
          color: 'danger',
        })
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    } finally {
      setIsEditing(false)
    }
  }

  const handleDeleteClick = (feed: RssFeed) => {
    setFeedToDelete(feed)
    confirmModal.onOpen()
  }

  const handleDeleteFeed = async () => {
    if (!feedToDelete) return

    try {
      const result = await graphqlClient
        .mutation<{ deleteRssFeed: MutationResult }>(DELETE_RSS_FEED_MUTATION, { id: feedToDelete.id })
        .toPromise()

      if (result.data?.deleteRssFeed.success) {
        addToast({
          title: 'Feed Deleted',
          description: 'RSS feed deleted successfully',
          color: 'success',
        })
        fetchFeeds()
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(result.data?.deleteRssFeed.error || 'Failed to delete feed'),
          color: 'danger',
        })
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    }
    confirmModal.onClose()
  }

  const handleTestFeed = async (url: string): Promise<RssFeedTestResult> => {
    const result = await graphqlClient
      .mutation<{ testRssFeed: RssFeedTestResult }>(TEST_RSS_FEED_MUTATION, {
        url,
      })
      .toPromise()

    if (result.data?.testRssFeed) {
      return result.data.testRssFeed
    }

    throw new Error('Failed to test feed')
  }

  const handlePollFeed = async (id: string) => {
    setPollingFeedId(id)

    try {
      const result = await graphqlClient
        .mutation<{ pollRssFeed: RssFeedResult }>(POLL_RSS_FEED_MUTATION, { id })
        .toPromise()

      if (result.data?.pollRssFeed.success) {
        addToast({
          title: 'Feed Polled',
          description: 'Feed polled successfully',
          color: 'success',
        })
        fetchFeeds()
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(result.data?.pollRssFeed.error || 'Failed to poll feed'),
          color: 'danger',
        })
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    } finally {
      setPollingFeedId(null)
    }
  }

  const handleToggleEnabled = async (feed: RssFeed) => {
    try {
      const result = await graphqlClient
        .mutation<{ updateRssFeed: RssFeedResult }>(UPDATE_RSS_FEED_MUTATION, {
          id: feed.id,
          input: {
            name: feed.name,
            url: feed.url,
            enabled: !feed.enabled,
            pollIntervalMinutes: feed.pollIntervalMinutes,
          },
        })
        .toPromise()

      if (result.data?.updateRssFeed.success) {
        addToast({
          title: feed.enabled ? 'Feed Disabled' : 'Feed Enabled',
          description: feed.enabled ? 'Feed has been disabled' : 'Feed has been enabled',
          color: 'success',
        })
        fetchFeeds()
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(result.data?.updateRssFeed.error || 'Failed to toggle feed'),
          color: 'danger',
        })
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    }
  }

  const openEditModal = (feed: RssFeed) => {
    setEditingFeed(feed)
    editModal.onOpen()
  }

  const openTestModal = (url?: string) => {
    setTestUrl(url || '')
    testModal.onOpen()
  }

  // Skeleton loading content for the table
  const SkeletonTable = () => (
    <div className="p-4 space-y-3">
      {[1, 2, 3].map((i) => (
        <div key={i} className="flex items-center gap-4">
          <div className="flex-1">
            <Skeleton className="w-32 h-4 rounded mb-1" />
            <Skeleton className="w-48 h-3 rounded" />
          </div>
          <Skeleton className="w-16 h-5 rounded-full" />
          <Skeleton className="w-20 h-4 rounded" />
          <Skeleton className="w-12 h-4 rounded" />
          <Skeleton className="w-8 h-8 rounded" />
        </div>
      ))}
    </div>
  )

  return (
    <div className="flex flex-col gap-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">RSS Feeds</h2>
          <p className="text-default-500 text-sm">
            Configure RSS feeds to automatically find new episodes
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="flat" onPress={() => openTestModal()}>
            Test URL
          </Button>
          <Button color="primary" onPress={addModal.onOpen}>
            Add Feed
          </Button>
        </div>
      </div>

      {/* Feeds Table */}
      <Card>
        <CardBody className="p-0">
          {isLoading ? (
            <SkeletonTable />
          ) : (
          <Table aria-label="RSS Feeds" removeWrapper>
            <TableHeader>
              <TableColumn>NAME</TableColumn>
              <TableColumn>STATUS</TableColumn>
              <TableColumn>LAST POLLED</TableColumn>
              <TableColumn>INTERVAL</TableColumn>
              <TableColumn width={80} align="center">ACTIONS</TableColumn>
            </TableHeader>
            <TableBody emptyContent="No RSS feeds configured">
              {feeds.map((feed) => (
                <TableRow key={feed.id}>
                  <TableCell>
                    <div className="flex flex-col gap-1">
                      <span className="font-medium">{feed.name}</span>
                      <span className="text-xs text-default-400 truncate max-w-md">
                        {feed.url}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    {feed.lastError ? (
                      <Tooltip content={feed.lastError}>
                        <Chip size="sm" color="danger" variant="flat">
                          Error ({feed.consecutiveFailures})
                        </Chip>
                      </Tooltip>
                    ) : feed.enabled ? (
                      <Chip size="sm" color="success" variant="flat">
                        Active
                      </Chip>
                    ) : (
                      <Chip size="sm" color="default" variant="flat">
                        Disabled
                      </Chip>
                    )}
                  </TableCell>
                  <TableCell>
                    <span className="text-sm text-default-500">
                      {formatDateTime(feed.lastPolledAt)}
                    </span>
                  </TableCell>
                  <TableCell>
                    <span className="text-sm">{feed.pollIntervalMinutes} min</span>
                  </TableCell>
                  <TableCell>
                    <div className="flex justify-end">
                      <Dropdown>
                        <DropdownTrigger>
                          <Button
                            isIconOnly
                            size="sm"
                            variant="light"
                            isLoading={pollingFeedId === feed.id}
                          >
                            ⋮
                          </Button>
                        </DropdownTrigger>
                        <DropdownMenu aria-label="Feed actions">
                          <DropdownItem
                            key="poll"
                            startContent={<IconRefresh size={16} />}
                            onPress={() => handlePollFeed(feed.id)}
                          >
                            Poll Now
                          </DropdownItem>
                          <DropdownItem
                            key="test"
                            startContent={<IconTestPipe size={16} />}
                            onPress={() => openTestModal(feed.url)}
                          >
                            Test Feed
                          </DropdownItem>
                          <DropdownItem
                            key="toggle"
                            startContent={feed.enabled ? <IconPlayerPause size={16} /> : <IconPlayerPlay size={16} />}
                            onPress={() => handleToggleEnabled(feed)}
                          >
                            {feed.enabled ? 'Disable' : 'Enable'}
                          </DropdownItem>
                          <DropdownItem
                            key="edit"
                            startContent={<IconPencil size={16} />}
                            onPress={() => openEditModal(feed)}
                          >
                            Edit
                          </DropdownItem>
                          <DropdownItem
                            key="delete"
                            startContent={<IconTrash size={16} className="text-red-400" />}
                            className="text-danger"
                            color="danger"
                            onPress={() => handleDeleteClick(feed)}
                          >
                            Delete
                          </DropdownItem>
                        </DropdownMenu>
                      </Dropdown>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          )}
        </CardBody>
      </Card>

      {/* Add Feed Modal */}
      <AddRssFeedModal
        isOpen={addModal.isOpen}
        onClose={addModal.onClose}
        onAdd={handleAddFeed}
        isLoading={isAdding}
      />

      {/* Edit Feed Modal */}
      <EditRssFeedModal
        isOpen={editModal.isOpen}
        onClose={editModal.onClose}
        feed={editingFeed}
        onSave={handleEditFeed}
        isLoading={isEditing}
      />

      {/* Test Feed Modal */}
      <TestRssFeedModal
        isOpen={testModal.isOpen}
        onClose={testModal.onClose}
        initialUrl={testUrl}
        onTest={handleTestFeed}
      />

      {/* Info Card */}
      <Card className="bg-content1/50">
        <CardBody>
          <div className="flex gap-3">
            <IconBulb size={24} className="text-amber-400 shrink-0 mt-0.5" />
            <div>
              <p className="font-medium">How RSS feeds work</p>
              <p className="text-sm text-default-500 mt-1">
                RSS feeds are polled periodically to find new torrent releases. When a release
                matches a wanted episode in your library, it will be marked as "available" and
                can be automatically downloaded.
              </p>
              <p className="text-sm text-default-500 mt-2">
                Most private trackers provide personal RSS feed URLs. Copy your RSS URL from
                your tracker's settings page.
              </p>
            </div>
          </div>
        </CardBody>
      </Card>

      {/* Confirm Delete Modal */}
      <ConfirmModal
        isOpen={confirmModal.isOpen}
        onClose={confirmModal.onClose}
        onConfirm={handleDeleteFeed}
        title="Delete RSS Feed"
        message={`Are you sure you want to delete "${feedToDelete?.name}"?`}
        description="This will stop monitoring this feed for new torrents."
        confirmLabel="Delete"
        confirmColor="danger"
      />
    </div>
  )
}
