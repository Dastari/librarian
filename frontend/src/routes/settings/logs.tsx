import { createFileRoute } from "@tanstack/react-router";
import {
  useState,
  useEffect,
  useCallback,
  useMemo,
  useRef,
  type ReactNode,
} from "react";
import { useQueryState, parseAsString, parseAsStringLiteral } from "nuqs";
import { Button, ButtonGroup } from "@heroui/button";
import { Chip } from "@heroui/chip";
import { Skeleton } from "@heroui/skeleton";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  useDisclosure,
} from "@heroui/modal";
import { Tooltip } from "@heroui/tooltip";
import { addToast } from "@heroui/toast";
import { Code } from "@heroui/code";
import { Select, SelectItem } from "@heroui/select";
import { Switch } from "@heroui/switch";
import { ConfirmModal } from "../../components/ConfirmModal";
import { graphqlClient } from "../../lib/graphql";
import {
  AppLogsDocument,
  AppLogChangedDocument,
  DeleteAppLogsDocument,
  ChangeAction,
  SortDirection,
  type AppLogWhereInput,
} from "../../lib/graphql/generated/graphql";
import { sanitizeError } from "../../lib/format";
import {
  DataTable,
  type DataTableColumn,
  type RowAction,
} from "../../components/data-table";
import {
  IconEye,
  IconRefresh,
  IconFilter,
  IconCopy,
} from "@tabler/icons-react";

export const Route = createFileRoute("/settings/logs")({
  component: LogsSettingsPage,
});

// Local types for UI (map from AppLog PascalCase)
type LogLevel = "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR";
interface LogEntry {
  id: string;
  timestamp: string;
  level: LogLevel;
  target: string;
  message: string;
  fields: Record<string, unknown> | null;
  spanName: string | null;
}

function appLogNodeToEntry(node: {
  Id: string;
  Timestamp: string;
  Level: string;
  Target: string;
  Message: string;
  Fields?: string | null;
  SpanName?: string | null;
}): LogEntry {
  let fields: Record<string, unknown> | null = null;
  if (node.Fields) {
    try {
      fields = JSON.parse(node.Fields) as Record<string, unknown>;
    } catch {
      fields = null;
    }
  }
  return {
    id: node.Id,
    timestamp: node.Timestamp,
    level: node.Level as LogLevel,
    target: node.Target,
    message: node.Message,
    fields,
    spanName: node.SpanName ?? null,
  };
}

// Log level colors and labels
const LOG_LEVEL_INFO: Record<
  LogLevel,
  {
    color: "default" | "primary" | "success" | "warning" | "danger";
    label: string;
  }
> = {
  TRACE: { color: "default", label: "Trace" },
  DEBUG: { color: "default", label: "Debug" },
  INFO: { color: "primary", label: "Info" },
  WARN: { color: "warning", label: "Warn" },
  ERROR: { color: "danger", label: "Error" },
};

// Format timestamp to relative time or date
function formatTimestamp(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSecs = Math.floor(diffMs / 1000);
  const diffMins = Math.floor(diffSecs / 60);
  const diffHours = Math.floor(diffMins / 60);

  if (diffSecs < 60) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;

  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

// Simplify target path for display
function simplifyTarget(target: string): string {
  const parts = target.split("::");
  if (parts.length <= 2) return target;
  // Keep last 2 parts
  return parts.slice(-2).join("::");
}

function LogsSettingsPage() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [totalCount, setTotalCount] = useState(0);
  const [hasMore, setHasMore] = useState(true);
  const [selectedLog, setSelectedLog] = useState<LogEntry | null>(null);
  const {
    isOpen: isDetailOpen,
    onOpen: onDetailOpen,
    onClose: onDetailClose,
  } = useDisclosure();

  // Live feed state
  const [isLiveFeedEnabled, setIsLiveFeedEnabled] = useState(true);
  const [liveEventCount, setLiveEventCount] = useState(0);

  // Confirm modal for clear-all
  const {
    isOpen: isConfirmOpen,
    onOpen: onConfirmOpen,
    onClose: onConfirmClose,
  } = useDisclosure();
  const [confirmAction, setConfirmAction] = useState<{
    title: string;
    message: string;
    onConfirm: () => Promise<void>;
  } | null>(null);

  // Source filter - persisted in URL via nuqs
  const [sources, setSources] = useState<string[]>([]);
  const [selectedSource, setSelectedSource] = useQueryState(
    "source",
    parseAsString.withDefault(""),
  );

  // Level filter - persisted in URL via nuqs
  const [levelFilter, setLevelFilter] = useQueryState(
    "level",
    parseAsStringLiteral([
      "TRACE",
      "DEBUG",
      "INFO",
      "WARN",
      "ERROR",
    ] as const).withDefault(null as unknown as "INFO"),
  );
  // Convert to LogLevel | null (nuqs returns the literal type)
  const normalizedLevelFilter: LogLevel | null = levelFilter as LogLevel | null;

  // Sort state - persisted in URL via nuqs
  const [sortColumn, setSortColumn] = useQueryState(
    "sort",
    parseAsStringLiteral(["timestamp", "level", "target"] as const).withDefault(
      "timestamp",
    ),
  );
  const [sortDirection, setSortDirection] = useQueryState(
    "order",
    parseAsStringLiteral(["asc", "desc"] as const).withDefault("desc"),
  );

  // Pagination
  const pageSize = 50;
  const offsetRef = useRef(0);

  // Create a ref to track if this is the initial mount
  const isInitialMount = useRef(true);

  // Refs to store current values for use in fetchLogs (avoids stale closures)
  const selectedSourceRef = useRef(selectedSource);
  const sortColumnRef = useRef(sortColumn);
  const sortDirectionRef = useRef(sortDirection);

  useEffect(() => {
    selectedSourceRef.current = selectedSource;
  }, [selectedSource]);

  useEffect(() => {
    sortColumnRef.current = sortColumn;
    sortDirectionRef.current = sortDirection;
  }, [sortColumn, sortDirection]);

  // Fetch logs using AppLogs query
  const fetchLogsWithCurrentSource = useCallback(async (reset = true) => {
    try {
      if (reset) {
        setIsLoading(true);
        offsetRef.current = 0;
      } else {
        setIsLoadingMore(true);
      }

      const currentSource = selectedSourceRef.current;
      const where: AppLogWhereInput | undefined = currentSource
        ? { Target: { Eq: currentSource } }
        : undefined;

      const currentSortColumn = sortColumnRef.current;
      const currentSortDirection = sortDirectionRef.current;
      const orderBy =
        currentSortColumn === "timestamp"
          ? [{ Timestamp: currentSortDirection === "desc" ? SortDirection.Desc : SortDirection.Asc }]
          : currentSortColumn === "level"
            ? [{ Level: currentSortDirection === "desc" ? SortDirection.Desc : SortDirection.Asc }]
            : [{ Target: currentSortDirection === "desc" ? SortDirection.Desc : SortDirection.Asc }];

      const result = await graphqlClient
        .query(AppLogsDocument, {
          Where: where,
          OrderBy: orderBy,
          Page: { Limit: pageSize, Offset: offsetRef.current },
        })
        .toPromise();

      if (result.data?.AppLogs) {
        const connection = result.data.AppLogs;
        const nodes = connection.Edges.map((e) => e.Node);
        const newEntries = nodes.map(appLogNodeToEntry);
        if (reset) {
          setLogs(newEntries);
          setSources((prev) => {
            const merged = new Set(prev);
            nodes.forEach((n) => merged.add(n.Target));
            return Array.from(merged).sort();
          });
        } else {
          setLogs((prev) => [...prev, ...newEntries]);
        }
        setTotalCount(connection.PageInfo.TotalCount ?? 0);
        setHasMore(connection.PageInfo.HasNextPage);
        offsetRef.current += newEntries.length;
      }
      if (result.error) {
        const isAuthError = result.error.message
          ?.toLowerCase()
          .includes("authentication");
        if (!isAuthError) {
          addToast({
            title: "Error",
            description: sanitizeError(result.error),
            color: "danger",
          });
        }
      }
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      if (!errorMsg.toLowerCase().includes("authentication")) {
        addToast({
          title: "Error",
          description: sanitizeError(e),
          color: "danger",
        });
      }
    } finally {
      setIsLoading(false);
      setIsLoadingMore(false);
    }
  }, []);

  // Load more for infinite scroll
  const loadMore = useCallback(() => {
    if (!isLoadingMore && hasMore) {
      fetchLogsWithCurrentSource(false);
    }
  }, [fetchLogsWithCurrentSource, isLoadingMore, hasMore]);

  // Initial load
  useEffect(() => {
    fetchLogsWithCurrentSource(true);
  }, [fetchLogsWithCurrentSource]);

  // Re-fetch when source filter or sort changes (skip initial mount)
  useEffect(() => {
    if (isInitialMount.current) {
      isInitialMount.current = false;
      return;
    }
    fetchLogsWithCurrentSource(true);
  }, [selectedSource, sortColumn, sortDirection, fetchLogsWithCurrentSource]);

  // Subscribe to AppLogChanged for live updates
  useEffect(() => {
    if (!isLiveFeedEnabled) return;

    const subscription = graphqlClient
      .subscription(AppLogChangedDocument, {})
      .subscribe({
        next: (result) => {
          const event = result.data?.AppLogChanged;
          if (!event || event.Action !== ChangeAction.Created || !event.AppLog) return;

          const node = event.AppLog;

          // Filter by source if selected
          if (
            selectedSource &&
            node.Target !== selectedSource &&
            !node.Target.includes(selectedSource)
          ) {
            return;
          }

          const newLog = appLogNodeToEntry(node);

          setLogs((prev) => [newLog, ...prev.slice(0, 499)]);
          setLiveEventCount((prev) => prev + 1);
          setTotalCount((prev) => prev + 1);
          setSources((prev) =>
            prev.includes(node.Target) ? prev : [...prev, node.Target].sort(),
          );
        },
      });

    return () => {
      subscription.unsubscribe();
    };
  }, [isLiveFeedEnabled, selectedSource]);

  // Clear all logs (Where required: match all via Timestamp >= epoch)
  const handleClearAll = () => {
    setConfirmAction({
      title: "Clear All Logs",
      message:
        "Are you sure you want to delete ALL logs? This cannot be undone.",
      onConfirm: async () => {
        try {
          const result = await graphqlClient
            .mutation(DeleteAppLogsDocument, {
              Where: { Timestamp: { Gte: "1970-01-01T00:00:00.000Z" } },
            })
            .toPromise();
          const payload = result.data?.DeleteAppLogs;
          if (payload?.success) {
            addToast({
              title: "Logs Cleared",
              description: `Deleted ${payload.DeletedCount} logs`,
              color: "success",
            });
            setLogs([]);
            setTotalCount(0);
            setLiveEventCount(0);
          } else {
            addToast({
              title: "Error",
              description: sanitizeError(
                payload?.error ?? result.error?.message ?? "Failed to clear logs",
              ),
              color: "danger",
            });
          }
        } catch (e) {
          addToast({
            title: "Error",
            description: sanitizeError(e),
            color: "danger",
          });
        }
        onConfirmClose();
      },
    });
    onConfirmOpen();
  };

  // Clear logs older than N days
  const handleClearOld = async (days: number) => {
    const date = new Date();
    date.setDate(date.getDate() - days);
    const isoBefore = date.toISOString();
    try {
      const result = await graphqlClient
        .mutation(DeleteAppLogsDocument, {
          Where: { Timestamp: { Lt: isoBefore } },
        })
        .toPromise();
      const payload = result.data?.DeleteAppLogs;
      if (payload?.success) {
        addToast({
          title: "Old Logs Cleared",
          description: `Deleted ${payload.DeletedCount} logs older than ${days} days`,
          color: "success",
        });
        fetchLogsWithCurrentSource(true);
      } else {
        addToast({
          title: "Error",
          description: sanitizeError(
            payload?.error ?? result.error?.message ?? "Failed to clear old logs",
          ),
          color: "danger",
        });
      }
    } catch (e) {
      addToast({
        title: "Error",
        description: sanitizeError(e),
        color: "danger",
      });
    }
  };

  // View log details
  const handleViewLog = (log: LogEntry) => {
    setSelectedLog(log);
    onDetailOpen();
  };

  // Calculate level counts for filter badges
  const levelCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const log of logs) {
      counts[log.level] = (counts[log.level] || 0) + 1;
    }
    return counts;
  }, [logs]);

  // Filter logs by level
  const filteredLogs = useMemo(() => {
    if (!normalizedLevelFilter) return logs;
    return logs.filter((log) => log.level === normalizedLevelFilter);
  }, [logs, normalizedLevelFilter]);

  // Column definitions with skeleton support
  // Server-side sorting is now supported for timestamp, level, and target
  const columns: DataTableColumn<LogEntry>[] = useMemo(
    () => [
      {
        key: "timestamp",
        label: "TIME",
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
      },
      {
        key: "level",
        label: "LEVEL",
        width: { width: 80, minWidth: 70 },
        sortable: true,
        truncate: false,
        skeleton: () => <Skeleton className="w-14 h-5 rounded-full" />,
        render: (log) => (
          <Chip
            size="sm"
            color={LOG_LEVEL_INFO[log.level]?.color || "default"}
            variant="flat"
            className="text-xs"
          >
            {LOG_LEVEL_INFO[log.level]?.label || log.level}
          </Chip>
        ),
      },
      {
        key: "target",
        label: "SOURCE",
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
      },
      {
        key: "message",
        label: "MESSAGE",
        // No width specified - will grow to fill remaining space
        sortable: false, // Message is not sortable
        skeleton: () => <Skeleton className="w-full h-4 rounded" />,
        render: (log) => (
          <div className="flex items-center justify-between gap-2 group">
            <span className="flex-1 min-w-0">{log.message}</span>
            <Tooltip content="Copy message">
              <Button
                isIconOnly
                variant="light"
                size="sm"
                className="opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                onPress={() => {
                  navigator.clipboard.writeText(log.message);
                  addToast({
                    title: "Copied",
                    description: "Message copied to clipboard",
                    color: "success",
                  });
                }}
              >
                <IconCopy size={14} />
              </Button>
            </Tooltip>
          </div>
        ),
      },
    ],
    [],
  );

  // Handle sort change from DataTable
  const handleSortChange = useCallback(
    (column: string | null, direction: "asc" | "desc") => {
      if (column && ["timestamp", "level", "target"].includes(column)) {
        setSortColumn(column as "timestamp" | "level" | "target");
        setSortDirection(direction);
      }
    },
    [setSortColumn, setSortDirection],
  );

  // Level filter options
  const levelFilterOptions: {
    key: LogLevel;
    label: string;
    color: "danger" | "warning" | "primary" | "default";
  }[] = [
    { key: "ERROR", label: "Error", color: "danger" },
    { key: "WARN", label: "Warn", color: "warning" },
    { key: "INFO", label: "Info", color: "primary" },
    { key: "DEBUG", label: "Debug", color: "default" },
    { key: "TRACE", label: "Trace", color: "default" },
  ];

  // Row actions - view icon button
  const rowActions: RowAction<LogEntry>[] = useMemo(
    () => [
      {
        key: "view",
        label: "View Details",
        icon: <IconEye size={16} />,
        inDropdown: false,
        isVisible: (log) => !!log.fields && Object.keys(log.fields).length > 0,
        onAction: handleViewLog,
      },
    ],
    [],
  );

  // Search function
  const searchFn = (log: LogEntry, term: string) => {
    const lowerTerm = term.toLowerCase();
    return (
      log.message.toLowerCase().includes(lowerTerm) ||
      log.target.toLowerCase().includes(lowerTerm) ||
      log.spanName?.toLowerCase().includes(lowerTerm) ||
      false
    );
  };

  // Filter row content - Level filter chips and source dropdown
  const filterRowContent: ReactNode = useMemo(
    () => (
      <>
        <span className="text-sm text-default-500 flex items-center gap-1">
          <IconFilter size={16} /> Filter:
        </span>
        <ButtonGroup size="sm" variant="solid">
          <Button
            variant={normalizedLevelFilter === null ? "solid" : "flat"}
            color={normalizedLevelFilter === null ? "primary" : "default"}
            onPress={() => setLevelFilter(null)}
          >
            All
          </Button>
          {levelFilterOptions.map((option) => {
            const count = levelCounts[option.key] || 0;
            return (
              <Button
                key={option.key}
                variant={
                  normalizedLevelFilter === option.key ? "solid" : "flat"
                }
                color={
                  normalizedLevelFilter === option.key
                    ? option.color
                    : "default"
                }
                onPress={() =>
                  setLevelFilter(
                    normalizedLevelFilter === option.key ? null : option.key,
                  )
                }
                className="gap-1"
              >
                <span>{option.label}</span>
                {count > 0 && (
                  <Chip size="sm" variant="flat" className="ml-1">
                    {count}
                  </Chip>
                )}
              </Button>
            );
          })}
        </ButtonGroup>
        <Select
          size="sm"
          placeholder="All Sources"
          aria-label="Filter by source"
          className="w-52"
          selectedKeys={
            // Only set selected key if it exists in sources or is the special __all__ key
            selectedSource && sources.includes(selectedSource)
              ? [selectedSource]
              : ["__all__"]
          }
          onSelectionChange={(keys) => {
            const selected = Array.from(keys)[0] as string;
            setSelectedSource(selected === "__all__" ? "" : selected || "");
          }}
        >
          {[
            <SelectItem key="__all__">All Sources</SelectItem>,
            ...sources.map((source) => (
              <SelectItem key={source}>{simplifyTarget(source)}</SelectItem>
            )),
          ]}
        </Select>
      </>
    ),
    [
      normalizedLevelFilter,
      levelCounts,
      selectedSource,
      sources,
      setLevelFilter,
      setSelectedSource,
    ],
  );

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
            "Paused"
          )}
        </span>
      </div>

      <Tooltip content="Refresh">
        <Button
          isIconOnly
          variant="flat"
          size="sm"
          onPress={() => fetchLogsWithCurrentSource(true)}
        >
          <IconRefresh size={16} />
        </Button>
      </Tooltip>
      <Button
        variant="flat"
        color="warning"
        size="sm"
        onPress={() => handleClearOld(7)}
      >
        Clear 7+ days
      </Button>
      <Button variant="flat" color="danger" size="sm" onPress={handleClearAll}>
        Clear All
      </Button>
    </div>
  );

  return (
    <div className="flex flex-col gap-6 h-full min-h-0">
      {/* Page Header */}
      <div className="flex items-center justify-between shrink-0">
        <div>
          <h2 className="text-xl font-semibold">System Logs</h2>
          <p className="text-default-500 text-sm">
            {totalCount > 0
              ? `${totalCount.toLocaleString()} total logs`
              : "View system activity and errors"}
            {liveEventCount > 0 && isLiveFeedEnabled && (
              <span className="ml-2 text-success">
                (+{liveEventCount} live)
              </span>
            )}
          </p>
        </div>
        {toolbarContent}
      </div>

      {/* Logs Table */}
      <DataTable
        stateKey="settings-logs"
        skeletonDelay={500}
        data={filteredLogs}
        columns={columns}
        getRowKey={(log) => log.id}
        isLoading={isLoading}
        skeletonRowCount={15}
        selectionMode="multiple"
        searchFn={searchFn}
        searchPlaceholder="Search logs..."
        rowActions={rowActions}
        isCompact
        fillHeight={true}
        showItemCount
        ariaLabel="Application logs"
        filterRowContent={filterRowContent}
        // Server-side mode with server-side sorting
        serverSide
        serverTotalCount={totalCount}
        paginationMode="infinite"
        onLoadMore={loadMore}
        hasMore={hasMore}
        isLoadingMore={isLoadingMore}
        // Controlled server-side sorting
        sortColumn={sortColumn}
        sortDirection={sortDirection}
        onSortChange={handleSortChange}
      />

      {/* Log Detail Modal */}
      <Modal isOpen={isDetailOpen} onClose={onDetailClose} size="2xl">
        <ModalContent>
          <ModalHeader className="flex flex-col gap-1">
            <div className="flex items-center gap-2">
              <Chip
                size="sm"
                color={
                  LOG_LEVEL_INFO[selectedLog?.level || "INFO"]?.color ||
                  "default"
                }
                variant="flat"
              >
                {LOG_LEVEL_INFO[selectedLog?.level || "INFO"]?.label}
              </Chip>
              <span className="text-default-500 text-sm">
                {selectedLog &&
                  new Date(selectedLog.timestamp).toLocaleString()}
              </span>
            </div>
          </ModalHeader>
          <ModalBody>
            {selectedLog && (
              <div className="space-y-4">
                {/* Message */}
                <div>
                  <label className="text-xs text-default-400 uppercase font-medium">
                    Message
                  </label>
                  <p className="mt-1 text-sm">{selectedLog.message}</p>
                </div>

                {/* Source */}
                <div>
                  <label className="text-xs text-default-400 uppercase font-medium">
                    Source
                  </label>
                  <p className="mt-1 text-xs font-mono text-default-500">
                    {selectedLog.target}
                  </p>
                </div>

                {/* Span */}
                {selectedLog.spanName && (
                  <div>
                    <label className="text-xs text-default-400 uppercase font-medium">
                      Span
                    </label>
                    <p className="mt-1 text-sm">{selectedLog.spanName}</p>
                  </div>
                )}

                {/* Fields */}
                {selectedLog.fields &&
                  Object.keys(selectedLog.fields).length > 0 && (
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

      <ConfirmModal
        isOpen={isConfirmOpen}
        onClose={onConfirmClose}
        onConfirm={() => confirmAction?.onConfirm()}
        title={confirmAction?.title ?? "Confirm"}
        message={confirmAction?.message ?? ""}
        confirmLabel="Delete"
        confirmColor="danger"
      />
    </div>
  );
}
