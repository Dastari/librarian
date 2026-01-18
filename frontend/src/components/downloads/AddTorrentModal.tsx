import { useState } from 'react'
import { Modal, ModalContent, ModalHeader, ModalBody } from '@heroui/modal'
import { Button } from '@heroui/button'
import { Input } from '@heroui/input'
import { Tabs, Tab } from '@heroui/tabs'
import { IconLink, IconFolder } from '@tabler/icons-react'

type InputMode = 'magnet' | 'url' | 'file'

export interface AddTorrentModalProps {
  isOpen: boolean
  onClose: () => void
  onAddMagnet: (magnet: string) => Promise<void>
  onAddUrl: (url: string) => Promise<void>
  onAddFile: (file: File) => Promise<void>
  isLoading: boolean
}

export function AddTorrentModal({
  isOpen,
  onClose,
  onAddMagnet,
  onAddUrl,
  onAddFile,
  isLoading,
}: AddTorrentModalProps) {
  const [inputMode, setInputMode] = useState<InputMode>('magnet')
  const [magnetUrl, setMagnetUrl] = useState('')
  const [torrentUrl, setTorrentUrl] = useState('')
  const [isDragging, setIsDragging] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (inputMode === 'magnet' && magnetUrl.trim()) {
      await onAddMagnet(magnetUrl.trim())
      setMagnetUrl('')
      onClose()
    } else if (inputMode === 'url' && torrentUrl.trim()) {
      await onAddUrl(torrentUrl.trim())
      setTorrentUrl('')
      onClose()
    }
  }

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)

    const files = e.dataTransfer.files
    if (files.length > 0) {
      const file = files[0]
      if (file.name.endsWith('.torrent')) {
        await onAddFile(file)
        onClose()
      }
    }
  }

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files
    if (files && files.length > 0) {
      await onAddFile(files[0])
      e.target.value = ''
      onClose()
    }
  }

  const handleClose = () => {
    setMagnetUrl('')
    setTorrentUrl('')
    onClose()
  }

  return (
    <Modal isOpen={isOpen} onClose={handleClose} size="2xl">
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1">
          <h2 className="text-xl font-bold">Add Torrent</h2>
          <p className="text-sm text-default-500 font-normal">
            Add a torrent via magnet link, URL, or file upload
          </p>
        </ModalHeader>
        <ModalBody className="pb-6">
          <Tabs
            selectedKey={inputMode}
            onSelectionChange={(key) => setInputMode(key as InputMode)}
            aria-label="Add torrent options"
            fullWidth
          >
            <Tab key="magnet" title="🧲 Magnet Link">
              <form onSubmit={handleSubmit} className="pt-4 space-y-4">
                <Input
                  label="Magnet Link"
                  labelPlacement="inside"
                  variant="flat"
                  value={magnetUrl}
                  onChange={(e) => setMagnetUrl(e.target.value)}
                  placeholder="Paste magnet link (magnet:?xt=urn:btih:...)..."
                  size="lg"
                  autoFocus
                  classNames={{
                    label: 'text-sm font-medium text-primary!',
                  }}
                />
                <div className="flex justify-end gap-2">
                  <Button variant="flat" onPress={handleClose}>
                    Cancel
                  </Button>
                  <Button
                    type="submit"
                    color="primary"
                    isLoading={isLoading}
                    isDisabled={!magnetUrl.trim()}
                  >
                    Add Torrent
                  </Button>
                </div>
              </form>
            </Tab>
            <Tab key="url" title={<span className="flex items-center gap-1"><IconLink size={16} /> Torrent URL</span>}>
              <form onSubmit={handleSubmit} className="pt-4 space-y-4">
                <Input
                  label="Torrent URL"
                  labelPlacement="inside"
                  variant="flat"
                  value={torrentUrl}
                  onChange={(e) => setTorrentUrl(e.target.value)}
                  placeholder="Enter URL to .torrent file (https://...)..."
                  size="lg"
                  autoFocus
                  classNames={{
                    label: 'text-sm font-medium text-primary!',
                  }}
                />
                <div className="flex justify-end gap-2">
                  <Button variant="flat" onPress={handleClose}>
                    Cancel
                  </Button>
                  <Button
                    type="submit"
                    color="primary"
                    isLoading={isLoading}
                    isDisabled={!torrentUrl.trim()}
                  >
                    Add Torrent
                  </Button>
                </div>
              </form>
            </Tab>
            <Tab key="file" title={<span className="flex items-center gap-1"><IconFolder size={16} className="text-amber-400" /> Upload File</span>}>
              <div className="pt-4 space-y-4">
                <div
                  onDragOver={(e) => {
                    e.preventDefault()
                    setIsDragging(true)
                  }}
                  onDragLeave={() => setIsDragging(false)}
                  onDrop={handleDrop}
                  className={`border-2 border-dashed rounded-xl p-8 text-center transition-colors ${
                    isDragging ? 'border-primary bg-primary/10' : 'border-default-300'
                  }`}
                >
                  <input
                    type="file"
                    accept=".torrent"
                    onChange={handleFileSelect}
                    className="hidden"
                    id="torrent-file-input-modal"
                  />
                  <label htmlFor="torrent-file-input-modal" className="cursor-pointer">
                    <IconFolder size={48} className="mb-4 text-amber-400 mx-auto" />
                    <p className="text-default-600 mb-2">
                      {isDragging ? 'Drop your .torrent file here!' : 'Drag & drop a .torrent file'}
                    </p>
                    <p className="text-default-400 text-sm mb-4">or</p>
                    <Button color="primary" as="span" isLoading={isLoading}>
                      Browse Files
                    </Button>
                  </label>
                </div>
                <div className="flex justify-end">
                  <Button variant="flat" onPress={handleClose}>
                    Cancel
                  </Button>
                </div>
              </div>
            </Tab>
          </Tabs>
        </ModalBody>
      </ModalContent>
    </Modal>
  )
}
