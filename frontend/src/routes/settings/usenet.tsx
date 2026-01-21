import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import { Card, CardBody } from '@heroui/card'
import { Button } from '@heroui/button'
import { Chip } from '@heroui/chip'
import { Divider } from '@heroui/divider'
import { Switch } from '@heroui/switch'
import { Spinner } from '@heroui/spinner'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter, useDisclosure } from '@heroui/modal'
import { Input } from '@heroui/input'
import { Tooltip } from '@heroui/tooltip'
import { addToast } from '@heroui/toast'
import {
  IconPlus,
  IconTrash,
  IconEdit,
  IconServer,
  IconLock,
  IconLockOpen,
  IconArrowUp,
  IconArrowDown,
} from '@tabler/icons-react'
import { graphqlClient } from '../../lib/graphql'
import { sanitizeError } from '../../lib/format'
import { InlineError } from '../../components/shared'

export const Route = createFileRoute('/settings/usenet')({
  component: UsenetSettingsPage,
})

// Types
interface UsenetServer {
  id: string
  name: string
  host: string
  port: number
  useSsl: boolean
  username: string | null
  connections: number
  priority: number
  enabled: boolean
  retentionDays: number | null
  lastSuccessAt: string | null
  lastError: string | null
  errorCount: number
}

interface UsenetServerResult {
  success: boolean
  error: string | null
  server: UsenetServer | null
}

// GraphQL Queries
const USENET_SERVERS_QUERY = `
  query UsenetServers {
    usenetServers {
      id
      name
      host
      port
      useSsl
      username
      connections
      priority
      enabled
      retentionDays
      lastSuccessAt
      lastError
      errorCount
    }
  }
`

const CREATE_USENET_SERVER_MUTATION = `
  mutation CreateUsenetServer($input: CreateUsenetServerInput!) {
    createUsenetServer(input: $input) {
      success
      error
      server {
        id
        name
        host
        port
        useSsl
        username
        connections
        priority
        enabled
        retentionDays
        lastSuccessAt
        lastError
        errorCount
      }
    }
  }
`

const UPDATE_USENET_SERVER_MUTATION = `
  mutation UpdateUsenetServer($id: String!, $input: UpdateUsenetServerInput!) {
    updateUsenetServer(id: $id, input: $input) {
      success
      error
      server {
        id
        name
        host
        port
        useSsl
        username
        connections
        priority
        enabled
        retentionDays
        lastSuccessAt
        lastError
        errorCount
      }
    }
  }
`

const DELETE_USENET_SERVER_MUTATION = `
  mutation DeleteUsenetServer($id: String!) {
    deleteUsenetServer(id: $id) {
      success
      error
    }
  }
`

const REORDER_USENET_SERVERS_MUTATION = `
  mutation ReorderUsenetServers($ids: [String!]!) {
    reorderUsenetServers(ids: $ids) {
      success
      error
    }
  }
`

function UsenetSettingsPage() {
  const [servers, setServers] = useState<UsenetServer[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  
  // Modal state
  const { isOpen, onOpen, onClose } = useDisclosure()
  const [editingServer, setEditingServer] = useState<UsenetServer | null>(null)
  const [formData, setFormData] = useState({
    name: '',
    host: '',
    port: 563,
    useSsl: true,
    username: '',
    password: '',
    connections: 10,
    retentionDays: null as number | null,
  })
  const [saving, setSaving] = useState(false)

  // Load servers
  const loadServers = useCallback(async () => {
    try {
      setLoading(true)
      const result = await graphqlClient
        .query<{ usenetServers: UsenetServer[] }>(USENET_SERVERS_QUERY, {})
        .toPromise()
      
      if (result.error) {
        throw new Error(result.error.message)
      }
      
      setServers(result.data?.usenetServers || [])
      setError(null)
    } catch (err) {
      setError(sanitizeError(err))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    loadServers()
  }, [loadServers])

  // Open modal for adding new server
  const handleAdd = () => {
    setEditingServer(null)
    setFormData({
      name: '',
      host: '',
      port: 563,
      useSsl: true,
      username: '',
      password: '',
      connections: 10,
      retentionDays: null,
    })
    onOpen()
  }

  // Open modal for editing server
  const handleEdit = (server: UsenetServer) => {
    setEditingServer(server)
    setFormData({
      name: server.name,
      host: server.host,
      port: server.port,
      useSsl: server.useSsl,
      username: server.username || '',
      password: '',
      connections: server.connections,
      retentionDays: server.retentionDays,
    })
    onOpen()
  }

  // Save server (create or update)
  const handleSave = async () => {
    try {
      setSaving(true)

      if (editingServer) {
        // Update
        const result = await graphqlClient
          .mutation<{ updateUsenetServer: UsenetServerResult }>(
            UPDATE_USENET_SERVER_MUTATION,
            {
              id: editingServer.id,
              input: {
                name: formData.name || undefined,
                host: formData.host || undefined,
                port: formData.port,
                useSsl: formData.useSsl,
                username: formData.username || undefined,
                password: formData.password || undefined,
                connections: formData.connections,
                retentionDays: formData.retentionDays,
              },
            }
          )
          .toPromise()

        if (result.error || !result.data?.updateUsenetServer.success) {
          throw new Error(result.data?.updateUsenetServer.error || result.error?.message || 'Failed to update')
        }

        addToast({ title: 'Server updated', color: 'success' })
      } else {
        // Create
        const result = await graphqlClient
          .mutation<{ createUsenetServer: UsenetServerResult }>(
            CREATE_USENET_SERVER_MUTATION,
            {
              input: {
                name: formData.name,
                host: formData.host,
                port: formData.port,
                useSsl: formData.useSsl,
                username: formData.username || undefined,
                password: formData.password || undefined,
                connections: formData.connections,
                retentionDays: formData.retentionDays,
              },
            }
          )
          .toPromise()

        if (result.error || !result.data?.createUsenetServer.success) {
          throw new Error(result.data?.createUsenetServer.error || result.error?.message || 'Failed to create')
        }

        addToast({ title: 'Server added', color: 'success' })
      }

      onClose()
      loadServers()
    } catch (err) {
      addToast({ title: 'Error', description: sanitizeError(err), color: 'danger' })
    } finally {
      setSaving(false)
    }
  }

  // Delete server
  const handleDelete = async (server: UsenetServer) => {
    if (!confirm(`Delete server "${server.name}"?`)) return

    try {
      const result = await graphqlClient
        .mutation<{ deleteUsenetServer: { success: boolean; error: string | null } }>(
          DELETE_USENET_SERVER_MUTATION,
          { id: server.id }
        )
        .toPromise()

      if (result.error || !result.data?.deleteUsenetServer.success) {
        throw new Error(result.data?.deleteUsenetServer.error || result.error?.message || 'Failed to delete')
      }

      addToast({ title: 'Server deleted', color: 'success' })
      loadServers()
    } catch (err) {
      addToast({ title: 'Error', description: sanitizeError(err), color: 'danger' })
    }
  }

  // Toggle enabled
  const handleToggleEnabled = async (server: UsenetServer) => {
    try {
      const result = await graphqlClient
        .mutation<{ updateUsenetServer: UsenetServerResult }>(
          UPDATE_USENET_SERVER_MUTATION,
          {
            id: server.id,
            input: { enabled: !server.enabled },
          }
        )
        .toPromise()

      if (result.error || !result.data?.updateUsenetServer.success) {
        throw new Error(result.data?.updateUsenetServer.error || result.error?.message)
      }

      loadServers()
    } catch (err) {
      addToast({ title: 'Error', description: sanitizeError(err), color: 'danger' })
    }
  }

  // Move server up/down in priority
  const handleMove = async (index: number, direction: 'up' | 'down') => {
    const newIndex = direction === 'up' ? index - 1 : index + 1
    if (newIndex < 0 || newIndex >= servers.length) return

    const newServers = [...servers]
    const [moved] = newServers.splice(index, 1)
    newServers.splice(newIndex, 0, moved)

    // Update UI optimistically
    setServers(newServers)

    // Save new order
    try {
      const result = await graphqlClient
        .mutation<{ reorderUsenetServers: { success: boolean; error: string | null } }>(
          REORDER_USENET_SERVERS_MUTATION,
          { ids: newServers.map((s) => s.id) }
        )
        .toPromise()

      if (result.error || !result.data?.reorderUsenetServers.success) {
        throw new Error(result.data?.reorderUsenetServers.error || result.error?.message)
      }
    } catch (err) {
      addToast({ title: 'Error reordering', description: sanitizeError(err), color: 'danger' })
      loadServers() // Reload to get correct order
    }
  }

  if (loading) {
    return (
      <div className="flex justify-center items-center h-64">
        <Spinner size="lg" />
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Usenet Servers</h1>
          <p className="text-default-500">Configure your Usenet news server providers</p>
        </div>
        <Button color="primary" startContent={<IconPlus size={16} />} onPress={handleAdd}>
          Add Server
        </Button>
      </div>

      {error && <InlineError message={error} />}

      {servers.length === 0 ? (
        <Card>
          <CardBody className="text-center py-12">
            <IconServer size={48} className="mx-auto text-default-300 mb-4" />
            <p className="text-default-500">No Usenet servers configured</p>
            <p className="text-default-400 text-sm mb-4">
              Add a server to start downloading from Usenet
            </p>
            <Button color="primary" onPress={handleAdd}>
              Add Your First Server
            </Button>
          </CardBody>
        </Card>
      ) : (
        <div className="space-y-3">
          {servers.map((server, index) => (
            <Card key={server.id}>
              <CardBody className="flex flex-row items-center gap-4">
                {/* Priority controls */}
                <div className="flex flex-col gap-0.5">
                  <Button
                    isIconOnly
                    size="sm"
                    variant="light"
                    isDisabled={index === 0}
                    onPress={() => handleMove(index, 'up')}
                  >
                    <IconArrowUp size={14} />
                  </Button>
                  <Button
                    isIconOnly
                    size="sm"
                    variant="light"
                    isDisabled={index === servers.length - 1}
                    onPress={() => handleMove(index, 'down')}
                  >
                    <IconArrowDown size={14} />
                  </Button>
                </div>

                {/* Server info */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-medium truncate">{server.name}</span>
                    {server.useSsl ? (
                      <Tooltip content="SSL/TLS enabled">
                        <IconLock size={14} className="text-green-500" />
                      </Tooltip>
                    ) : (
                      <Tooltip content="No SSL">
                        <IconLockOpen size={14} className="text-amber-500" />
                      </Tooltip>
                    )}
                    {server.errorCount > 0 && (
                      <Chip color="danger" size="sm" variant="flat">
                        {server.errorCount} errors
                      </Chip>
                    )}
                  </div>
                  <div className="text-sm text-default-500">
                    {server.host}:{server.port} • {server.connections} connections
                    {server.retentionDays && ` • ${server.retentionDays} days retention`}
                  </div>
                  {server.lastError && (
                    <div className="text-xs text-danger mt-1 truncate">{server.lastError}</div>
                  )}
                </div>

                {/* Actions */}
                <div className="flex items-center gap-2">
                  <Switch isSelected={server.enabled} onValueChange={() => handleToggleEnabled(server)} size="sm" />
                  <Tooltip content="Edit">
                    <Button isIconOnly size="sm" variant="light" onPress={() => handleEdit(server)}>
                      <IconEdit size={16} />
                    </Button>
                  </Tooltip>
                  <Tooltip content="Delete">
                    <Button
                      isIconOnly
                      size="sm"
                      variant="light"
                      color="danger"
                      onPress={() => handleDelete(server)}
                    >
                      <IconTrash size={16} />
                    </Button>
                  </Tooltip>
                </div>
              </CardBody>
            </Card>
          ))}
        </div>
      )}

      {/* Add/Edit Modal */}
      <Modal isOpen={isOpen} onClose={onClose} size="lg">
        <ModalContent>
          <ModalHeader>{editingServer ? 'Edit Server' : 'Add Server'}</ModalHeader>
          <ModalBody className="gap-4">
            <Input
              label="Name"
              placeholder="My Usenet Provider"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              isRequired
            />
            <div className="flex gap-4">
              <Input
                label="Host"
                placeholder="news.example.com"
                value={formData.host}
                onChange={(e) => setFormData({ ...formData, host: e.target.value })}
                isRequired
                className="flex-1"
              />
              <Input
                label="Port"
                type="number"
                value={formData.port.toString()}
                onChange={(e) => setFormData({ ...formData, port: parseInt(e.target.value) || 563 })}
                className="w-24"
              />
            </div>
            <Switch
              isSelected={formData.useSsl}
              onValueChange={(v) => setFormData({ ...formData, useSsl: v, port: v ? 563 : 119 })}
            >
              Use SSL/TLS
            </Switch>
            <Divider />
            <div className="flex gap-4">
              <Input
                label="Username"
                placeholder="(optional)"
                value={formData.username}
                onChange={(e) => setFormData({ ...formData, username: e.target.value })}
                className="flex-1"
              />
              <Input
                label="Password"
                type="password"
                placeholder={editingServer ? '(unchanged)' : '(optional)'}
                value={formData.password}
                onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                className="flex-1"
              />
            </div>
            <Divider />
            <div className="flex gap-4">
              <Input
                label="Connections"
                type="number"
                value={formData.connections.toString()}
                onChange={(e) => setFormData({ ...formData, connections: parseInt(e.target.value) || 10 })}
                description="Number of simultaneous connections"
                className="flex-1"
              />
              <Input
                label="Retention (days)"
                type="number"
                placeholder="(optional)"
                value={formData.retentionDays?.toString() || ''}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    retentionDays: e.target.value ? parseInt(e.target.value) : null,
                  })
                }
                description="How many days of articles the server keeps"
                className="flex-1"
              />
            </div>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onClose}>
              Cancel
            </Button>
            <Button
              color="primary"
              onPress={handleSave}
              isLoading={saving}
              isDisabled={!formData.name || !formData.host}
            >
              {editingServer ? 'Save' : 'Add'}
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  )
}
