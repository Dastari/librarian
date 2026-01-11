import { useState, useEffect } from 'react'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'
import { Input } from '@heroui/input'
import { Switch } from '@heroui/switch'
import type { RssFeed } from '../../lib/graphql'

export interface RssFeedFormData {
  name: string
  url: string
  enabled: boolean
  pollIntervalMinutes: number
}

export interface AddRssFeedModalProps {
  isOpen: boolean
  onClose: () => void
  onAdd: (data: RssFeedFormData) => Promise<void>
  isLoading: boolean
}

export function AddRssFeedModal({
  isOpen,
  onClose,
  onAdd,
  isLoading,
}: AddRssFeedModalProps) {
  const [name, setName] = useState('')
  const [url, setUrl] = useState('')
  const [enabled, setEnabled] = useState(true)
  const [pollInterval, setPollInterval] = useState(15)

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setName('')
      setUrl('')
      setEnabled(true)
      setPollInterval(15)
    }
  }, [isOpen])

  const handleSubmit = async () => {
    await onAdd({
      name,
      url,
      enabled,
      pollIntervalMinutes: pollInterval,
    })
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="lg">
      <ModalContent>
        <ModalHeader>Add RSS Feed</ModalHeader>
        <ModalBody className="gap-4">
          <Input
            label="Feed Name"
            placeholder="My Torrent Feed"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <Input
            label="Feed URL"
            placeholder="https://example.com/rss"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
          />
          <div className="flex justify-between items-center">
            <div>
              <p className="font-medium">Enabled</p>
              <p className="text-xs text-default-400">
                Start polling immediately after adding
              </p>
            </div>
            <Switch isSelected={enabled} onValueChange={setEnabled} />
          </div>
          <Input
            type="number"
            label="Poll Interval (minutes)"
            placeholder="15"
            value={pollInterval.toString()}
            onChange={(e) => setPollInterval(parseInt(e.target.value) || 15)}
            min={5}
            max={1440}
          />
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            onPress={handleSubmit}
            isLoading={isLoading}
            isDisabled={!name || !url}
          >
            Add Feed
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}

export interface EditRssFeedModalProps {
  isOpen: boolean
  onClose: () => void
  feed: RssFeed | null
  onSave: (data: RssFeedFormData) => Promise<void>
  isLoading: boolean
}

export function EditRssFeedModal({
  isOpen,
  onClose,
  feed,
  onSave,
  isLoading,
}: EditRssFeedModalProps) {
  const [name, setName] = useState('')
  const [url, setUrl] = useState('')
  const [enabled, setEnabled] = useState(true)
  const [pollInterval, setPollInterval] = useState(15)

  // Initialize form when feed changes
  useEffect(() => {
    if (feed) {
      setName(feed.name)
      setUrl(feed.url)
      setEnabled(feed.enabled)
      setPollInterval(feed.pollIntervalMinutes)
    }
  }, [feed])

  const handleSubmit = async () => {
    await onSave({
      name,
      url,
      enabled,
      pollIntervalMinutes: pollInterval,
    })
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="lg">
      <ModalContent>
        <ModalHeader>Edit RSS Feed</ModalHeader>
        <ModalBody className="gap-4">
          <Input
            label="Feed Name"
            placeholder="My Torrent Feed"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <Input
            label="Feed URL"
            placeholder="https://example.com/rss"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
          />
          <div className="flex justify-between items-center">
            <div>
              <p className="font-medium">Enabled</p>
              <p className="text-xs text-default-400">Enable or disable polling</p>
            </div>
            <Switch isSelected={enabled} onValueChange={setEnabled} />
          </div>
          <Input
            type="number"
            label="Poll Interval (minutes)"
            placeholder="15"
            value={pollInterval.toString()}
            onChange={(e) => setPollInterval(parseInt(e.target.value) || 15)}
            min={5}
            max={1440}
          />
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            onPress={handleSubmit}
            isLoading={isLoading}
            isDisabled={!name || !url}
          >
            Save Changes
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
