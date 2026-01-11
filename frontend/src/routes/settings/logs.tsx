import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useMemo, useRef } from 'react'
import { Button } from '@heroui/button'
import { Chip } from '@heroui/chip'
import { Skeleton } from '@heroui/skeleton'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter, useDisclosure } from '@heroui/modal'
import { Tooltip } from '@heroui/tooltip'
import { addToast } from '@heroui/toast'
import { Code } from '@heroui/code'
import { Select, SelectItem } from '@heroui/select'
import { Switch } from '@heroui/switch'
import { ConfirmModal } from '../../components/ConfirmModal'
import {
  graphqlClient,
  LOGS_QUERY,
  LOG_TARGETS_QUERY,
  CLEAR_ALL_LOGS_MUTATION,
  CLEAR_OLD_LOGS_MUTATION,
  LOG_EVENTS_SUBSCRIPTION,
  type LogEntry,
  type LogLevel,
  type PaginatedLogResult,
  type ClearLogsResult,
} from '../../lib/graphql'
import { sanitizeError } from '../../lib/format'
import {
  DataTable,
  type DataTableColumn,
  type DataTableFilter,
  type FilterOption,
  type RowAction,
} from '../../components/data-table'
import { ViewIcon, RefreshIcon } from '../../components/icons'

export const Route = createFileRoute('/settings/logs')({
  component: LogsSettingsPage,
})

// Log level colors and labels
const LOG_LEVEL_INFO: Record<LogLevel, { color: 'default' | 'primary' | 'success' | 'warning' | 'danger', label: string }> = {
  TRACE: { color: 'default', label: 'Trace' },
  DEBUG: { color: 'default', label: 'Debug' },
  INFO: { color: 'primary', label: 'Info' },
  WARN: { color: 'warning', label: 'Warn' },
  ERROR: { color: 'danger', label: 'Error' },
}

// Format timestamp to relative time or date
function formatTimestamp(isoString: string): string {
  const date = new Date(isoString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffSecs = Math.floor(diffMs / 1000)
  const diffMins = Math.floor(diffSecs / 60)
  const diffHours = Math.floor(diffMins / 60)

  if (diffSecs < 60) return 'just now'
  if (diffMins < 60) return `${diffMins}m ago`
  if (diffHours < 24) return `${diffHours}h ago`

  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

// Simplify target path for display
function simplifyTarget(target: string): string {
  const parts = target.split('::')
  if (parts.length <= 2) return target
  // Keep last 2 parts
  return parts.slice(-2).join('::')
}

// Live event from subscription (may have different field names)
interface LiveLogEvent {
  timestamp: string
  level: LogLevel
  target: string
  message: string
  fields?: Record<string, unknown>
  spanName?: string
}

function LogsSettingsPage() {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isLoadingMore, setIsLoadingMore] = useState(false)
  const [totalCount, setTotalCount] = useState(0)
  const [hasMore, setHasMore] = useState(true)
  const [selectedLog, setSelectedLog] = useState<LogEntry | null>(null)
  const { isOpen: isDetailOpen, onOpen: onDetailOpen, onClose: onDetailClose } = useDisclosure()

  // Confirm modal state
  const { isOpen: isConfirmOpen, onOpen: onConfirmOpen, onClose: onConfirmClose } = useDisclosure()
  const [confirmAction, setConfirmAction] = useState<{
    title: string
    message: string
    onConfirm: () => Promise<void>
  } | null>(null)

  // Live feed state
  const [isLiveFeedEnabled, setIsLiveFeedEnabled] = useState(true)
  const [liveEventCount, setLiveEventCount] = useState(0)

  // Source filter
  const [sources, setSources] = useState<string[]>([])
  const [selectedSource, setSelectedSource] = useState<string>('')

  // Pagination
  const pageSize = 50
  const offsetRef = useRef(0)

  // Fetch available sources/targets
  const fetchSources = useCallback(async () => {
    try {
      const result = await graphqlClient
        .query<{ logTargets: string[] }>(LOG_TARGETS_QUERY, { limit: 100 })
        .toPromise()

      if (result.data?.logTargets) {
        setSources(result.data.logTargets)
      }
    } catch (e) {
      console.error('Failed to fetch log sources:', e)
    }
  }, [])

  const fetchLogs = useCallback(async (reset = true) => {
    try {
      if (reset) {
        setIsLoading(true)
        offsetRef.current = 0
      } else {
        setIsLoadingMore(true)
      }

      const filter: { levels?: LogLevel[]; targets?: string[] } = {}
      if (selectedSource) {
        filter.targets = [selectedSource]
      }

      const result = await graphqlClient
        .query<{ logs: PaginatedLogResult }>(LOGS_QUERY, {
          filter: Object.keys(filter).length > 0 ? filter : null,
          limit: pageSize,
          offset: offsetRef.current,
        })
        .toPromise()

      if (result.data?.logs) {
        const newLogs = result.data.logs.logs
        if (reset) {
          setLogs(newLogs)
        } else {
          setLogs(prev => [...prev, ...newLogs])
        }
        setTotalCount(result.data.logs.totalCount)
        setHasMore(result.data.logs.hasMore)
        offsetRef.current += newLogs.length
      }
      if (result.error) {
        addToast({
          title: 'Error',
          description: sanitizeError(result.error),
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
      setIsLoading(false)
      setIsLoadingMore(false)
    }
  }, [selectedSource])

  // Load more for infinite scroll
  const loadMore = useCallback(() => {
    if (!isLoadingMore && hasMore) {
      fetchLogs(false)
    }
  }, [fetchLogs, isLoadingMore, hasMore])

  // Initial load
  useEffect(() => {
    fetchSources()
    fetchLogs(true)
  }, [fetchLogs, fetchSources])

  // Re-fetch when source filter changes
  useEffect(() => {
    fetchLogs(true)
  }, [selectedSource]) // eslint-disable-line react-hooks/exhaustive-deps

  // Subscribe to live log events
  useEffect(() => {
    if (!isLiveFeedEnabled) return

    const subscription = graphqlClient.subscription<{ logEvents: LiveLogEvent }>(
      LOG_EVENTS_SUBSCRIPTION,
      { levels: null }
    ).subscribe({
      next: (result) => {
        if (result.data?.logEvents) {
          const event = result.data.logEvents
          
          // Filter by source if selected
          if (selectedSource && event.target !== selectedSource && !event.target.includes(selectedSource)) {
            return
          }

          // Create a log entry from the live event
          const newLog: LogEntry = {
            id: `live-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
            timestamp: event.timestamp,
            level: event.level,
            target: event.target,
            message: event.message,
            fields: event.fields || null,
            spanName: event.spanName || null,
          }

          // Prepend to logs (most recent first)
          setLogs(prev => [newLog, ...prev.slice(0, 499)]) // Keep max 500 live
          setLiveEventCount(prev => prev + 1)
          setTotalCount(prev => prev + 1)
        }
      },
    })

    return () => {
      subscription.unsubscribe()
    }
  }, [isLiveFeedEnabled, selectedSource])

  // Handle clear all logs
  const handleClearAll = () => {
    setConfirmAction({
      title: 'Clear All Logs',
      message: 'Are you sure you want to delete ALL logs? This cannot be undone.',
      onConfirm: async () => {
        try {
          const result = await graphqlClient
            .mutation<{ clearAllLogs: ClearLogsResult }>(CLEAR_ALL_LOGS_MUTATION, {})
            .toPromise()

          if (result.data?.clearAllLogs.success) {
            addToast({
              title: 'Logs Cleared',
              description: `Deleted ${result.data.clearAllLogs.deletedCount} logs`,
              color: 'success',
            })
            setLogs([])
            setTotalCount(0)
            setLiveEventCount(0)
          } else {
            addToast({
              title: 'Error',
              description: result.data?.clearAllLogs.error || 'Failed to clear logs',
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
        onConfirmClose()
      },
    })
    onConfirmOpen()
  }

  // Handle clear old logs
  const handleClearOld = async (days: number) => {
    try {
      const result = await graphqlClient
        .mutation<{ clearOldLogs: ClearLogsResult }>(CLEAR_OLD_LOGS_MUTATION, { days })
        .toPromise()

      if (result.data?.clearOldLogs.success) {
        addToast({
          title: 'Old Logs Cleared',
          description: `Deleted ${result.data.clearOldLogs.deletedCount} logs older than ${days} days`,
          color: 'success',
        })
        fetchLogs(true)
      } else {
        addToast({
          title: 'Error',
          description: result.data?.clearOldLogs.error || 'Failed to clear old logs',
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

  // View log details
  const handleViewLog = (log: LogEntry) => {
    setSelectedLog(log)
    onDetailOpen()
  }

  // Calculate level counts for filter badges
  const levelCounts = useMemo(() => {
    const counts: Record<string, number> = {}
    for (const log of logs) {
      counts[log.level] = (counts[log.level] || 0) + 1
    }
    return counts
  }, [logs])

  // Column definitions with skeleton support
  const columns: DataTableColumn<LogEntry>[] = useMemo(
    () => [
      {
        key: 'timestamp',
        label: 'TIME',
        width: { width: 100, minWidth: 80 },
        sortable: true,
        truncate: false, // Don't truncate time - use whitespace-nowrap instead
        skeleton: () => <Skeleton className="w-16 h-4 rounded" />,
        render: (log) => (
          <Tooltip content={new Date(log.timestamp).toLocaleString()}>
            <span className="text-xs text-default-500 whitespace-nowrap">
              {formatTimestamp(log.timestamp)}
            </span>
          </Tooltip>
        ),
        sortFn: (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime(),
      },
      {
        key: 'level',
        label: 'LEVEL',
        width: { width: 80, minWidth: 70 },
        sortable: true,
        truncate: false,
        skeleton: () => <Skeleton className="w-14 h-5 rounded-full" />,
        render: (log) => (
          <Chip
            size="sm"
            color={LOG_LEVEL_INFO[log.level]?.color || 'default'}
            variant="flat"
            className="text-xs"
          >
            {LOG_LEVEL_INFO[log.level]?.label || log.level}
          </Chip>
        ),
        sortFn: (a, b) => {
          const order = ['ERROR', 'WARN', 'INFO', 'DEBUG', 'TRACE']
          return order.indexOf(a.level) - order.indexOf(b.level)
        },
      },
      {
        key: 'target',
        label: 'SOURCE',
        width: { width: 150, minWidth: 100, resizable: true },
        sortable: true,
        skeleton: () => <Skeleton className="w-24 h-4 rounded" />,
        render: (log) => (
          <Tooltip content={log.target}>
            <span className="text-xs text-default-400 font-mono">
              {simplifyTarget(log.target)}
            </span>
          </Tooltip>
        ),
        sortFn: (a, b) => a.target.localeCompare(b.target),
      },
      {
        key: 'message',
        label: 'MESSAGE',
        // No width specified - will grow to fill remaining space
        // Truncation is now automatic via the DataTable component
        sortable: true,
        skeleton: () => <Skeleton className="w-full h-4 rounded" />,
        render: (log) => log.message,
        sortFn: (a, b) => a.message.localeCompare(b.message),
      },
    ],
    []
  )

  // Filter options with counts
  const levelFilterOptions: FilterOption[] = useMemo(
    () => [
      { key: 'ERROR', label: 'Error', icon: '🔴', color: 'danger', count: levelCounts['ERROR'] || 0 },
      { key: 'WARN', label: 'Warn', icon: '🟡', color: 'warning', count: levelCounts['WARN'] || 0 },
      { key: 'INFO', label: 'Info', icon: '🔵', color: 'primary', count: levelCounts['INFO'] || 0 },
      { key: 'DEBUG', label: 'Debug', icon: '⚪', color: 'default', count: levelCounts['DEBUG'] || 0 },
      { key: 'TRACE', label: 'Trace', icon: '⚪', color: 'default', count: levelCounts['TRACE'] || 0 },
    ],
    [levelCounts]
  )

  // Filter definitions
  const filters: DataTableFilter<LogEntry>[] = useMemo(
    () => [
      {
        key: 'level',
        label: 'Level',
        type: 'select',
        options: levelFilterOptions,
        filterFn: (log, value) => {
          if (!value) return true
          return log.level === value
        },
        position: 'toolbar',
      },
    ],
    [levelFilterOptions]
  )

  // Row actions - view icon button
  const rowActions: RowAction<LogEntry>[] = useMemo(
    () => [
      {
        key: 'view',
        label: 'View Details',
        icon: <ViewIcon />,
        inDropdown: false,
        isVisible: (log) => !!log.fields && Object.keys(log.fields).length > 0,
        onAction: handleViewLog,
      },
    ],
    []
  )

  // Search function
  const searchFn = (log: LogEntry, term: string) => {
    const lowerTerm = term.toLowerCase()
    return (
      log.message.toLowerCase().includes(lowerTerm) ||
      log.target.toLowerCase().includes(lowerTerm) ||
      (log.spanName?.toLowerCase().includes(lowerTerm) || false)
    )
  }

  // Filter row content - Source filter alongside level filter chips
  const filterRowContent = (
    <Select
      size="sm"
      placeholder="All Sources"
      aria-label="Filter by source"
      className="w-52"
      selectedKeys={selectedSource ? [selectedSource] : []}
      onSelectionChange={(keys) => {
        const selected = Array.from(keys)[0] as string
        setSelectedSource(selected || '')
      }}
    >
      {sources.map((source) => (
        <SelectItem key={source}>
          {simplifyTarget(source)}
        </SelectItem>
      ))}
    </Select>
  )

  // Toolbar content - actions on the right side of the search bar
  const toolbarContent = (
    <div className="flex items-center gap-2">
      {/* Live Feed Toggle */}
      <div className="flex items-center gap-2 mr-2">
        <Switch
          size="sm"
          isSelected={isLiveFeedEnabled}
          onValueChange={setIsLiveFeedEnabled}
          color="success"
        />
        <span className="text-sm text-default-600">
          {isLiveFeedEnabled ? (
            <span className="flex items-center gap-1">
              <span className="w-2 h-2 bg-success rounded-full animate-pulse" />
              Live
            </span>
          ) : (
            'Paused'
          )}
        </span>
      </div>

      <Tooltip content="Refresh">
        <Button isIconOnly variant="flat" size="sm" onPress={() => fetchLogs(true)}>
          <RefreshIcon />
        </Button>
      </Tooltip>
      <Button variant="flat" color="warning" size="sm" onPress={() => handleClearOld(7)}>
        Clear 7+ days
      </Button>
      <Button variant="flat" color="danger" size="sm" onPress={handleClearAll}>
        Clear All
      </Button>
    </div>
  )

  // Header content - title above the toolbar
  const headerContent = (
    <div className="mb-4">
      <h2 className="text-xl font-semibold">Application Logs</h2>
      <p className="text-default-500 text-sm">
        {totalCount > 0 ? `${totalCount.toLocaleString()} total logs` : 'View system activity and errors'}
        {liveEventCount > 0 && isLiveFeedEnabled && (
          <span className="ml-2 text-success">
            (+{liveEventCount} live)
          </span>
        )}
      </p>
    </div>
  )

  return (
    <>
      {/* Logs Table */}
      <DataTable
        stateKey="settings-logs"
        data={logs}
        columns={columns}
        getRowKey={(log) => log.id}
        isLoading={isLoading}
        skeletonRowCount={15}
        selectionMode="multiple"
        filters={filters}
        searchFn={searchFn}
        searchPlaceholder="Search logs..."
        defaultSortColumn="timestamp"
        defaultSortDirection="desc"
        rowActions={rowActions}
        isCompact
        fillHeight={true}
        showItemCount
        ariaLabel="Application logs"
        headerContent={headerContent}
        toolbarContent={toolbarContent}
        toolbarContentPosition="end"
        filterRowContent={filterRowContent}
        paginationMode="infinite"
        onLoadMore={loadMore}
        hasMore={hasMore}
        isLoadingMore={isLoadingMore}
      />

      {/* Log Detail Modal */}
      <Modal isOpen={isDetailOpen} onClose={onDetailClose} size="2xl">
        <ModalContent>
          <ModalHeader className="flex flex-col gap-1">
            <div className="flex items-center gap-2">
              <Chip
                size="sm"
                color={LOG_LEVEL_INFO[selectedLog?.level || 'INFO']?.color || 'default'}
                variant="flat"
              >
                {LOG_LEVEL_INFO[selectedLog?.level || 'INFO']?.label}
              </Chip>
              <span className="text-default-500 text-sm">
                {selectedLog && new Date(selectedLog.timestamp).toLocaleString()}
              </span>
            </div>
          </ModalHeader>
          <ModalBody>
            {selectedLog && (
              <div className="space-y-4">
                {/* Message */}
                <div>
                  <label className="text-xs text-default-400 uppercase font-medium">Message</label>
                  <p className="mt-1 text-sm">{selectedLog.message}</p>
                </div>

                {/* Source */}
                <div>
                  <label className="text-xs text-default-400 uppercase font-medium">Source</label>
                  <p className="mt-1 text-xs font-mono text-default-500">{selectedLog.target}</p>
                </div>

                {/* Span */}
                {selectedLog.spanName && (
                  <div>
                    <label className="text-xs text-default-400 uppercase font-medium">Span</label>
                    <p className="mt-1 text-sm">{selectedLog.spanName}</p>
                  </div>
                )}

                {/* Fields */}
                {selectedLog.fields && Object.keys(selectedLog.fields).length > 0 && (
                  <div>
                    <label className="text-xs text-default-400 uppercase font-medium mb-2 block">
                      Event Data
                    </label>
                    <div className="bg-content2 rounded-lg p-4 overflow-x-auto">
                      <Code className="text-xs block whitespace-pre-wrap">
                        {JSON.stringify(selectedLog.fields, null, 2)}
                      </Code>
                    </div>
                  </div>
                )}
              </div>
            )}
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onDetailClose}>
              Close
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>

      {/* Confirm Modal */}
      <ConfirmModal
        isOpen={isConfirmOpen}
        onClose={onConfirmClose}
        onConfirm={() => confirmAction?.onConfirm()}
        title={confirmAction?.title || 'Confirm'}
        message={confirmAction?.message || ''}
        confirmLabel="Delete"
        confirmColor="danger"
      />
    </>
  )
}
