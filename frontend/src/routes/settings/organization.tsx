import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import { Card, CardBody, CardHeader } from '@heroui/card'
import { Tooltip } from '@heroui/tooltip'
import { Button } from '@heroui/button'
import { Chip } from '@heroui/chip'
import { Spinner } from '@heroui/spinner'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter, useDisclosure } from '@heroui/modal'
import { Input, Textarea } from '@heroui/input'
import { addToast } from '@heroui/toast'
import {
  IconPlus,
  IconTrash,
  IconStarFilled,
  IconTemplate,
} from '@tabler/icons-react'
import {
  graphqlClient,
  NAMING_PATTERNS_QUERY,
  CREATE_NAMING_PATTERN_MUTATION,
  DELETE_NAMING_PATTERN_MUTATION,
  SET_DEFAULT_NAMING_PATTERN_MUTATION,
  type NamingPattern,
} from '../../lib/graphql'
import { DataTable, type DataTableColumn, type CardRendererProps, type RowAction } from '../../components/data-table'
import { previewNamingPattern } from '../../lib/format'

// GraphQL response types
interface NamingPatternsQueryResponse {
  namingPatterns: NamingPattern[]
}

interface CreateNamingPatternResponse {
  createNamingPattern: {
    success: boolean
    namingPattern: NamingPattern | null
    error: string | null
  }
}

interface DeleteNamingPatternResponse {
  deleteNamingPattern: {
    success: boolean
    error: string | null
  }
}

interface SetDefaultNamingPatternResponse {
  setDefaultNamingPattern: {
    success: boolean
    error: string | null
  }
}

export const Route = createFileRoute('/settings/organization')({
  component: OrganizationSettingsPage,
})

function OrganizationSettingsPage() {
  const [patterns, setPatterns] = useState<NamingPattern[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Modal state
  const { isOpen: isAddOpen, onOpen: onAddOpen, onClose: onAddClose } = useDisclosure()
  const { isOpen: isDeleteOpen, onOpen: onDeleteOpen, onClose: onDeleteClose } = useDisclosure()
  
  const [selectedPattern, setSelectedPattern] = useState<NamingPattern | null>(null)
  const [formData, setFormData] = useState({ name: '', pattern: '', description: '' })
  const [isSaving, setIsSaving] = useState(false)

  // Fetch patterns
  const fetchPatterns = useCallback(async () => {
    try {
      setIsLoading(true)
      const result = await graphqlClient
        .query<NamingPatternsQueryResponse>(NAMING_PATTERNS_QUERY, {})
        .toPromise()

      if (result.error) {
        throw new Error(result.error.message)
      }

      if (result.data?.namingPatterns) {
        setPatterns(result.data.namingPatterns)
      }
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load naming patterns')
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchPatterns()
  }, [fetchPatterns])

  // Create pattern
  const handleCreate = async () => {
    if (!formData.name.trim() || !formData.pattern.trim()) {
      addToast({
        title: 'Validation Error',
        description: 'Name and pattern are required',
        color: 'danger',
      })
      return
    }

    setIsSaving(true)
    try {
      const result = await graphqlClient
        .mutation<CreateNamingPatternResponse>(CREATE_NAMING_PATTERN_MUTATION, {
          input: {
            name: formData.name.trim(),
            pattern: formData.pattern.trim(),
            description: formData.description.trim() || null,
          },
        })
        .toPromise()

      if (result.data?.createNamingPattern.success) {
        addToast({
          title: 'Pattern Created',
          description: `"${formData.name}" has been added`,
          color: 'success',
        })
        onAddClose()
        setFormData({ name: '', pattern: '', description: '' })
        fetchPatterns()
      } else {
        throw new Error(result.data?.createNamingPattern.error || 'Failed to create pattern')
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: e instanceof Error ? e.message : 'Failed to create pattern',
        color: 'danger',
      })
    } finally {
      setIsSaving(false)
    }
  }

  // Delete pattern
  const handleDelete = async () => {
    if (!selectedPattern) return

    setIsSaving(true)
    try {
      const result = await graphqlClient
        .mutation<DeleteNamingPatternResponse>(DELETE_NAMING_PATTERN_MUTATION, {
          id: selectedPattern.id,
        })
        .toPromise()

      if (result.data?.deleteNamingPattern.success) {
        addToast({
          title: 'Pattern Deleted',
          description: `"${selectedPattern.name}" has been removed`,
          color: 'success',
        })
        onDeleteClose()
        setSelectedPattern(null)
        fetchPatterns()
      } else {
        throw new Error(result.data?.deleteNamingPattern.error || 'Failed to delete pattern')
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: e instanceof Error ? e.message : 'Failed to delete pattern',
        color: 'danger',
      })
    } finally {
      setIsSaving(false)
    }
  }

  // Set default pattern
  const handleSetDefault = async (pattern: NamingPattern) => {
    try {
      const result = await graphqlClient
        .mutation<SetDefaultNamingPatternResponse>(SET_DEFAULT_NAMING_PATTERN_MUTATION, {
          id: pattern.id,
        })
        .toPromise()

      if (result.data?.setDefaultNamingPattern.success) {
        addToast({
          title: 'Default Updated',
          description: `"${pattern.name}" is now the default pattern`,
          color: 'success',
        })
        fetchPatterns()
      } else {
        throw new Error(result.data?.setDefaultNamingPattern.error || 'Failed to set default')
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: e instanceof Error ? e.message : 'Failed to set default',
        color: 'danger',
      })
    }
  }

  // Table columns
  const columns: DataTableColumn<NamingPattern>[] = [
    {
      key: 'name',
      label: 'Name',
      sortable: true,
      render: (pattern) => (
        <div className="flex items-center gap-2">
          <IconTemplate size={16} className="text-amber-400" />
          <span className="font-medium">{pattern.name}</span>
          {pattern.isDefault && (
            <Chip size="sm" color="primary" variant="flat">
              Default
            </Chip>
          )}
          {pattern.isSystem && (
            <Chip size="sm" variant="flat" className="text-default-500">
              System
            </Chip>
          )}
        </div>
      ),
    },
    {
      key: 'pattern',
      label: 'Pattern',
      render: (pattern) => (
        <Tooltip
          content={
            <code className="text-xs font-mono">{pattern.pattern}</code>
          }
          delay={300}
        >
          <code className="text-xs bg-default-100 px-2 py-1 rounded font-mono text-default-600 break-all cursor-help">
            {previewNamingPattern(pattern.pattern)}
          </code>
        </Tooltip>
      ),
    },
    {
      key: 'description',
      label: 'Description',
      render: (pattern) => (
        <span className="text-sm text-default-500">
          {pattern.description || '-'}
        </span>
      ),
    },
  ]

  // Row actions
  const rowActions: RowAction<NamingPattern>[] = [
    {
      key: 'set-default',
      label: 'Set as Default',
      icon: <IconStarFilled size={16} />,
      color: 'warning',
      isVisible: (pattern: NamingPattern) => !pattern.isDefault,
      onAction: handleSetDefault,
    },
    {
      key: 'delete',
      label: 'Delete',
      icon: <IconTrash size={16} />,
      color: 'danger',
      isVisible: (pattern: NamingPattern) => !pattern.isSystem,
      onAction: (pattern: NamingPattern) => {
        setSelectedPattern(pattern)
        onDeleteOpen()
      },
    },
  ]

  // Card renderer for mobile view
  const cardRenderer = ({ item, actions }: CardRendererProps<NamingPattern>) => (
    <Card className="w-full">
      <CardBody className="p-4">
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-2">
              <IconTemplate size={18} className="text-amber-400" />
              <span className="font-medium">{item.name}</span>
              {item.isDefault && (
                <Chip size="sm" color="primary" variant="flat">
                  Default
                </Chip>
              )}
            </div>
            <code className="text-xs bg-default-100 px-2 py-1 rounded font-mono text-default-600 block mb-2 break-all">
              {item.pattern}
            </code>
            {item.description && (
              <p className="text-sm text-default-500">{item.description}</p>
            )}
          </div>
          <div className="flex gap-1">
            {actions
              .filter((action) => !action.isVisible || action.isVisible(item))
              .map((action) => (
                <Button
                  key={action.key}
                  size="sm"
                  isIconOnly
                  variant="light"
                  color={action.color}
                  onPress={() => action.onAction(item)}
                  isDisabled={action.isDisabled?.(item)}
                >
                  {action.icon}
                </Button>
              ))}
          </div>
        </div>
      </CardBody>
    </Card>
  )


  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Spinner size="lg" />
      </div>
    )
  }

  if (error) {
    return (
      <Card>
        <CardBody className="text-center py-8">
          <p className="text-danger mb-4">{error}</p>
          <Button color="primary" onPress={fetchPatterns}>
            Retry
          </Button>
        </CardBody>
      </Card>
    )
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-xl font-semibold">File Organization</h2>
        <p className="text-default-500 text-sm mt-1">
          Configure how media files are renamed and organized into folders
        </p>
      </div>

      {/* Info Card */}
      <Card>
        <CardHeader className="pb-2">
          <h3 className="text-sm font-medium">Pattern Variables</h3>
        </CardHeader>
        <CardBody className="pt-0">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-2 text-xs">
            <div className="bg-default-100 px-2 py-1 rounded">
              <code className="text-primary">{'{show}'}</code> - Show name
            </div>
            <div className="bg-default-100 px-2 py-1 rounded">
              <code className="text-primary">{'{year}'}</code> - Premiere year
            </div>
            <div className="bg-default-100 px-2 py-1 rounded">
              <code className="text-primary">{'{season}'}</code> - Season number
            </div>
            <div className="bg-default-100 px-2 py-1 rounded">
              <code className="text-primary">{'{season:02}'}</code> - Padded season
            </div>
            <div className="bg-default-100 px-2 py-1 rounded">
              <code className="text-primary">{'{episode}'}</code> - Episode number
            </div>
            <div className="bg-default-100 px-2 py-1 rounded">
              <code className="text-primary">{'{episode:02}'}</code> - Padded episode
            </div>
            <div className="bg-default-100 px-2 py-1 rounded">
              <code className="text-primary">{'{title}'}</code> - Episode title
            </div>
            <div className="bg-default-100 px-2 py-1 rounded">
              <code className="text-primary">{'{ext}'}</code> - File extension
            </div>
          </div>
        </CardBody>
      </Card>

      {/* Patterns Table */}
      <DataTable
        data={patterns}
        columns={columns}
        getRowKey={(pattern) => pattern.id}
        rowActions={rowActions}
        cardRenderer={cardRenderer}
        toolbarContent={
          <Tooltip content="Add Pattern">
            <Button
              color="primary"
              size="sm"
              isIconOnly
              onPress={() => {
                setFormData({ name: '', pattern: '', description: '' })
                onAddOpen()
              }}
            >
              <IconPlus size={16} />
            </Button>
          </Tooltip>
        }
        toolbarContentPosition="end"
        emptyContent={
          <div className="text-center py-8">
            <IconTemplate size={48} className="mx-auto text-default-300 mb-4" />
            <p className="text-default-500">No naming patterns found</p>
            <Tooltip content="Add Pattern">
              <Button
                color="primary"
                size="sm"
                isIconOnly
                className="mt-4"
                onPress={onAddOpen}
              >
                <IconPlus size={16} />
              </Button>
            </Tooltip>
          </div>
        }
      />

      {/* Add Pattern Modal */}
      <Modal isOpen={isAddOpen} onClose={onAddClose} size="lg">
        <ModalContent>
          <ModalHeader>Add Naming Pattern</ModalHeader>
          <ModalBody>
            <div className="space-y-4">
              <Input
                label="Name"
                labelPlacement="inside"
                variant="flat"
                placeholder="e.g., My Custom Pattern"
                value={formData.name}
                onValueChange={(value) => setFormData({ ...formData, name: value })}
                isRequired
                classNames={{
                  label: 'text-sm font-medium text-primary!',
                }}
              />
              <Textarea
                label="Pattern"
                placeholder="{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}"
                value={formData.pattern}
                onValueChange={(value) => setFormData({ ...formData, pattern: value })}
                isRequired
                classNames={{
                  input: 'font-mono text-sm',
                }}
              />
              <Input
                label="Description / Example"
                labelPlacement="inside"
                variant="flat"
                placeholder="e.g., Show/Season 01/Show - S01E01 - Title.mkv"
                value={formData.description}
                onValueChange={(value) => setFormData({ ...formData, description: value })}
                classNames={{
                  label: 'text-sm font-medium text-primary!',
                }}
              />
              {formData.pattern && (
                <div className="bg-default-100 p-3 rounded">
                  <p className="text-xs text-default-500 mb-1">Preview:</p>
                  <code className="text-sm font-mono break-all">
                    {previewNamingPattern(formData.pattern)}
                  </code>
                </div>
              )}
            </div>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onAddClose}>
              Cancel
            </Button>
            <Button
              color="primary"
              onPress={handleCreate}
              isLoading={isSaving}
              isDisabled={!formData.name.trim() || !formData.pattern.trim()}
            >
              Create Pattern
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>

      {/* Delete Confirmation Modal */}
      <Modal isOpen={isDeleteOpen} onClose={onDeleteClose}>
        <ModalContent>
          <ModalHeader>Delete Pattern</ModalHeader>
          <ModalBody>
            <p>
              Are you sure you want to delete <strong>"{selectedPattern?.name}"</strong>?
            </p>
            <p className="text-sm text-default-500 mt-2">
              This action cannot be undone. Libraries using this pattern will fall back to the default.
            </p>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onDeleteClose}>
              Cancel
            </Button>
            <Button color="danger" onPress={handleDelete} isLoading={isSaving}>
              Delete
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  )
}
