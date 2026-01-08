import { useState, useCallback } from 'react'
import {
  Button,
  Input,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  useDisclosure,
  Spinner,
  Divider,
} from '@heroui/react'
import {
  browseDirectory,
  createDirectory,
  type FileEntry,
  type QuickPath,
} from '../lib/graphql'

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
      setBrowseError(e instanceof Error ? e.message : 'Failed to browse directory')
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
        setBrowseError(result.error || 'Failed to create folder')
      }
    } catch (e) {
      setBrowseError(e instanceof Error ? e.message : 'Failed to create folder')
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
      {label && <label className="text-sm font-medium">{label}</label>}
      <div className="flex gap-2">
        <Input
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className="flex-1"
          classNames={{
            input: 'font-mono text-sm',
          }}
          isDisabled={isDisabled}
        />
        <Button 
          color="primary" 
          variant="flat" 
          onPress={openBrowser}
          isDisabled={isDisabled}
        >
          Browse
        </Button>
      </div>
      {description && (
        <p className="text-xs text-default-400">{description}</p>
      )}

      {/* Folder Browser Modal */}
      <Modal isOpen={isOpen} onClose={onClose} size="2xl" scrollBehavior="inside">
        <ModalContent>
          <ModalHeader className="flex flex-col gap-1">
            {modalTitle}
          </ModalHeader>
          <ModalBody>
            {/* Current path input */}
            <div className="flex items-center gap-2 mb-4">
              <Input
                value={currentPath}
                onChange={(e) => setCurrentPath(e.target.value)}
                onKeyDown={handlePathInputKeyDown}
                className="flex-1"
                classNames={{
                  input: 'font-mono text-sm',
                }}
                size="sm"
                placeholder="/path/to/folder"
              />
              <Button size="sm" onPress={() => browse(currentPath)}>
                Go
              </Button>
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
            {browseError && (
              <div className="text-danger text-sm mb-4 p-2 bg-danger-50 rounded-lg">
                {browseError}
              </div>
            )}

            {/* Directory listing */}
            {isBrowsing ? (
              <div className="flex justify-center py-8">
                <Spinner />
              </div>
            ) : (
              <div className="max-h-80 overflow-y-auto">
                {/* Parent directory */}
                {parentPath && (
                  <button
                    onClick={() => browse(parentPath)}
                    className="w-full text-left px-3 py-2 hover:bg-default-100 rounded-lg flex items-center gap-2 transition-colors"
                  >
                    <span className="text-lg">📁</span>
                    <span className="text-default-600">..</span>
                    <span className="text-xs text-default-400 ml-auto">Parent directory</span>
                  </button>
                )}

                {/* Directory entries */}
                {entries.map((entry) => (
                  <button
                    key={entry.path}
                    onClick={() => entry.isDir && entry.readable && browse(entry.path)}
                    className={`w-full text-left px-3 py-2 hover:bg-default-100 rounded-lg flex items-center gap-2 transition-colors ${
                      !entry.readable ? 'opacity-50 cursor-not-allowed' : ''
                    }`}
                    disabled={!entry.isDir || !entry.readable}
                  >
                    <span className="text-lg">{entry.isDir ? '📁' : '📄'}</span>
                    <span className="flex-1 truncate">{entry.name}</span>
                    {entry.isDir && entry.writable && (
                      <span className="text-xs text-success-600 bg-success-50 px-2 py-0.5 rounded">
                        writable
                      </span>
                    )}
                    {entry.isDir && !entry.writable && entry.readable && (
                      <span className="text-xs text-warning-600 bg-warning-50 px-2 py-0.5 rounded">
                        read-only
                      </span>
                    )}
                  </button>
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
              <div className="flex gap-2">
                <Input
                  size="sm"
                  placeholder="New folder name"
                  value={newFolderName}
                  onChange={(e) => setNewFolderName(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && handleCreateFolder()}
                  className="flex-1"
                  isDisabled={isCreatingFolder}
                />
                <Button 
                  size="sm" 
                  color="primary" 
                  onPress={handleCreateFolder}
                  isLoading={isCreatingFolder}
                >
                  Create
                </Button>
                <Button
                  size="sm"
                  variant="flat"
                  onPress={() => {
                    setShowNewFolder(false)
                    setNewFolderName('')
                  }}
                  isDisabled={isCreatingFolder}
                >
                  Cancel
                </Button>
              </div>
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
