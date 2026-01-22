import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import { Button } from '@heroui/button'
import { Chip } from '@heroui/chip'
import { Skeleton } from '@heroui/skeleton'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter, useDisclosure } from '@heroui/modal'
import { Tabs, Tab } from '@heroui/tabs'
import { addToast } from '@heroui/toast'
import {
  IconCheck,
  IconX,
  IconTrash,
  IconAlertTriangle,
  IconInfoCircle,
  IconAlertCircle,
  IconBellRinging,
  IconRefresh,
} from '@tabler/icons-react'
import {
  graphqlClient,
  NOTIFICATIONS_QUERY,
  MARK_NOTIFICATION_READ_MUTATION,
  MARK_ALL_NOTIFICATIONS_READ_MUTATION,
  RESOLVE_NOTIFICATION_MUTATION,
  DELETE_NOTIFICATION_MUTATION,
  type Notification,
  type NotificationType,
  type NotificationCategory,
  type NotificationResolution,
  type PaginatedNotifications,
} from '../lib/graphql'
import {
  DataTable,
  type DataTableColumn,
  type RowAction,
} from '../components/data-table'

export const Route = createFileRoute('/notifications')({
  component: NotificationsPage,
})

// Notification type info for display
const NOTIFICATION_TYPE_INFO: Record<
  NotificationType,
  { color: 'default' | 'primary' | 'success' | 'warning' | 'danger' | 'secondary'; label: string }
> = {
  INFO: { color: 'primary', label: 'Info' },
  WARNING: { color: 'warning', label: 'Warning' },
  ERROR: { color: 'danger', label: 'Error' },
  ACTION_REQUIRED: { color: 'secondary', label: 'Action Required' },
}

// Category labels
const CATEGORY_LABELS: Record<NotificationCategory, string> = {
  MATCHING: 'Matching',
  PROCESSING: 'Processing',
  QUALITY: 'Quality',
  STORAGE: 'Storage',
  EXTRACTION: 'Extraction',
}

const getNotificationIcon = (type: NotificationType) => {
  switch (type) {
    case 'ERROR':
      return <IconAlertCircle size={16} className="text-red-400" />
    case 'WARNING':
      return <IconAlertTriangle size={16} className="text-amber-400" />
    case 'ACTION_REQUIRED':
      return <IconBellRinging size={16} className="text-purple-400" />
    default:
      return <IconInfoCircle size={16} className="text-blue-400" />
  }
}

function formatTimestamp(isoString: string): string {
  const date = new Date(isoString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMs / 3600000)
  const diffDays = Math.floor(diffMs / 86400000)

  if (diffMins < 1) return 'Just now'
  if (diffMins < 60) return `${diffMins}m ago`
  if (diffHours < 24) return `${diffHours}h ago`
  if (diffDays < 7) return `${diffDays}d ago`

  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

type TabKey = 'all' | 'unread' | 'action_required'

function NotificationsPage() {
  const [notifications, setNotifications] = useState<Notification[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [totalCount, setTotalCount] = useState(0)
  const [selectedNotification, setSelectedNotification] = useState<Notification | null>(null)
  const [activeTab, setActiveTab] = useState<TabKey>('all')

  const { isOpen: isDetailOpen, onOpen: onDetailOpen, onClose: onDetailClose } = useDisclosure()

  const fetchNotifications = useCallback(async () => {
    setIsLoading(true)

    const filter: Record<string, boolean | undefined> = {}
    if (activeTab === 'unread') {
      filter.unreadOnly = true
    } else if (activeTab === 'action_required') {
      filter.unresolvedOnly = true
    }

    try {
      const result = await graphqlClient
        .query<{ notifications: PaginatedNotifications }>(NOTIFICATIONS_QUERY, {
          filter: Object.keys(filter).length > 0 ? filter : undefined,
          limit: 50,
          offset: 0,
        })
        .toPromise()

      if (result.data?.notifications) {
        setNotifications(result.data.notifications.notifications)
        setTotalCount(result.data.notifications.totalCount)
      }
    } catch (error) {
      console.error('Failed to fetch notifications:', error)
      addToast({
        title: 'Error',
        description: 'Failed to fetch notifications',
        color: 'danger',
      })
    } finally {
      setIsLoading(false)
    }
  }, [activeTab])

  useEffect(() => {
    fetchNotifications()
  }, [fetchNotifications])

  const handleMarkRead = async (id: string) => {
    try {
      await graphqlClient
        .mutation(MARK_NOTIFICATION_READ_MUTATION, { id })
        .toPromise()

      setNotifications((prev) =>
        prev.map((n) => (n.id === id ? { ...n, readAt: new Date().toISOString() } : n))
      )
    } catch (error) {
      addToast({ title: 'Error', description: 'Failed to mark notification as read', color: 'danger' })
    }
  }

  const handleMarkAllRead = async () => {
    try {
      const result = await graphqlClient
        .mutation<{ markAllNotificationsRead: { success: boolean; count: number } }>(
          MARK_ALL_NOTIFICATIONS_READ_MUTATION,
          {}
        )
        .toPromise()

      if (result.data?.markAllNotificationsRead.success) {
        setNotifications((prev) =>
          prev.map((n) => ({ ...n, readAt: n.readAt || new Date().toISOString() }))
        )
        addToast({
          title: 'Success',
          description: `Marked ${result.data.markAllNotificationsRead.count} notifications as read`,
          color: 'success',
        })
      }
    } catch (error) {
      addToast({ title: 'Error', description: 'Failed to mark all as read', color: 'danger' })
    }
  }

  const handleResolve = async (id: string, resolution: NotificationResolution) => {
    try {
      await graphqlClient
        .mutation(RESOLVE_NOTIFICATION_MUTATION, { input: { id, resolution } })
        .toPromise()

      setNotifications((prev) =>
        prev.map((n) =>
          n.id === id
            ? { ...n, resolvedAt: new Date().toISOString(), resolution, readAt: n.readAt || new Date().toISOString() }
            : n
        )
      )

      onDetailClose()

      addToast({
        title: 'Notification resolved',
        description: `Action: ${resolution.toLowerCase().replace('_', ' ')}`,
        color: 'success',
      })
    } catch (error) {
      addToast({ title: 'Error', description: 'Failed to resolve notification', color: 'danger' })
    }
  }

  const handleDelete = async (id: string) => {
    try {
      await graphqlClient
        .mutation(DELETE_NOTIFICATION_MUTATION, { id })
        .toPromise()

      setNotifications((prev) => prev.filter((n) => n.id !== id))

      addToast({ title: 'Notification deleted', color: 'success' })
    } catch (error) {
      addToast({ title: 'Error', description: 'Failed to delete notification', color: 'danger' })
    }
  }

  const handleViewDetails = (notification: Notification) => {
    setSelectedNotification(notification)
    onDetailOpen()
  }

  const columns: DataTableColumn<Notification>[] = [
    {
      key: 'type',
      label: 'Type',
      width: 140,
      render: (notification) => (
        <div className="flex items-center gap-2">
          {getNotificationIcon(notification.notificationType)}
          <Chip
            size="sm"
            variant="flat"
            color={NOTIFICATION_TYPE_INFO[notification.notificationType].color}
          >
            {NOTIFICATION_TYPE_INFO[notification.notificationType].label}
          </Chip>
        </div>
      ),
    },
    {
      key: 'title',
      label: 'Title',
      render: (notification) => (
        <div className="flex flex-col">
          <span className={`text-sm ${!notification.readAt ? 'font-semibold' : ''}`}>
            {notification.title}
          </span>
          <span className="text-xs text-default-500 line-clamp-1">{notification.message}</span>
        </div>
      ),
    },
    {
      key: 'category',
      label: 'Category',
      width: 120,
      render: (notification) => (
        <Chip size="sm" variant="flat">
          {CATEGORY_LABELS[notification.category]}
        </Chip>
      ),
    },
    {
      key: 'status',
      label: 'Status',
      width: 120,
      render: (notification) => {
        if (notification.resolvedAt && notification.resolution) {
          return (
            <Chip
              size="sm"
              variant="flat"
              color={
                notification.resolution === 'ACCEPTED'
                  ? 'success'
                  : notification.resolution === 'REJECTED'
                  ? 'danger'
                  : 'default'
              }
            >
              {notification.resolution.toLowerCase().replace('_', ' ')}
            </Chip>
          )
        }
        if (notification.readAt) {
          return (
            <Chip size="sm" variant="flat" color="default">
              Read
            </Chip>
          )
        }
        return (
          <Chip size="sm" variant="flat" color="primary">
            Unread
          </Chip>
        )
      },
    },
    {
      key: 'createdAt',
      label: 'Time',
      width: 100,
      render: (notification) => (
        <span className="text-sm text-default-500">{formatTimestamp(notification.createdAt)}</span>
      ),
    },
  ]

  const rowActions: RowAction<Notification>[] = [
    {
      key: 'view',
      label: 'View Details',
      onAction: handleViewDetails,
    },
    {
      key: 'markRead',
      label: 'Mark as Read',
      onAction: (notification: Notification) => handleMarkRead(notification.id),
      isDisabled: (notification: Notification) => !!notification.readAt,
    },
    {
      key: 'delete',
      label: 'Delete',
      color: 'danger',
      onAction: (notification: Notification) => handleDelete(notification.id),
    },
  ]

  const unreadCount = notifications.filter((n) => !n.readAt).length

  return (
    <div className="container mx-auto p-6 max-w-6xl">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Notifications</h1>
          <p className="text-default-500">
            {totalCount} total, {unreadCount} unread
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            variant="flat"
            startContent={<IconRefresh size={16} />}
            onPress={fetchNotifications}
            isLoading={isLoading}
          >
            Refresh
          </Button>
          {unreadCount > 0 && (
            <Button color="primary" variant="flat" onPress={handleMarkAllRead}>
              Mark All Read
            </Button>
          )}
        </div>
      </div>

      <Tabs
        selectedKey={activeTab}
        onSelectionChange={(key) => setActiveTab(key as TabKey)}
        className="mb-4"
      >
        <Tab key="all" title="All" />
        <Tab key="unread" title="Unread" />
        <Tab key="action_required" title="Action Required" />
      </Tabs>

      {isLoading ? (
        <div className="space-y-2">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-16 w-full rounded-lg" />
          ))}
        </div>
      ) : (
        <DataTable
          data={notifications}
          columns={columns}
          rowActions={rowActions}
          emptyContent={<div className="text-center text-default-500 py-8">No notifications</div>}
          getRowKey={(notification) => notification.id}
        />
      )}

      {/* Notification Detail Modal */}
      <Modal isOpen={isDetailOpen} onClose={onDetailClose} size="lg">
        <ModalContent>
          {selectedNotification && (
            <>
              <ModalHeader className="flex items-center gap-3">
                {getNotificationIcon(selectedNotification.notificationType)}
                <span>{selectedNotification.title}</span>
              </ModalHeader>
              <ModalBody>
                <div className="space-y-4">
                  <div className="flex gap-2">
                    <Chip
                      size="sm"
                      variant="flat"
                      color={NOTIFICATION_TYPE_INFO[selectedNotification.notificationType].color}
                    >
                      {NOTIFICATION_TYPE_INFO[selectedNotification.notificationType].label}
                    </Chip>
                    <Chip size="sm" variant="flat">
                      {CATEGORY_LABELS[selectedNotification.category]}
                    </Chip>
                  </div>

                  <p className="text-default-700">{selectedNotification.message}</p>

                  <div className="text-sm text-default-500">
                    <p>Created: {new Date(selectedNotification.createdAt).toLocaleString()}</p>
                    {selectedNotification.readAt && (
                      <p>Read: {new Date(selectedNotification.readAt).toLocaleString()}</p>
                    )}
                    {selectedNotification.resolvedAt && (
                      <p>
                        Resolved: {new Date(selectedNotification.resolvedAt).toLocaleString()} (
                        {selectedNotification.resolution?.toLowerCase().replace('_', ' ')})
                      </p>
                    )}
                  </div>

                  {selectedNotification.actionData && (
                    <div className="bg-default-100 rounded-lg p-4">
                      <p className="text-sm font-semibold mb-2">Action Details</p>
                      <pre className="text-xs overflow-auto">
                        {JSON.stringify(selectedNotification.actionData, null, 2)}
                      </pre>
                    </div>
                  )}
                </div>
              </ModalBody>
              <ModalFooter>
                {selectedNotification.notificationType === 'ACTION_REQUIRED' &&
                  !selectedNotification.resolvedAt && (
                    <>
                      <Button
                        color="success"
                        startContent={<IconCheck size={16} />}
                        onPress={() => handleResolve(selectedNotification.id, 'ACCEPTED')}
                      >
                        Accept
                      </Button>
                      <Button
                        color="danger"
                        variant="flat"
                        startContent={<IconX size={16} />}
                        onPress={() => handleResolve(selectedNotification.id, 'REJECTED')}
                      >
                        Reject
                      </Button>
                    </>
                  )}
                <Button
                  color="danger"
                  variant="light"
                  startContent={<IconTrash size={16} />}
                  onPress={() => {
                    handleDelete(selectedNotification.id)
                    onDetailClose()
                  }}
                >
                  Delete
                </Button>
                <Button variant="flat" onPress={onDetailClose}>
                  Close
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </div>
  )
}
