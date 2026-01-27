import { useState, useCallback, useEffect } from 'react'
import { Button } from '@heroui/button'
import { Input } from '@heroui/input'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Spinner } from '@heroui/spinner'
import { Divider } from '@heroui/divider'
import { Chip } from '@heroui/chip'
import {
  browseDirectory,
  createDirectory,
  type BrowseDirectoryEntry,
  type BrowseQuickPath,
} from '../lib/graphql'
import { IconFolder, IconFolderPlus } from '@tabler/icons-react'
import { InlineError } from './shared'
import { sanitizeError } from '../lib/format'

interface DestinationPickerModalProps {
  /** Whether the modal is open */
  isOpen: boolean
  /** Called when the modal should close */
  onClose: () => void
  /** Called when a destination is selected */
  onSelect: (destinationPath: string) => void
  /** Modal title */
  title?: string
  /** Description of what will happen */
  description?: string
  /** Initial path to start browsing from */
  initialPath?: string
  /** Text for the confirm button */
  confirmLabel?: string
  /** Whether the operation is in progress */
  isLoading?: boolean
}

export function DestinationPickerModal({
  isOpen,
  onClose,
  onSelect,
  title = 'Select Destination',
  description,
  initialPath = '/',
  confirmLabel = 'Select',
  isLoading = false,
}: DestinationPickerModalProps) {
  const [currentPath, setCurrentPath] = useState(initialPath)
  const [entries, setEntries] = useState<BrowseDirectoryEntry[]>([])
  const [quickPaths, setQuickPaths] = useState<BrowseQuickPath[]>([])
  const [parentPath, setParentPath] = useState<string | null>(null)
  const [isBrowsing, setIsBrowsing] = useState(false)
  const [browseError, setBrowseError] = useState<string | null>(null)
  const [newFolderName, setNewFolderName] = useState('')
  const [showNewFolder, setShowNewFolder] = useState(false)
  const [isCreatingFolder, setIsCreatingFolder] = useState(false)

  const browse = useCallback(async (path?: string): Promise<boolean> => {
    setIsBrowsing(true)
    setBrowseError(null)
    try {
      const result = await browseDirectory(path, true)
      setCurrentPath(result.CurrentPath || '/')
      setParentPath(result.ParentPath ?? null)
      setEntries(result.Entries ?? [])
      setQuickPaths(result.QuickPaths ?? [])
      return true
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e)
      const isAuthError = errorMsg.toLowerCase().includes('authentication')
      setBrowseError(isAuthError ? 'Please sign in to browse folders' : sanitizeError(e))
      return false
    } finally {
      setIsBrowsing(false)
    }
  }, [])

  // Browse initial path when modal opens
  useEffect(() => {
    if (isOpen) {
      browse(initialPath)
    }
  }, [isOpen, initialPath, browse])

  const handleCreateFolder = async () => {
    if (!newFolderName.trim()) return

    setIsCreatingFolder(true)
    const newPath = `${currentPath}/${newFolderName.trim()}`

    try {
      const result = await createDirectory(newPath)
      if (result.success) {
        setNewFolderName('')
        setShowNewFolder(false)
        await browse(newPath)
      } else {
        setBrowseError(sanitizeError(result.error || 'Failed to create folder'))
      }
    } catch (e) {
      setBrowseError(sanitizeError(e))
    } finally {
      setIsCreatingFolder(false)
    }
  }

  const handlePathInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      browse(currentPath)
    }
  }

  const handleSelect = () => {
    onSelect(currentPath)
  }

  // Sort entries: directories first, then alphabetically
  const sortedEntries = [...entries]
    .filter((e) => e.IsDir)
    .sort((a, b) => a.Name.localeCompare(b.Name))

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="2xl" scrollBehavior="inside">
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1">
          <span>{title}</span>
          {description && (
            <span className="text-sm font-normal text-default-500">{description}</span>
          )}
        </ModalHeader>
        <ModalBody>
          {/* Current path input */}
          <div className="mb-4">
            <Input
              label="Destination Path"
              labelPlacement="inside"
              variant="flat"
              value={currentPath}
              onChange={(e) => setCurrentPath(e.target.value)}
              onKeyDown={handlePathInputKeyDown}
              className="flex-1"
              classNames={{
                input: 'font-mono text-sm',
                label: 'text-sm font-medium text-primary!',
              }}
              size="sm"
              placeholder="/path/to/folder"
              endContent={
                <Button
                  size="sm"
                  variant="light"
                  color="primary"
                  className="font-semibold"
                  onPress={() => browse(currentPath)}
                >
                  Go
                </Button>
              }
            />
          </div>

          {/* Quick paths */}
          {quickPaths.length > 0 && (
            <div className="flex flex-wrap gap-2 mb-4">
              {quickPaths.map((qp) => (
                <Button key={qp.Path} size="sm" variant="flat" onPress={() => browse(qp.Path)}>
                  {qp.Name}
                </Button>
              ))}
            </div>
          )}

          <Divider className="my-2" />

          {/* Error message */}
          {browseError && <InlineError message={browseError} className="mb-4" />}

          {/* Directory listing */}
          {isBrowsing ? (
            <div className="flex justify-center py-8">
              <Spinner />
            </div>
          ) : (
            <div className="max-h-80 overflow-y-auto">
              {/* Parent directory */}
              {parentPath && (
                <Button
                  variant="light"
                  onPress={() => browse(parentPath)}
                  className="w-full justify-start px-3 py-2 h-auto"
                >
                  <IconFolder size={20} className="text-amber-400" />
                  <span className="text-default-600">..</span>
                  <span className="text-xs text-default-400 ml-auto">Parent directory</span>
                </Button>
              )}

              {/* Directory entries - only show writable directories */}
              {sortedEntries.map((entry) => (
                <Button
                  key={entry.Path}
                  variant="light"
                  onPress={() => entry.Readable && browse(entry.Path)}
                  className={`w-full justify-start px-3 py-2 h-auto ${
                    !entry.Readable ? 'opacity-50' : ''
                  }`}
                  isDisabled={!entry.Readable}
                >
                  <IconFolder size={20} className="text-amber-400" />
                  <span className="flex-1 truncate text-left">{entry.Name}</span>
                  {entry.Writable && (
                    <Chip size="sm" color="success" variant="flat">
                      writable
                    </Chip>
                  )}
                  {!entry.Writable && entry.Readable && (
                    <Chip size="sm" color="warning" variant="flat">
                      read-only
                    </Chip>
                  )}
                </Button>
              ))}

              {sortedEntries.length === 0 && !parentPath && (
                <p className="text-center text-default-400 py-4">Empty directory</p>
              )}

              {sortedEntries.length === 0 && parentPath && (
                <p className="text-center text-default-400 py-4">No subdirectories</p>
              )}
            </div>
          )}

          <Divider className="my-2" />

          {/* New folder creation */}
          {showNewFolder ? (
            <Input
              label="New Folder Name"
              labelPlacement="inside"
              variant="flat"
              size="sm"
              placeholder="Enter folder name"
              value={newFolderName}
              onChange={(e) => setNewFolderName(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleCreateFolder()}
              className="flex-1"
              isDisabled={isCreatingFolder}
              classNames={{
                label: 'text-sm font-medium text-primary!',
              }}
              endContent={
                <div className="flex items-center gap-1">
                  <Button
                    size="sm"
                    variant="light"
                    color="primary"
                    className="font-semibold"
                    onPress={handleCreateFolder}
                    isLoading={isCreatingFolder}
                  >
                    Create
                  </Button>
                  <Button
                    size="sm"
                    variant="light"
                    onPress={() => {
                      setShowNewFolder(false)
                      setNewFolderName('')
                    }}
                    isDisabled={isCreatingFolder}
                  >
                    Cancel
                  </Button>
                </div>
              }
            />
          ) : (
            <Button
              size="sm"
              variant="flat"
              onPress={() => setShowNewFolder(true)}
              startContent={<IconFolderPlus size={16} />}
            >
              New Folder
            </Button>
          )}
        </ModalBody>
        <ModalFooter>
          <div className="flex-1 text-sm text-default-500 truncate font-mono">{currentPath}</div>
          <Button variant="flat" onPress={onClose} isDisabled={isLoading}>
            Cancel
          </Button>
          <Button color="primary" onPress={handleSelect} isLoading={isLoading}>
            {confirmLabel}
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
