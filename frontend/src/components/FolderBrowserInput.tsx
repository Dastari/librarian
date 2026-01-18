import { useState, useCallback } from 'react'
import { Button } from '@heroui/button'
import { Input } from '@heroui/input'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter, useDisclosure } from '@heroui/modal'
import { Spinner } from '@heroui/spinner'
import { Divider } from '@heroui/divider'
import { Chip } from '@heroui/chip'
import {
  browseDirectory,
  createDirectory,
  type FileEntry,
  type QuickPath,
} from '../lib/graphql'
import { IconFolder, IconFile } from '@tabler/icons-react'
import { InlineError } from './shared'
import { sanitizeError } from '../lib/format'

interface FolderBrowserInputProps {
  /** Current folder path value */
  value: string
  /** Called when folder selection changes */
  onChange: (path: string) => void
  /** Label for the input field */
  label?: string
  /** Placeholder text */
  placeholder?: string
  /** Description text below the input */
  description?: string
  /** Modal title when browsing */
  modalTitle?: string
  /** Whether the input is disabled */
  isDisabled?: boolean
  /** Custom class name for the container */
  className?: string
}

export function FolderBrowserInput({
  value,
  onChange,
  label,
  placeholder = '/path/to/folder',
  description,
  modalTitle = 'Select Folder',
  isDisabled = false,
  className = '',
}: FolderBrowserInputProps) {
  const { isOpen, onOpen, onClose } = useDisclosure()
  const [currentPath, setCurrentPath] = useState('')
  const [entries, setEntries] = useState<FileEntry[]>([])
  const [quickPaths, setQuickPaths] = useState<QuickPath[]>([])
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
      setCurrentPath(result.currentPath || '/')
      setParentPath(result.parentPath ?? null)
      setEntries(result.entries || [])
      setQuickPaths(result.quickPaths || [])
      return true
    } catch (e) {
      console.error('Browse error:', e)
      setBrowseError(sanitizeError(e))
      return false
    } finally {
      setIsBrowsing(false)
    }
  }, [])

  const openBrowser = async () => {
    // Start browsing from current value, or root if empty
    // If the current value fails (invalid path), fall back to root
    const initialPath = value || '/'
    const success = await browse(initialPath)
    if (!success && initialPath !== '/') {
      // If browsing the initial path fails, start from root
      await browse('/')
    }
    onOpen()
  }

  const selectPath = () => {
    onChange(currentPath)
    onClose()
  }

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

  return (
    <div className={`flex flex-col gap-2 ${className}`}>
      <div className="flex gap-2">
        <Input
          value={value}
          label={label}
          labelPlacement="inside"
          variant="flat"
          description={description}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className="flex-1"
          classNames={{
            input: 'font-mono text-sm',
            label: 'text-sm font-medium text-primary!',
          }}
          isDisabled={isDisabled}
          endContent={<Button size="sm" variant="light" color="primary" className="font-semibold" onPress={openBrowser}>Browse</Button>}
        />
      </div>

      <Modal isOpen={isOpen} onClose={onClose} size="2xl" scrollBehavior="inside">
        <ModalContent>
          <ModalHeader className="flex flex-col gap-1">
            {modalTitle}
          </ModalHeader>
          <ModalBody>
            {/* Current path input */}
            <div className="mb-4">
              <Input
                label="Path"
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
                  <Button size="sm" variant="light" color="primary" className="font-semibold" onPress={() => browse(currentPath)}>
                    Go
                  </Button>
                }
              />
            </div>

            {/* Quick paths */}
            {quickPaths.length > 0 && (
              <div className="flex flex-wrap gap-2 mb-4">
                {quickPaths.map((qp) => (
                  <Button
                    key={qp.path}
                    size="sm"
                    variant="flat"
                    onPress={() => browse(qp.path)}
                  >
                    {qp.name}
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

                {/* Directory entries */}
                {entries.map((entry) => (
                  <Button
                    key={entry.path}
                    variant="light"
                    onPress={() => entry.isDir && entry.readable && browse(entry.path)}
                    className={`w-full justify-start px-3 py-2 h-auto ${
                      !entry.readable ? 'opacity-50' : ''
                    }`}
                    isDisabled={!entry.isDir || !entry.readable}
                  >
                    {entry.isDir ? <IconFolder size={20} className="text-amber-400" /> : <IconFile size={20} className="text-default-400" />}
                    <span className="flex-1 truncate text-left">{entry.name}</span>
                    {entry.isDir && entry.writable && (
                      <Chip size="sm" color="success" variant="flat">
                        writable
                      </Chip>
                    )}
                    {entry.isDir && !entry.writable && entry.readable && (
                      <Chip size="sm" color="warning" variant="flat">
                        read-only
                      </Chip>
                    )}
                  </Button>
                ))}

                {entries.length === 0 && !parentPath && (
                  <p className="text-center text-default-400 py-4">
                    Empty directory
                  </p>
                )}

                {entries.length === 0 && parentPath && (
                  <p className="text-center text-default-400 py-4">
                    No subdirectories
                  </p>
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
                startContent={<span>+</span>}
              >
                New Folder
              </Button>
            )}
          </ModalBody>
          <ModalFooter>
            <div className="flex-1 text-sm text-default-500 truncate font-mono">
              {currentPath}
            </div>
            <Button variant="flat" onPress={onClose}>
              Cancel
            </Button>
            <Button color="primary" onPress={selectPath}>
              Select This Folder
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  )
}
