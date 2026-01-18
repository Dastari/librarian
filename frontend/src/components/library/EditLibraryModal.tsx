import { useState, useEffect } from 'react'
import { Button } from '@heroui/button'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Input } from '@heroui/input'
import { Select, SelectItem } from '@heroui/select'
import { Switch } from '@heroui/switch'
import { Divider } from '@heroui/divider'
import { FolderBrowserInput } from '../FolderBrowserInput'
import { LIBRARY_TYPES, getLibraryTypeInfo, type Library, type LibraryType, type PostDownloadAction, type UpdateLibraryInput } from '../../lib/graphql'
import { IconFolder } from '@tabler/icons-react'

export interface EditLibraryModalProps {
  isOpen: boolean
  onClose: () => void
  library: Library | null
  onSave: (id: string, input: UpdateLibraryInput) => Promise<void>
  isLoading: boolean
}

export function EditLibraryModal({ isOpen, onClose, library, onSave, isLoading }: EditLibraryModalProps) {
  const [name, setName] = useState('')
  const [path, setPath] = useState('')
  const [libraryType, setLibraryType] = useState<LibraryType>('TV')
  const [autoScan, setAutoScan] = useState(true)
  const [scanInterval, setScanInterval] = useState(60)
  const [watchForChanges, setWatchForChanges] = useState(false)
  const [postDownloadAction, setPostDownloadAction] = useState<PostDownloadAction>('COPY')
  const [organizeFiles, setOrganizeFiles] = useState(true)
  const [autoAddDiscovered, setAutoAddDiscovered] = useState(true)

  // Populate form when library changes
  useEffect(() => {
    if (library) {
      setName(library.name)
      setPath(library.path)
      setLibraryType(library.libraryType)
      setAutoScan(library.autoScan)
      setScanInterval(library.scanIntervalMinutes)
      setWatchForChanges(library.watchForChanges)
      setPostDownloadAction(library.postDownloadAction)
      setOrganizeFiles(library.organizeFiles)
      setAutoAddDiscovered(library.autoAddDiscovered)
    }
  }, [library])

  const handleSubmit = async () => {
    if (!library || !name || !path) return
    
    await onSave(library.id, {
      name,
      path,
      autoScan,
      scanIntervalMinutes: scanInterval,
      watchForChanges,
      postDownloadAction,
      organizeFiles,
      autoAddDiscovered,
    })
    onClose()
  }

  const handleClose = () => {
    onClose()
  }

  if (!library) return null

  return (
    <Modal isOpen={isOpen} onClose={handleClose} size="xl">
      <ModalContent>
        <ModalHeader className="flex items-center gap-3">
          {(() => {
            const typeInfo = getLibraryTypeInfo(libraryType)
            const IconComponent = typeInfo?.Icon || IconFolder
            return <IconComponent className="w-6 h-6" />
          })()}
          <span>Edit Library</span>
        </ModalHeader>
        <ModalBody>
          <div className="space-y-4">
            <Input
              label="Library Name"
              placeholder="e.g., Movies, TV Shows"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />

            <FolderBrowserInput
              label="Path"
              value={path}
              onChange={setPath}
              placeholder="/data/media/TV"
              description="Full path to the media folder"
              modalTitle="Select Library Folder"
            />

            <Select
              label="Library Type"
              selectedKeys={[libraryType]}
              onChange={(e) => setLibraryType(e.target.value as LibraryType)}
              isDisabled
              description="Library type cannot be changed after creation"
            >
              {LIBRARY_TYPES.map((type) => (
                <SelectItem key={type.value} textValue={type.label}>
                  <div className="flex items-center gap-2">
                    <type.Icon className="w-4 h-4" />
                    {type.label}
                  </div>
                </SelectItem>
              ))}
            </Select>

            <Divider />

            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium">Auto-scan</p>
                <p className="text-xs text-default-500">
                  Automatically scan for new files periodically
                </p>
              </div>
              <Switch isSelected={autoScan} onValueChange={setAutoScan} />
            </div>

            {autoScan && (
              <Input
                type="number"
                label="Scan Interval (minutes)"
                value={scanInterval.toString()}
                onChange={(e) => setScanInterval(parseInt(e.target.value) || 60)}
                min={5}
                max={1440}
              />
            )}

            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium">Watch for changes</p>
                <p className="text-xs text-default-500">
                  Use filesystem notifications for instant detection
                </p>
              </div>
              <Switch
                isSelected={watchForChanges}
                onValueChange={setWatchForChanges}
              />
            </div>

            <Divider />

            <Select
              label="Post-download action"
              selectedKeys={[postDownloadAction]}
              onChange={(e) =>
                setPostDownloadAction(e.target.value as PostDownloadAction)
              }
              description="What to do with files after downloading"
            >
              <SelectItem key="COPY" textValue="Copy">
                Copy (preserves seeding)
              </SelectItem>
              <SelectItem key="MOVE" textValue="Move">
                Move (stops seeding)
              </SelectItem>
              <SelectItem key="HARDLINK" textValue="Hardlink">
                Hardlink (same disk only)
              </SelectItem>
            </Select>

            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium">Organize files</p>
                <p className="text-xs text-default-500">
                  Organize downloaded files into show/season folders
                </p>
              </div>
              <Switch isSelected={organizeFiles} onValueChange={setOrganizeFiles} />
            </div>

            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium">Auto-add discovered shows</p>
                <p className="text-xs text-default-500">
                  Automatically add shows found during scanning
                </p>
              </div>
              <Switch
                isSelected={autoAddDiscovered}
                onValueChange={setAutoAddDiscovered}
              />
            </div>
          </div>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={handleClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            onPress={handleSubmit}
            isDisabled={!name || !path}
            isLoading={isLoading}
          >
            Save Changes
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
