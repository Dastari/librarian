import { createFileRoute, Link, redirect } from '@tanstack/react-router'
import { useState } from 'react'
import { Button, Card, CardBody, CardHeader, Modal, ModalContent, ModalHeader, ModalBody, ModalFooter, Input, Select, SelectItem, Switch, useDisclosure, Chip, Spinner, Divider } from '@heroui/react'
import { useAuth } from '../../hooks/useAuth'
import { LIBRARY_TYPES, type Library, type LibraryType, type CreateLibraryRequest } from '../../lib/api'
import { FolderBrowserInput } from '../../components/FolderBrowserInput'

export const Route = createFileRoute('/libraries/')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/auth/login' })
    }
  },
  component: LibrariesPage,
})

// Mock data for display
const mockLibraries: Library[] = [
  {
    id: '1',
    name: 'Movies',
    path: '/data/media/Movies',
    library_type: 'movies',
    icon: 'film',
    color: 'purple',
    auto_scan: true,
    scan_interval_hours: 24,
    last_scanned_at: null,
    file_count: 142,
    total_size_bytes: 856000000000,
  },
  {
    id: '2',
    name: 'TV Shows',
    path: '/data/media/TV',
    library_type: 'tv',
    icon: 'tv',
    color: 'blue',
    auto_scan: true,
    scan_interval_hours: 6,
    last_scanned_at: '2026-01-08T12:00:00Z',
    file_count: 1247,
    total_size_bytes: 2340000000000,
  },
]

function formatBytes(bytes: number | null): string {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let unitIndex = 0
  let size = bytes
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }
  return `${size.toFixed(1)} ${units[unitIndex]}`
}

function LibraryCard({ library, onScan, onEdit }: { library: Library; onScan: () => void; onEdit: () => void }) {
  const typeInfo = LIBRARY_TYPES.find(t => t.value === library.library_type) || LIBRARY_TYPES[4]
  
  return (
    <Card className="bg-content1">
      <CardHeader className="flex justify-between items-start">
        <div className="flex items-center gap-3">
          <span className="text-3xl">{typeInfo.icon}</span>
          <div>
            <h3 className="text-lg font-semibold">{library.name}</h3>
            <p className="text-default-500 text-sm">{typeInfo.label}</p>
          </div>
        </div>
        <Chip size="sm" color={library.auto_scan ? 'success' : 'default'} variant="flat">
          {library.auto_scan ? 'Auto-scan' : 'Manual'}
        </Chip>
      </CardHeader>
      <CardBody className="pt-0">
        <div className="space-y-3">
          <div className="text-sm">
            <span className="text-default-500">Path:</span>
            <span className="ml-2 text-default-400 font-mono text-xs">{library.path}</span>
          </div>
          
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-default-500">Files:</span>
              <span className="ml-2">{library.file_count ?? 0}</span>
            </div>
            <div>
              <span className="text-default-500">Size:</span>
              <span className="ml-2">{formatBytes(library.total_size_bytes)}</span>
            </div>
          </div>

          {library.last_scanned_at && (
            <div className="text-sm">
              <span className="text-default-500">Last scan:</span>
              <span className="ml-2 text-default-400">
                {new Date(library.last_scanned_at).toLocaleString()}
              </span>
            </div>
          )}

          <div className="flex gap-2 pt-2">
            <Button 
              size="sm" 
              color="primary" 
              variant="flat"
              onPress={onScan}
            >
              Scan Now
            </Button>
            <Button 
              size="sm" 
              variant="flat"
              onPress={onEdit}
            >
              Settings
            </Button>
            <Link
              to="/"
              className="flex-1"
            >
              <Button 
                size="sm" 
                variant="flat"
                className="w-full"
              >
                Browse
              </Button>
            </Link>
          </div>
        </div>
      </CardBody>
    </Card>
  )
}

function AddLibraryModal({ isOpen, onClose, onAdd }: { 
  isOpen: boolean
  onClose: () => void
  onAdd: (library: CreateLibraryRequest) => void 
}) {
  const [name, setName] = useState('')
  const [path, setPath] = useState('')
  const [libraryType, setLibraryType] = useState<LibraryType>('movies')
  const [autoScan, setAutoScan] = useState(true)
  const [scanInterval, setScanInterval] = useState(24)

  const handleSubmit = () => {
    if (!name || !path) return
    onAdd({
      name,
      path,
      library_type: libraryType,
      auto_scan: autoScan,
      scan_interval_hours: scanInterval,
    })
    // Reset form
    setName('')
    setPath('')
    setLibraryType('movies')
    setAutoScan(true)
    setScanInterval(24)
    onClose()
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} className="dark">
      <ModalContent>
        <ModalHeader>Add Library</ModalHeader>
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
              placeholder="/data/media/Movies"
              description="Full path to the media folder"
              modalTitle="Select Library Folder"
            />

            <Select
              label="Library Type"
              selectedKeys={[libraryType]}
              onChange={(e) => setLibraryType(e.target.value as LibraryType)}
            >
              {LIBRARY_TYPES.map((type) => (
                <SelectItem key={type.value} textValue={type.label}>
                  <span className="mr-2">{type.icon}</span>
                  {type.label}
                </SelectItem>
              ))}
            </Select>

            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium">Auto-scan</p>
                <p className="text-xs text-default-500">Automatically scan for new files</p>
              </div>
              <Switch 
                isSelected={autoScan} 
                onValueChange={setAutoScan}
              />
            </div>

            {autoScan && (
              <Input
                type="number"
                label="Scan Interval (hours)"
                value={scanInterval.toString()}
                onChange={(e) => setScanInterval(parseInt(e.target.value) || 24)}
                min={1}
                max={168}
              />
            )}
          </div>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button color="primary" onPress={handleSubmit} isDisabled={!name || !path}>
            Add Library
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}

function LibrariesPage() {
  const { user, loading } = useAuth()
  const { isOpen, onOpen, onClose } = useDisclosure()
  const [libraries, setLibraries] = useState<Library[]>(mockLibraries)

  if (loading) {
    return (
      <div className="flex items-center justify-center h-[calc(100vh-4rem)]">
        <Spinner size="lg" color="primary" />
      </div>
    )
  }

  if (!user) {
    return (
      <div className="flex flex-col items-center justify-center h-[calc(100vh-4rem)] px-4">
        <h1 className="text-2xl font-bold mb-4">Sign in to manage libraries</h1>
        <Link to="/auth/login">
          <Button color="primary">Sign In</Button>
        </Link>
      </div>
    )
  }

  const handleAddLibrary = (data: CreateLibraryRequest) => {
    // TODO: Call API
    const newLibrary: Library = {
      id: crypto.randomUUID(),
      ...data,
      icon: LIBRARY_TYPES.find(t => t.value === data.library_type)?.icon || '📁',
      color: LIBRARY_TYPES.find(t => t.value === data.library_type)?.color || 'slate',
      auto_scan: data.auto_scan ?? true,
      scan_interval_hours: data.scan_interval_hours ?? 24,
      last_scanned_at: null,
      file_count: 0,
      total_size_bytes: 0,
    }
    setLibraries([...libraries, newLibrary])
  }

  const handleScan = (libraryId: string) => {
    // TODO: Call API
    console.log('Scanning library:', libraryId)
    alert(`Scan started for library ${libraryId}`)
  }

  const handleEdit = (libraryId: string) => {
    // TODO: Open edit modal
    console.log('Editing library:', libraryId)
    alert(`Edit library ${libraryId} - coming soon!`)
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Libraries</h1>
          <p className="text-default-500">Manage your media libraries</p>
        </div>
        <Button color="primary" onPress={onOpen}>
          + Add Library
        </Button>
      </div>

      {libraries.length === 0 ? (
        <Card className="bg-content1/50 border-default-300 border-dashed border-2">
          <CardBody className="py-12 text-center">
            <span className="text-5xl mb-4 block">📚</span>
            <h3 className="text-lg font-semibold mb-2">No libraries yet</h3>
            <p className="text-default-500 mb-4">
              Add a library to start organizing your media collection.
            </p>
            <Button color="primary" onPress={onOpen}>
              Add Your First Library
            </Button>
          </CardBody>
        </Card>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {libraries.map((library) => (
            <LibraryCard
              key={library.id}
              library={library}
              onScan={() => handleScan(library.id)}
              onEdit={() => handleEdit(library.id)}
            />
          ))}
        </div>
      )}

      <AddLibraryModal
        isOpen={isOpen}
        onClose={onClose}
        onAdd={handleAddLibrary}
      />

      {/* Info section */}
      <Card className="mt-8 bg-content2">
        <CardHeader>
          <h3 className="font-semibold">About Libraries</h3>
        </CardHeader>
        <Divider />
        <CardBody>
          <ul className="text-default-500 text-sm space-y-1">
            <li>• <strong>Movies</strong> - Feature films, organized by title and year</li>
            <li>• <strong>TV Shows</strong> - Series organized by show, season, and episode</li>
            <li>• <strong>Music</strong> - Albums and tracks (coming soon)</li>
            <li>• <strong>Audiobooks</strong> - Audio books organized by author and title (coming soon)</li>
            <li>• Each library can have its own scan schedule and settings</li>
          </ul>
        </CardBody>
      </Card>
    </div>
  )
}
