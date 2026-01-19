import { useState, useEffect, useCallback, useMemo } from 'react'
import { Button } from '@heroui/button'
import { Input } from '@heroui/input'
import { Divider } from '@heroui/divider'
import { Chip } from '@heroui/chip'
import { Skeleton } from '@heroui/skeleton'
import { addToast } from '@heroui/toast'
import { useDisclosure } from '@heroui/modal'
import { ConfirmModal } from '../ConfirmModal'
import { FilePropertiesModal } from '../FilePropertiesModal'
import {
  DataTable,
  type DataTableColumn,
  type BulkAction,
  type RowAction,
} from '../data-table'
import {
  browseDirectory,
  graphqlClient,
  MEDIA_FILE_BY_PATH_QUERY,
  type FileEntry,
  type QuickPath,
  type MediaFile,
} from '../../lib/graphql'
import { sanitizeError } from '../../lib/format'
import { formatBytes } from '../../lib/format'
import { IconCopy, IconArrowRight, IconTrash, IconSearch, IconInfoCircle, IconRefresh, IconFolder, IconFolderOpen, IconMovie, IconFile, IconPhoto } from '@tabler/icons-react'

// ============================================================================
// Utility Functions
// ============================================================================

function getFileIcon(filename: string, isDir: boolean): React.ReactNode {
  if (isDir) return <IconFolder size={20} className="text-amber-400" />
  const ext = filename.split('.').pop()?.toLowerCase()
  switch (ext) {
    case 'mkv':
    case 'mp4':
    case 'avi':
    case 'mov':
    case 'wmv':
    case 'webm':
    case 'm4v':
      return <IconMovie size={20} className="text-purple-400" />
    case 'srt':
    case 'sub':
    case 'ass':
    case 'vtt':
      return <IconFile size={20} className="text-default-400" />
    case 'nfo':
    case 'txt':
      return <IconFile size={20} className="text-default-400" />
    case 'jpg':
    case 'jpeg':
    case 'png':
    case 'gif':
    case 'webp':
      return <IconPhoto size={20} className="text-green-400" />
    default:
      return <IconFile size={20} className="text-default-400" />
  }
}

// ============================================================================
// Component Props
// ============================================================================

interface LibraryFileBrowserTabProps {
  libraryPath: string
}

// ============================================================================
// Main Component
// ============================================================================

export function LibraryFileBrowserTab({ libraryPath }: LibraryFileBrowserTabProps) {
  const [currentPath, setCurrentPath] = useState(libraryPath)
  const [inputPath, setInputPath] = useState(libraryPath)
  const [entries, setEntries] = useState<FileEntry[]>([])
  const [quickPaths, setQuickPaths] = useState<QuickPath[]>([])
  const [parentPath, setParentPath] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  // Confirm modal state
  const { isOpen: isConfirmOpen, onOpen: onConfirmOpen, onClose: onConfirmClose } = useDisclosure()
  const [pathsToDelete, setPathsToDelete] = useState<string[]>([])

  // File properties modal state
  const { isOpen: isPropertiesOpen, onOpen: onPropertiesOpen, onClose: onPropertiesClose } = useDisclosure()
  const [propertiesMediaFileId, setPropertiesMediaFileId] = useState<string | null>(null)
  const [propertiesFileName, setPropertiesFileName] = useState<string | null>(null)

  const fetchDirectory = useCallback(async (path: string) => {
    try {
      setLoading(true)
      const data = await browseDirectory(path, false)
      setCurrentPath(data.currentPath)
      setInputPath(data.currentPath)
      setParentPath(data.parentPath)
      setEntries(data.entries || [])
      setQuickPaths(data.quickPaths || [])
    } catch (err) {
      console.error('Failed to browse directory:', err)
      addToast({
        title: 'Error',
        description: sanitizeError(err),
        color: 'danger',
      })
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchDirectory(libraryPath)
  }, [libraryPath, fetchDirectory])

  const navigateTo = (path: string) => {
    fetchDirectory(path)
  }

  const handlePathInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      navigateTo(inputPath)
    }
  }

  // Sort entries: directories first, then files, both alphabetically
  // Prepend parent directory as a synthetic entry if it exists
  const sortedEntries = useMemo(() => {
    const sorted = [...entries].sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1
      return a.name.localeCompare(b.name)
    })
    
    // Add parent directory as the first entry
    if (parentPath) {
      const parentEntry: FileEntry = {
        name: '..',
        path: parentPath,
        isDir: true,
        readable: true,
        writable: false,
        size: 0,
        sizeFormatted: '-',
        mimeType: null,
        modifiedAt: null,
      }
      return [parentEntry, ...sorted]
    }
    
    return sorted
  }, [entries, parentPath])

  // Action handlers
  const handleCopy = (paths: string[]) => {
    addToast({
      title: 'Copy',
      description: `Copying ${paths.length} item(s)... (not implemented)`,
      color: 'primary',
    })
  }

  const handleMove = (paths: string[]) => {
    addToast({
      title: 'Move',
      description: `Moving ${paths.length} item(s)... (not implemented)`,
      color: 'primary',
    })
  }

  const handleDeleteClick = (paths: string[]) => {
    setPathsToDelete(paths)
    onConfirmOpen()
  }

  const handleDelete = () => {
    addToast({
      title: 'Delete',
      description: `Deleting ${pathsToDelete.length} item(s)... (not implemented)`,
      color: 'danger',
    })
    onConfirmClose()
  }

  const handleMatch = (paths: string[]) => {
    addToast({
      title: 'Match',
      description: `Matching ${paths.length} item(s)... (not implemented)`,
      color: 'primary',
    })
  }

  // Check if file is a video file
  const isVideoFile = (filename: string): boolean => {
    const ext = filename.split('.').pop()?.toLowerCase()
    return ['mkv', 'mp4', 'avi', 'mov', 'wmv', 'webm', 'm4v', 'ts', 'm2ts', 'flv', 'mpg', 'mpeg'].includes(ext || '')
  }

  const handleProperties = async (entry: FileEntry) => {
    // For video files, try to show the detailed FFmpeg analysis
    if (!entry.isDir && isVideoFile(entry.name)) {
      // Look up the media file by path
      const result = await graphqlClient
        .query<{ mediaFileByPath: MediaFile | null }>(MEDIA_FILE_BY_PATH_QUERY, { path: entry.path })
        .toPromise()

      if (result.data?.mediaFileByPath) {
        // File is in the database, show detailed properties modal
        setPropertiesMediaFileId(result.data.mediaFileByPath.id)
        setPropertiesFileName(entry.name)
        onPropertiesOpen()
        return
      }
      // If not in database, fall through to basic toast
    }

    // For non-video files or files not in database, show basic info toast
    addToast({
      title: entry.name,
      description: `Path: ${entry.path}\nSize: ${formatBytes(entry.size)}\nType: ${entry.isDir ? 'Directory' : 'File'}`,
      color: 'default',
    })
  }

  // Column definitions
  const columns: DataTableColumn<FileEntry>[] = useMemo(
    () => [
      {
        key: 'name',
        label: 'NAME',
        sortable: true,
        render: (entry) => {
          const isParent = entry.name === '..'
          const icon = getFileIcon(entry.name, entry.isDir)
          
          return (
            <Button
              variant="light"
              onPress={() => entry.isDir && entry.readable && navigateTo(entry.path)}
              className={`
                flex items-center gap-3 text-left min-w-0 w-full justify-start px-2
                ${!entry.readable ? 'opacity-50' : ''}
                ${isParent ? 'text-default-500' : ''}
              `}
              isDisabled={!entry.isDir || !entry.readable}
            >
              <span className="flex-shrink-0">{icon}</span>
              <span className="flex-1 truncate">{entry.name}</span>
            </Button>
          )
        },
        skeleton: () => (
          <div className="flex items-center gap-3 px-2">
            <Skeleton className="w-5 h-5 rounded" />
            <Skeleton className="w-48 h-4 rounded" />
          </div>
        ),
        sortFn: (a, b) => {
          // Parent directory always first
          if (a.name === '..') return -1
          if (b.name === '..') return 1
          // Directories first
          if (a.isDir !== b.isDir) return a.isDir ? -1 : 1
          return a.name.localeCompare(b.name)
        },
      },
      {
        key: 'size',
        label: 'SIZE',
        width: 100,
        sortable: true,
        render: (entry) => (
          <span className="text-xs text-default-400 tabular-nums">
            {!entry.isDir ? formatBytes(entry.size) : '—'}
          </span>
        ),
        skeleton: () => <Skeleton className="w-12 h-3 rounded" />,
        sortFn: (a, b) => (a.size || 0) - (b.size || 0),
      },
      {
        key: 'type',
        label: 'TYPE',
        width: 100,
        sortable: true,
        render: (entry) => (
          <span className="text-xs text-default-400">
            {entry.isDir ? 'Folder' : entry.name.split('.').pop()?.toUpperCase() || 'File'}
          </span>
        ),
        skeleton: () => <Skeleton className="w-10 h-3 rounded" />,
        sortFn: (a, b) => {
          if (a.isDir !== b.isDir) return a.isDir ? -1 : 1
          const extA = a.name.split('.').pop() || ''
          const extB = b.name.split('.').pop() || ''
          return extA.localeCompare(extB)
        },
      },
      {
        key: 'permissions',
        label: 'ACCESS',
        width: 100,
        render: (entry) => (
          <>
            {entry.isDir && entry.writable && (
              <Chip size="sm" color="success" variant="flat">writable</Chip>
            )}
            {entry.isDir && !entry.writable && entry.readable && (
              <Chip size="sm" color="warning" variant="flat">read-only</Chip>
            )}
          </>
        ),
        skeleton: () => <Skeleton className="w-16 h-5 rounded-full" />,
      },
    ],
    []
  )

  // Bulk actions
  const bulkActions: BulkAction<FileEntry>[] = useMemo(
    () => [
      {
        key: 'copy',
        label: 'Copy',
        icon: <IconCopy size={16} />,
        onAction: (items) => handleCopy(items.map((e) => e.path)),
      },
      {
        key: 'move',
        label: 'Move',
        icon: <IconArrowRight size={16} />,
        onAction: (items) => handleMove(items.map((e) => e.path)),
      },
      {
        key: 'match',
        label: 'Match',
        icon: <IconSearch size={16} />,
        onAction: (items) => handleMatch(items.map((e) => e.path)),
      },
      {
        key: 'delete',
        label: 'Delete',
        icon: <IconTrash size={16} className="text-red-400" />,
        color: 'danger',
        isDestructive: true,
        onAction: (items) => handleDeleteClick(items.map((e) => e.path)),
      },
    ],
    []
  )

  // Helper to check if entry is parent directory
  const isParentEntry = (entry: FileEntry) => entry.name === '..'

  // Row actions - hidden for parent directory
  const rowActions: RowAction<FileEntry>[] = useMemo(
    () => [
      {
        key: 'copy',
        label: 'Copy',
        icon: <IconCopy size={16} />,
        inDropdown: true,
        isVisible: (entry) => !isParentEntry(entry),
        onAction: (entry) => handleCopy([entry.path]),
      },
      {
        key: 'move',
        label: 'Move',
        icon: <IconArrowRight size={16} />,
        inDropdown: true,
        isVisible: (entry) => !isParentEntry(entry),
        onAction: (entry) => handleMove([entry.path]),
      },
      {
        key: 'match',
        label: 'Match to Show',
        icon: <IconSearch size={16} />,
        inDropdown: true,
        isVisible: (entry) => !isParentEntry(entry),
        onAction: (entry) => handleMatch([entry.path]),
      },
      {
        key: 'properties',
        label: 'Properties',
        icon: <IconInfoCircle size={16} />,
        inDropdown: true,
        isVisible: (entry) => !isParentEntry(entry),
        onAction: handleProperties,
      },
      {
        key: 'delete',
        label: 'Delete',
        icon: <IconTrash size={16} className="text-red-400" />,
        isDestructive: true,
        inDropdown: true,
        isVisible: (entry) => !isParentEntry(entry),
        onAction: (entry) => handleDeleteClick([entry.path]),
      },
    ],
    []
  )

  // Search function - exclude parent directory from search
  const searchFn = (entry: FileEntry, term: string) => {
    if (entry.name === '..') return true // Always show parent directory
    return entry.name.toLowerCase().includes(term.toLowerCase())
  }

  // Only show loading spinner if we don't have skeleton support (legacy fallback)
  // Now the DataTable handles showing skeletons during initial load

  return (
    <div className="flex flex-col h-full">
      {/* Fixed Header */}
      <div className="flex-shrink-0 space-y-4 pb-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-xl font-semibold">File Browser</h2>
            <p className="text-sm text-default-500">Browse files in this library</p>
          </div>
        </div>

        {/* Path Input */}
        <Input
          label="Path"
          labelPlacement="inside"
          variant="flat"
          value={inputPath}
          onChange={(e) => setInputPath(e.target.value)}
          onKeyDown={handlePathInputKeyDown}
          className="flex-1"
          classNames={{
            input: 'font-mono text-sm',
            label: 'text-sm font-medium text-primary!',
          }}
          size="sm"
          placeholder="/path/to/folder"
          endContent={
            <div className="flex items-center gap-1">
              <Button size="sm" variant="light" color="primary" className="font-semibold" onPress={() => navigateTo(inputPath)}>
                Go
              </Button>
              <Button
                size="sm"
                variant="light"
                isLoading={loading}
                onPress={() => fetchDirectory(currentPath)}
                isIconOnly
              >
                <IconRefresh size={16} />
              </Button>
            </div>
          }
        />

        {/* Quick Paths */}
        {quickPaths.length > 0 && (
          <div className="flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="flat"
              color="primary"
              onPress={() => navigateTo(libraryPath)}
            >
              📚 Library Root
            </Button>
            {quickPaths.map((qp) => (
              <Button
                key={qp.path}
                size="sm"
                variant="flat"
                onPress={() => navigateTo(qp.path)}
              >
                {qp.name}
              </Button>
            ))}
          </div>
        )}

        <Divider />
      </div>

      {/* Data Table */}
      <div className="flex-1 min-h-0 flex flex-col">
        <DataTable
          stateKey={`file-browser-${libraryPath}`}
          data={sortedEntries}
          columns={columns}
          getRowKey={(entry) => entry.path}
          isLoading={loading}
          selectionMode="multiple"
          isRowSelectable={(entry) => entry.name !== '..'}
          checkboxSelectionOnly
          isPinned={(entry) => entry.name === '..'}
          searchFn={searchFn}
          searchPlaceholder="Search files..."
          bulkActions={bulkActions}
          rowActions={rowActions}
          fillHeight
          showItemCount
          emptyContent={
            entries.length === 0 && !parentPath ? (
              <div className="px-4 py-8 text-center">
                <IconFolderOpen size={40} className="mb-3 mx-auto text-amber-400" />
                <p className="text-default-400">This directory is empty</p>
              </div>
            ) : entries.length === 0 ? (
              <div className="px-4 py-8 text-center">
                <p className="text-default-400">No files or subdirectories</p>
              </div>
            ) : undefined
          }
          ariaLabel="File browser"
          classNames={{
            wrapper: 'flex flex-col h-full',
          }}
        />
      </div>

      {/* Confirm Delete Modal */}
      <ConfirmModal
        isOpen={isConfirmOpen}
        onClose={onConfirmClose}
        onConfirm={handleDelete}
        title="Delete Files"
        message={`Are you sure you want to delete ${pathsToDelete.length} item(s)?`}
        description="This action cannot be undone."
        confirmLabel="Delete"
        confirmColor="danger"
      />

      {/* File Properties Modal */}
      <FilePropertiesModal
        isOpen={isPropertiesOpen}
        onClose={() => {
          onPropertiesClose()
          setPropertiesMediaFileId(null)
          setPropertiesFileName(null)
        }}
        mediaFileId={propertiesMediaFileId}
        title={propertiesFileName ?? undefined}
      />
    </div>
  )
}
