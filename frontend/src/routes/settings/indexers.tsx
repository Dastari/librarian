import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useMemo, useCallback } from 'react'
import { Card, CardBody, CardHeader } from '@heroui/card'
import { Button } from '@heroui/button'
import { Chip } from '@heroui/chip'
import { Divider } from '@heroui/divider'
import { Switch } from '@heroui/switch'
import { Spinner } from '@heroui/spinner'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter, useDisclosure } from '@heroui/modal'
import { Input, Textarea } from '@heroui/input'
import { Select, SelectItem } from '@heroui/select'
import { Tooltip } from '@heroui/tooltip'
import { addToast } from '@heroui/toast'
import { Checkbox } from '@heroui/checkbox'
import {
  IconPlus,
  IconTrash,
  IconCheck,
  IconX,
  IconSearch,
  IconAlertTriangle,
  IconEdit,
  IconTestPipe,
  IconDownload,
  IconUsers,
  IconKey,
  IconRefresh,
} from '@tabler/icons-react'
import {
  graphqlClient,
  SECURITY_SETTINGS_QUERY,
  INITIALIZE_ENCRYPTION_KEY_MUTATION,
  REGENERATE_ENCRYPTION_KEY_MUTATION,
  type SecuritySettings,
  type SecuritySettingsResult,
} from '../../lib/graphql'
import { formatBytes, sanitizeError } from '../../lib/format'
import { DataTable, type DataTableColumn, type CardRendererProps, type RowAction } from '../../components/data-table'
import { InlineError } from '../../components/shared'
import type {
  IndexerConfig,
  IndexerTypeInfo,
  IndexerSettingDefinition,
  IndexerTestResult,
  IndexerSearchResultSet,
  TorrentRelease,
} from '../../lib/graphql/types'

// GraphQL response types
interface IndexersQueryResponse {
  indexers: IndexerConfig[]
}

interface AvailableTypesQueryResponse {
  availableIndexerTypes: IndexerTypeInfo[]
}

interface SettingDefinitionsQueryResponse {
  indexerSettingDefinitions: IndexerSettingDefinition[]
}

interface UpdateIndexerResponse {
  updateIndexer: { success: boolean; error: string | null }
}

interface DeleteIndexerResponse {
  deleteIndexer: { success: boolean; error: string | null }
}

interface TestIndexerResponse {
  testIndexer: IndexerTestResult
}

interface CreateIndexerResponse {
  createIndexer: { success: boolean; error: string | null; indexer: IndexerConfig | null }
}

interface SearchIndexersResponse {
  searchIndexers: IndexerSearchResultSet
}

export const Route = createFileRoute('/settings/indexers')({
  component: IndexersSettingsPage,
})

// GraphQL queries and mutations
const INDEXERS_QUERY = `
  query Indexers {
    indexers {
      id
      indexerType
      name
      enabled
      priority
      siteUrl
      isHealthy
      lastError
      errorCount
      lastSuccessAt
      createdAt
      updatedAt
      capabilities {
        supportsSearch
        supportsTvSearch
        supportsMovieSearch
        supportsMusicSearch
        supportsBookSearch
        supportsImdbSearch
        supportsTvdbSearch
      }
    }
  }
`

const AVAILABLE_TYPES_QUERY = `
  query AvailableIndexerTypes {
    availableIndexerTypes {
      id
      name
      description
      trackerType
      language
      siteLink
      requiredCredentials
      isNative
    }
  }
`

const SETTING_DEFINITIONS_QUERY = `
  query IndexerSettingDefinitions($indexerType: String!) {
    indexerSettingDefinitions(indexerType: $indexerType) {
      key
      label
      settingType
      defaultValue
      options {
        value
        label
      }
    }
  }
`

const CREATE_INDEXER_MUTATION = `
  mutation CreateIndexer($input: CreateIndexerInput!) {
    createIndexer(input: $input) {
      success
      error
      indexer {
        id
        name
        enabled
      }
    }
  }
`

const UPDATE_INDEXER_MUTATION = `
  mutation UpdateIndexer($id: String!, $input: UpdateIndexerInput!) {
    updateIndexer(id: $id, input: $input) {
      success
      error
    }
  }
`

const DELETE_INDEXER_MUTATION = `
  mutation DeleteIndexer($id: String!) {
    deleteIndexer(id: $id) {
      success
      error
    }
  }
`

const TEST_INDEXER_MUTATION = `
  mutation TestIndexer($id: String!) {
    testIndexer(id: $id) {
      success
      error
      releasesFound
      elapsedMs
    }
  }
`

const SEARCH_INDEXERS_QUERY = `
  query SearchIndexers($input: IndexerSearchInput!) {
    searchIndexers(input: $input) {
      indexers {
        indexerId
        indexerName
        releases {
          title
          guid
          link
          magnetUri
          size
          sizeFormatted
          seeders
          leechers
          publishDate
          isFreeleech
        }
        elapsedMs
        fromCache
        error
      }
      totalReleases
      totalElapsedMs
    }
  }
`

function IndexersSettingsPage() {
  const [indexers, setIndexers] = useState<IndexerConfig[]>([])
  const [availableTypes, setAvailableTypes] = useState<IndexerTypeInfo[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [testResults, setTestResults] = useState<Record<string, IndexerTestResult>>({})
  const [testingIds, setTestingIds] = useState<Set<string>>(new Set())
  const [editingIndexer, setEditingIndexer] = useState<IndexerConfig | null>(null)
  
  // Search state
  const [searchQuery, setSearchQuery] = useState('')
  const [isSearching, setIsSearching] = useState(false)
  const [searchResults, setSearchResults] = useState<IndexerSearchResultSet | null>(null)

  const { isOpen: isAddOpen, onOpen: onAddOpen, onClose: onAddClose } = useDisclosure()
  const { isOpen: isEditOpen, onOpen: onEditOpen, onClose: onEditClose } = useDisclosure()
  const { isOpen: isDeleteOpen, onOpen: onDeleteOpen, onClose: onDeleteClose } = useDisclosure()
  const [indexerToDelete, setIndexerToDelete] = useState<IndexerConfig | null>(null)

  // Fetch indexers
  const fetchIndexers = async () => {
    try {
      const result = await graphqlClient.query<IndexersQueryResponse>(INDEXERS_QUERY, {}).toPromise()
      if (result.data?.indexers) {
        setIndexers(result.data.indexers)
      }
    } catch (e) {
      console.error('Failed to fetch indexers:', e)
    } finally {
      setIsLoading(false)
    }
  }

  // Fetch available types
  const fetchAvailableTypes = async () => {
    try {
      const result = await graphqlClient.query<AvailableTypesQueryResponse>(AVAILABLE_TYPES_QUERY, {}).toPromise()
      if (result.data?.availableIndexerTypes) {
        setAvailableTypes(result.data.availableIndexerTypes)
      }
    } catch (e) {
      console.error('Failed to fetch available types:', e)
    }
  }

  useEffect(() => {
    fetchIndexers()
    fetchAvailableTypes()
  }, [])

  // Toggle indexer enabled
  const toggleEnabled = async (indexer: IndexerConfig) => {
    const result = await graphqlClient
      .mutation<UpdateIndexerResponse>(UPDATE_INDEXER_MUTATION, {
        id: indexer.id,
        input: { enabled: !indexer.enabled },
      })
      .toPromise()

    if (result.data?.updateIndexer?.success) {
      setIndexers((prev) =>
        prev.map((idx) =>
          idx.id === indexer.id ? { ...idx, enabled: !idx.enabled } : idx
        )
      )
    }
  }

  // Show delete confirmation modal
  const confirmDeleteIndexer = (indexer: IndexerConfig) => {
    setIndexerToDelete(indexer)
    onDeleteOpen()
  }

  // Delete indexer after confirmation
  const deleteIndexer = async () => {
    if (!indexerToDelete) return

    const result = await graphqlClient
      .mutation<DeleteIndexerResponse>(DELETE_INDEXER_MUTATION, { id: indexerToDelete.id })
      .toPromise()

    if (result.data?.deleteIndexer?.success) {
      setIndexers((prev) => prev.filter((idx) => idx.id !== indexerToDelete.id))
      addToast({
        title: 'Indexer Deleted',
        description: `"${indexerToDelete.name}" has been removed`,
        color: 'success',
      })
    } else {
      addToast({
        title: 'Delete Failed',
        description: sanitizeError(result.data?.deleteIndexer?.error || 'Failed to delete indexer'),
        color: 'danger',
      })
    }
    
    setIndexerToDelete(null)
    onDeleteClose()
  }

  // Test indexer
  const testIndexer = async (indexer: IndexerConfig) => {
    setTestingIds((prev) => new Set(prev).add(indexer.id))

    try {
      const result = await graphqlClient
        .mutation<TestIndexerResponse>(TEST_INDEXER_MUTATION, { id: indexer.id })
        .toPromise()

      if (result.data?.testIndexer) {
        setTestResults((prev) => ({
          ...prev,
          [indexer.id]: result.data!.testIndexer,
        }))
      }
    } finally {
      setTestingIds((prev) => {
        const next = new Set(prev)
        next.delete(indexer.id)
        return next
      })
    }
  }

  // Search across all enabled indexers
  const handleSearch = async () => {
    if (!searchQuery.trim()) {
      addToast({
        title: 'Enter a search term',
        description: 'Please enter something to search for',
        color: 'warning',
      })
      return
    }

    const enabledIndexers = indexers.filter(i => i.enabled)
    if (enabledIndexers.length === 0) {
      addToast({
        title: 'No enabled indexers',
        description: 'Enable at least one indexer to search',
        color: 'warning',
      })
      return
    }

    setIsSearching(true)
    setSearchResults(null)

    try {
      const result = await graphqlClient
        .query<SearchIndexersResponse>(SEARCH_INDEXERS_QUERY, {
          input: {
            query: searchQuery,
            limit: 50,
          },
        })
        .toPromise()

      if (result.data?.searchIndexers) {
        setSearchResults(result.data.searchIndexers)
        addToast({
          title: 'Search complete',
          description: `Found ${result.data.searchIndexers.totalReleases} releases in ${result.data.searchIndexers.totalElapsedMs}ms`,
          color: 'success',
        })
      } else if (result.error) {
        addToast({
          title: 'Search failed',
          description: sanitizeError(result.error),
          color: 'danger',
        })
      }
    } catch (e) {
      addToast({
        title: 'Search failed',
        description: sanitizeError(e),
        color: 'danger',
      })
    } finally {
      setIsSearching(false)
    }
  }

  // Handle add complete
  const handleAddComplete = () => {
    onAddClose()
    fetchIndexers()
  }

  // Handle edit
  const handleEdit = (indexer: IndexerConfig) => {
    setEditingIndexer(indexer)
    onEditOpen()
  }

  // Handle edit complete
  const handleEditComplete = () => {
    onEditClose()
    setEditingIndexer(null)
    fetchIndexers()
  }

  const enabledCount = indexers.filter(i => i.enabled).length

  // Column definitions for table view (not used currently but required by DataTable)
  const indexerColumns: DataTableColumn<IndexerConfig>[] = useMemo(() => [
    { key: 'name', label: 'Name' },
    { key: 'indexerType', label: 'Type', width: 120 },
    { key: 'enabled', label: 'Enabled', width: 100 },
  ], [])

  // Row actions for DataTable
  const indexerActions: RowAction<IndexerConfig>[] = useMemo(() => [
    {
      key: 'test',
      label: 'Test connection',
      icon: <IconTestPipe size={16} />,
      onAction: (indexer) => testIndexer(indexer),
      isDisabled: (indexer) => testingIds.has(indexer.id),
      inDropdown: false,
    },
    {
      key: 'edit',
      label: 'Edit settings',
      icon: <IconEdit size={16} />,
      onAction: (indexer) => handleEdit(indexer),
      inDropdown: false,
    },
    {
      key: 'delete',
      label: 'Delete',
      icon: <IconTrash size={16} />,
      color: 'danger',
      isDestructive: true,
      onAction: (indexer) => confirmDeleteIndexer(indexer),
      inDropdown: false,
    },
  ], [testingIds])

  // Card renderer for indexers (wraps IndexerCard with external state)
  const renderIndexerCard = useCallback(({ item: indexer }: CardRendererProps<IndexerConfig>) => (
    <IndexerCard
      indexer={indexer}
      testResult={testResults[indexer.id]}
      isTesting={testingIds.has(indexer.id)}
      onToggleEnabled={() => toggleEnabled(indexer)}
      onEdit={() => handleEdit(indexer)}
      onDelete={() => confirmDeleteIndexer(indexer)}
      onTest={() => testIndexer(indexer)}
    />
  ), [testResults, testingIds])

  // Show loading spinner while fetching initial data
  if (isLoading) {
    return (
      <div className="flex justify-center items-center py-20 w-full">
        <Spinner size="lg" />
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">Indexers</h2>
          <p className="text-default-500 text-sm">
            Configure torrent indexer sources for searching releases
          </p>
        </div>
        <Button
          color="primary"
          startContent={<IconPlus size={16} />}
          onPress={onAddOpen}
        >
          Add Indexer
        </Button>
      </div>

      {/* Search Box */}
      {indexers.length > 0 && (
        <Card>
          <CardBody className="gap-4">
            <Input
              label="Search Indexers"
              labelPlacement="inside"
              variant="flat"
              placeholder="Search for releases across enabled indexers..."
              value={searchQuery}
              onValueChange={setSearchQuery}
              onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
              startContent={<IconSearch size={18} className="text-default-400" />}
              className="flex-1"
              classNames={{
                label: 'text-sm font-medium text-primary!',
              }}
              isDisabled={isSearching || enabledCount === 0}
              endContent={
                <Button
                  size="sm"
                  variant="light"
                  color="primary"
                  className="font-semibold"
                  onPress={handleSearch}
                  isLoading={isSearching}
                  isDisabled={enabledCount === 0}
                >
                  Search
                </Button>
              }
            />
            {enabledCount === 0 && (
              <p className="text-warning-500 text-sm">
                Enable at least one indexer to search
              </p>
            )}
            {enabledCount > 0 && (
              <p className="text-default-400 text-xs">
                Searching {enabledCount} enabled indexer{enabledCount !== 1 ? 's' : ''}
              </p>
            )}
          </CardBody>
        </Card>
      )}

      {/* Search Results */}
      {searchResults && (
        <SearchResultsCard 
          results={searchResults} 
          onClose={() => setSearchResults(null)} 
        />
      )}

      {/* Indexer List */}
      <DataTable<IndexerConfig>
        data={indexers}
        columns={indexerColumns}
        getRowKey={(indexer) => indexer.id}
        isLoading={isLoading}
        defaultViewMode="cards"
        hideToolbar={true}
        cardRenderer={renderIndexerCard}
        cardGridClassName="grid grid-cols-1 gap-4"
        rowActions={indexerActions}
        searchFn={(indexer, term) => 
          indexer.name.toLowerCase().includes(term.toLowerCase()) ||
          indexer.indexerType.toLowerCase().includes(term.toLowerCase())
        }
        searchPlaceholder="Filter indexers..."
        emptyContent={
          <Card>
            <CardBody className="py-12 text-center">
              <IconSearch size={48} className="mx-auto text-default-300 mb-4" />
              <p className="text-default-500">No indexers configured</p>
              <p className="text-default-400 text-sm mt-1">
                Add an indexer to start searching for torrents
              </p>
              <Button
                color="primary"
                className="mt-4"
                startContent={<IconPlus size={16} />}
                onPress={onAddOpen}
              >
                Add Your First Indexer
              </Button>
            </CardBody>
          </Card>
        }
      />

      {/* Security & Encryption */}
      <SecuritySettingsCard />

      {/* Add Indexer Modal */}
      <AddIndexerModal
        isOpen={isAddOpen}
        onClose={onAddClose}
        availableTypes={availableTypes}
        onComplete={handleAddComplete}
      />

      {/* Edit Indexer Modal */}
      {editingIndexer && (
        <EditIndexerModal
          isOpen={isEditOpen}
          onClose={() => {
            onEditClose()
            setEditingIndexer(null)
          }}
          indexer={editingIndexer}
          onComplete={handleEditComplete}
          onTest={() => testIndexer(editingIndexer)}
          testResult={testResults[editingIndexer.id]}
          isTesting={testingIds.has(editingIndexer.id)}
        />
      )}

      {/* Delete Confirmation Modal */}
      <Modal isOpen={isDeleteOpen} onClose={onDeleteClose} size="sm">
        <ModalContent>
          <ModalHeader className="flex flex-col gap-1">
            <span className="flex items-center gap-2">
              <IconTrash size={20} className="text-danger" />
              Delete Indexer
            </span>
          </ModalHeader>
          <ModalBody>
            <p>
              Are you sure you want to delete{' '}
              <span className="font-semibold">{indexerToDelete?.name}</span>?
            </p>
            <p className="text-sm text-default-500">
              This action cannot be undone.
            </p>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onDeleteClose}>
              Cancel
            </Button>
            <Button color="danger" onPress={deleteIndexer}>
              Delete
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  )
}

// Indexer Card Component
interface IndexerCardProps {
  indexer: IndexerConfig
  testResult?: IndexerTestResult
  isTesting: boolean
  onToggleEnabled: () => void
  onEdit: () => void
  onDelete: () => void
  onTest: () => void
}

function IndexerCard({
  indexer,
  testResult,
  isTesting,
  onToggleEnabled,
  onEdit,
  onDelete,
  onTest,
}: IndexerCardProps) {
  return (
    <Card>
      <CardBody className="flex flex-row items-center gap-4 py-4">
        {/* Enable Toggle */}
        <Switch
          isSelected={indexer.enabled}
          onValueChange={onToggleEnabled}
          size="sm"
        />

        {/* Info */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-medium">{indexer.name}</span>
            <Chip size="sm" variant="flat" color="default">
              {indexer.indexerType}
            </Chip>
            {!indexer.isHealthy && (
              <Tooltip content={indexer.lastError || 'Indexer has errors'}>
                <Chip size="sm" color="warning" variant="flat">
                  <IconAlertTriangle size={12} className="mr-1" />
                  {indexer.errorCount} error{indexer.errorCount !== 1 ? 's' : ''}
                </Chip>
              </Tooltip>
            )}
          </div>
          <div className="text-sm text-default-400 flex items-center gap-4 mt-1">
            {indexer.capabilities.supportsTvSearch && <span>TV</span>}
            {indexer.capabilities.supportsMovieSearch && <span>Movies</span>}
            {indexer.capabilities.supportsImdbSearch && <span>IMDB</span>}
            {indexer.lastSuccessAt && (
              <span>
                Last success:{' '}
                {new Date(indexer.lastSuccessAt).toLocaleDateString()}
              </span>
            )}
          </div>
        </div>

        {/* Test Status */}
        <div className="flex items-center gap-2 min-w-[140px] justify-end">
          {isTesting ? (
            <div className="flex items-center gap-2 text-default-400">
              <span className="text-sm">Testing...</span>
            </div>
          ) : testResult ? (
            testResult.success ? (
              <Tooltip content={`Completed in ${testResult.elapsedMs}ms`}>
                <Chip color="success" variant="flat" size="sm">
                  Success
                </Chip>
              </Tooltip>
            ) : (
              <Tooltip content={testResult.error || 'Test failed'}>
                <Chip color="danger" variant="flat" size="sm">
                  Failed
                </Chip>
              </Tooltip>
            )
          ) : null}
        </div>

        {/* Actions */}
        <div className="flex items-center gap-1">
          <Tooltip content="Test connection">
            <Button
              isIconOnly
              size="sm"
              variant="light"
              isDisabled={isTesting}
              onPress={onTest}
            >
              {isTesting ? <Spinner size="sm" /> : <IconTestPipe size={16} />}
            </Button>
          </Tooltip>
          <Tooltip content="Edit settings">
            <Button
              isIconOnly
              size="sm"
              variant="light"
              onPress={onEdit}
            >
              <IconEdit size={16} />
            </Button>
          </Tooltip>
          <Tooltip content="Delete">
            <Button
              isIconOnly
              size="sm"
              variant="light"
              color="danger"
              onPress={onDelete}
            >
              <IconTrash size={16} />
            </Button>
          </Tooltip>
        </div>
      </CardBody>
    </Card>
  )
}

// Add Indexer Modal
interface AddIndexerModalProps {
  isOpen: boolean
  onClose: () => void
  availableTypes: IndexerTypeInfo[]
  onComplete: () => void
}

function AddIndexerModal({
  isOpen,
  onClose,
  availableTypes,
  onComplete,
}: AddIndexerModalProps) {
  const [selectedType, setSelectedType] = useState<string>('')
  const [name, setName] = useState('')
  const [cookie, setCookie] = useState('')
  const [userAgent, setUserAgent] = useState('')
  const [settingDefinitions, setSettingDefinitions] = useState<IndexerSettingDefinition[]>([])
  const [settings, setSettings] = useState<Record<string, string>>({})
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const selectedTypeInfo = availableTypes.find((t) => t.id === selectedType)

  // Fetch setting definitions when type changes
  useEffect(() => {
    if (!selectedType) return

    graphqlClient
      .query<SettingDefinitionsQueryResponse>(SETTING_DEFINITIONS_QUERY, { indexerType: selectedType })
      .toPromise()
      .then((result) => {
        if (result.data?.indexerSettingDefinitions) {
          setSettingDefinitions(result.data.indexerSettingDefinitions)
          // Set defaults
          const defaults: Record<string, string> = {}
          result.data.indexerSettingDefinitions.forEach((def: IndexerSettingDefinition) => {
            if (def.defaultValue) {
              defaults[def.key] = def.defaultValue
            }
          })
          setSettings(defaults)
        }
      })
  }, [selectedType])

  // Reset form when modal closes
  useEffect(() => {
    if (!isOpen) {
      setSelectedType('')
      setName('')
      setCookie('')
      setUserAgent('')
      setSettingDefinitions([])
      setSettings({})
      setError(null)
    }
  }, [isOpen])

  const handleSubmit = async () => {
    if (!selectedType || !name) {
      setError('Please select an indexer type and enter a name')
      return
    }

    setIsSubmitting(true)
    setError(null)

    try {
      const credentials = []
      if (cookie) {
        credentials.push({ credentialType: 'cookie', value: cookie })
      }
      if (userAgent) {
        credentials.push({ credentialType: 'user_agent', value: userAgent })
      }

      const settingsInput = Object.entries(settings).map(([key, value]) => ({
        key,
        value,
      }))

      const result = await graphqlClient
        .mutation<CreateIndexerResponse>(CREATE_INDEXER_MUTATION, {
          input: {
            indexerType: selectedType,
            name,
            credentials,
            settings: settingsInput,
          },
        })
        .toPromise()

      if (result.data?.createIndexer?.success) {
        onComplete()
      } else {
        setError(result.data?.createIndexer?.error || 'Failed to create indexer')
      }
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="2xl">
      <ModalContent>
        <ModalHeader>Add Indexer</ModalHeader>
        <ModalBody>
          {error && <InlineError message={error} className="mb-4" />}

          {/* Indexer Type Selection */}
          <Select
            label="Indexer Type"
            placeholder="Select an indexer"
            selectedKeys={selectedType ? [selectedType] : []}
            onSelectionChange={(keys) => {
              const value = Array.from(keys)[0] as string
              setSelectedType(value)
              const info = availableTypes.find((t) => t.id === value)
              if (info) {
                setName(info.name)
              }
            }}
          >
            {availableTypes.map((type) => (
              <SelectItem key={type.id} textValue={type.name}>
                <div className="flex flex-col">
                  <span className="font-medium">{type.name}</span>
                  <span className="text-xs text-default-400">{type.description}</span>
                </div>
              </SelectItem>
            ))}
          </Select>

          {selectedTypeInfo && (
            <>
              <Divider className="my-4" />

              {/* Name */}
              <Input
                label="Display Name"
                labelPlacement="inside"
                variant="flat"
                placeholder="My Indexer"
                value={name}
                onValueChange={setName}
                classNames={{
                  label: 'text-sm font-medium text-primary!',
                }}
              />

              {/* Credentials */}
              {selectedTypeInfo.requiredCredentials.includes('cookie') && (
                <div className="space-y-2">
                  <Textarea
                    label="Cookie"
                    placeholder="uid=12345; pass=abc123def456; ..."
                    value={cookie}
                    onValueChange={setCookie}
                    minRows={2}
                  />
                  <div className="text-xs text-default-400 bg-content2 p-3 rounded-lg">
                    <p className="font-medium mb-1">How to get your cookie:</p>
                    <ol className="list-decimal list-inside space-y-1">
                      <li>Log into the tracker in your browser</li>
                      <li>Open Developer Tools (F12) → Network tab</li>
                      <li>Refresh the page and click any request</li>
                      <li>Find "Cookie" in Request Headers</li>
                      <li>Copy the <strong>entire value</strong> (e.g., <code className="bg-content3 px-1 rounded">uid=12345; pass=abc123...</code>)</li>
                    </ol>
                    <p className="mt-2 text-default-500">
                      Copy the full cookie string, not individual values.
                    </p>
                  </div>
                </div>
              )}

              {selectedTypeInfo.requiredCredentials.includes('user_agent') && (
                <Input
                  label="User Agent"
                  labelPlacement="inside"
                  variant="flat"
                  placeholder="Mozilla/5.0..."
                  description="Your browser's user agent (optional but recommended)"
                  value={userAgent}
                  onValueChange={setUserAgent}
                  classNames={{
                    label: 'text-sm font-medium text-primary!',
                  }}
                />
              )}

              {/* Optional Settings */}
              {settingDefinitions.length > 0 && (
                <>
                  <Divider className="my-4" />
                  <p className="text-sm text-default-500 mb-2">Optional Settings</p>

                  {settingDefinitions.map((def) => {
                    if (def.settingType === 'checkbox') {
                      return (
                        <Switch
                          key={def.key}
                          isSelected={settings[def.key] === 'true'}
                          onValueChange={(v) =>
                            setSettings((prev) => ({ ...prev, [def.key]: v ? 'true' : 'false' }))
                          }
                        >
                          {def.label}
                        </Switch>
                      )
                    }

                    if (def.settingType === 'select' && def.options) {
                      return (
                        <Select
                          key={def.key}
                          label={def.label}
                          selectedKeys={settings[def.key] ? [settings[def.key]] : []}
                          onSelectionChange={(keys) => {
                            const value = Array.from(keys)[0] as string
                            setSettings((prev) => ({ ...prev, [def.key]: value }))
                          }}
                        >
                          {def.options.map((opt) => (
                            <SelectItem key={opt.value}>{opt.label}</SelectItem>
                          ))}
                        </Select>
                      )
                    }

                    return (
                      <Input
                        key={def.key}
                        label={def.label}
                        labelPlacement="inside"
                        variant="flat"
                        type={def.settingType === 'password' ? 'password' : 'text'}
                        value={settings[def.key] || ''}
                        onValueChange={(v) =>
                          setSettings((prev) => ({ ...prev, [def.key]: v }))
                        }
                        classNames={{
                          label: 'text-sm font-medium text-primary!',
                        }}
                      />
                    )
                  })}
                </>
              )}
            </>
          )}
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            isLoading={isSubmitting}
            isDisabled={!selectedType || !name}
            onPress={handleSubmit}
          >
            Add Indexer
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}

// Edit Indexer Modal
interface EditIndexerModalProps {
  isOpen: boolean
  onClose: () => void
  indexer: IndexerConfig
  onComplete: () => void
  onTest: () => void
  testResult?: IndexerTestResult
  isTesting: boolean
}

function EditIndexerModal({
  isOpen,
  onClose,
  indexer,
  onComplete,
  onTest,
  testResult,
  isTesting,
}: EditIndexerModalProps) {
  const [name, setName] = useState(indexer.name)
  const [priority, setPriority] = useState(indexer.priority.toString())
  const [cookie, setCookie] = useState('')
  const [userAgent, setUserAgent] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Reset form when modal opens with new indexer
  useEffect(() => {
    if (isOpen) {
      setName(indexer.name)
      setPriority(indexer.priority.toString())
      setCookie('')
      setUserAgent('')
      setError(null)
    }
  }, [isOpen, indexer])

  const handleSubmit = async () => {
    if (!name) {
      setError('Name is required')
      return
    }

    setIsSubmitting(true)
    setError(null)

    try {
      const credentials = []
      if (cookie.trim()) {
        credentials.push({ credentialType: 'cookie', value: cookie })
      }
      if (userAgent.trim()) {
        credentials.push({ credentialType: 'user_agent', value: userAgent })
      }

      const input: Record<string, unknown> = {
        name,
        priority: parseInt(priority, 10) || 50,
      }

      // Only include credentials if provided
      if (credentials.length > 0) {
        input.credentials = credentials
      }

      const result = await graphqlClient
        .mutation<UpdateIndexerResponse>(UPDATE_INDEXER_MUTATION, {
          id: indexer.id,
          input,
        })
        .toPromise()

      if (result.data?.updateIndexer?.success) {
        onComplete()
      } else {
        setError(result.data?.updateIndexer?.error || 'Failed to update indexer')
      }
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="2xl">
      <ModalContent>
        <ModalHeader>
          <div className="flex items-center gap-2">
            Edit Indexer
            <Chip size="sm" variant="flat">{indexer.indexerType}</Chip>
          </div>
        </ModalHeader>
        <ModalBody className="space-y-4">
          {error && <InlineError message={error} />}

          {/* Test Result Banner */}
          {testResult && (
            <div className={`px-4 py-3 rounded-lg ${
              testResult.success 
                ? 'bg-success-50 text-success-700' 
                : 'bg-danger-50 text-danger-700'
            }`}>
              {testResult.success ? (
                <div className="flex items-center gap-2">
                  <IconCheck size={18} />
                  <span>Connection successful! Found {testResult.releasesFound} releases in {testResult.elapsedMs}ms</span>
                </div>
              ) : (
                <div className="flex items-center gap-2">
                  <IconX size={18} />
                  <span>Test failed: {testResult.error}</span>
                </div>
              )}
            </div>
          )}

          {/* Name */}
          <Input
            label="Display Name"
            labelPlacement="inside"
            variant="flat"
            placeholder="My Indexer"
            value={name}
            onValueChange={setName}
            classNames={{
              label: 'text-sm font-medium text-primary!',
            }}
          />

          {/* Priority */}
          <Input
            label="Priority"
            labelPlacement="inside"
            variant="flat"
            type="number"
            placeholder="50"
            value={priority}
            onValueChange={setPriority}
            description="Higher priority indexers are searched first (1-100)"
            classNames={{
              label: 'text-sm font-medium text-primary!',
            }}
          />

          <Divider />

          {/* Update Credentials */}
          <div>
            <p className="text-sm font-medium mb-2">Update Credentials</p>
            <p className="text-xs text-default-400 mb-4">
              Leave blank to keep existing credentials. Fill in to replace.
            </p>
          </div>

          <div className="space-y-2">
            <Textarea
              label="Cookie"
              placeholder="uid=12345; pass=abc123def456; ..."
              value={cookie}
              onValueChange={setCookie}
              minRows={2}
            />
            <div className="text-xs text-default-400 bg-content2 p-3 rounded-lg">
              <p className="font-medium mb-1">How to get your cookie:</p>
              <ol className="list-decimal list-inside space-y-1">
                <li>Log into the tracker in your browser</li>
                <li>Open Developer Tools (F12) → Network tab</li>
                <li>Refresh the page and click any request</li>
                <li>Find "Cookie" in Request Headers</li>
                <li>Copy the <strong>entire value</strong> (e.g., <code className="bg-content3 px-1 rounded">uid=12345; pass=abc123...</code>)</li>
              </ol>
            </div>
          </div>

          <Input
            label="User Agent"
            labelPlacement="inside"
            variant="flat"
            placeholder="Mozilla/5.0..."
            description="Your browser's user agent (optional)"
            value={userAgent}
            onValueChange={setUserAgent}
            classNames={{
              label: 'text-sm font-medium text-primary!',
            }}
          />

          {/* Health Status */}
          {!indexer.isHealthy && indexer.lastError && (
            <>
              <Divider />
              <div className="bg-warning-50 px-4 py-3 rounded-lg">
                <div className="flex items-center gap-2 text-warning-700 font-medium mb-1">
                  <IconAlertTriangle size={16} />
                  Last Error ({indexer.errorCount} total)
                </div>
                <p className="text-sm text-warning-600">{indexer.lastError}</p>
              </div>
            </>
          )}
        </ModalBody>
        <ModalFooter>
          <Button
            variant="flat"
            startContent={<IconTestPipe size={16} />}
            isLoading={isTesting}
            onPress={onTest}
          >
            Test Connection
          </Button>
          <div className="flex-1" />
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            isLoading={isSubmitting}
            isDisabled={!name}
            onPress={handleSubmit}
          >
            Save Changes
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}

// Search Results Card Component
interface SearchResultsCardProps {
  results: IndexerSearchResultSet
  onClose: () => void
}

// Extended release type with indexer info
interface FlattenedRelease extends TorrentRelease {
  indexerName: string
  indexerId: string
  _uniqueKey: string // Composite key for uniqueness
}

// Column definitions for search results table
const searchResultColumns: DataTableColumn<FlattenedRelease>[] = [
  {
    key: 'title',
    label: 'Title',
    render: (release) => (
      <div className="flex flex-col">
        <span className="font-medium text-sm truncate max-w-md" title={release.title}>
          {release.title}
        </span>
        <div className="flex items-center gap-2 mt-1">
          {release.isFreeleech && (
            <Chip size="sm" color="success" variant="flat">
              Freeleech
            </Chip>
          )}
          <span className="text-xs text-default-400">
            {new Date(release.publishDate).toLocaleDateString()}
          </span>
        </div>
      </div>
    ),
    sortFn: (a, b) => a.title.localeCompare(b.title),
  },
  {
    key: 'size',
    label: 'Size',
    width: 100,
    render: (release) => (
      <span className="text-sm">{release.sizeFormatted || formatBytes(release.size || 0)}</span>
    ),
    sortFn: (a, b) => (a.size || 0) - (b.size || 0),
  },
  {
    key: 'seeders',
    label: 'Seeds',
    width: 80,
    render: (release) => (
      <div className="flex items-center gap-1">
        <IconUsers size={14} className="text-success-500" />
        <span className="text-sm text-success-600">{release.seeders ?? '-'}</span>
      </div>
    ),
    sortFn: (a, b) => (a.seeders || 0) - (b.seeders || 0),
  },
  {
    key: 'leechers',
    label: 'Leech',
    width: 80,
    render: (release) => (
      <div className="flex items-center gap-1">
        <IconUsers size={14} className="text-danger-400" />
        <span className="text-sm text-danger-500">{release.leechers ?? '-'}</span>
      </div>
    ),
    sortFn: (a, b) => (a.leechers || 0) - (b.leechers || 0),
  },
  {
    key: 'indexerName',
    label: 'Indexer',
    width: 130,
    render: (release) => (
      <Chip size="sm" variant="flat">{release.indexerName}</Chip>
    ),
    sortFn: (a, b) => a.indexerName.localeCompare(b.indexerName),
  },
]

function SearchResultsCard({ results, onClose }: SearchResultsCardProps) {
  // Flatten all releases with indexer info
  const allReleases = useMemo<FlattenedRelease[]>(() => {
    let idx = 0
    return results.indexers.flatMap(indexer =>
      indexer.releases.map(release => ({
        ...release,
        indexerName: indexer.indexerName,
        indexerId: indexer.indexerId,
        _uniqueKey: `${indexer.indexerId}-${release.guid}-${idx++}`,
      }))
    )
  }, [results])

  // Row actions for download
  const releaseActions = useMemo<RowAction<FlattenedRelease>[]>(() => [
    {
      key: 'download',
      label: 'Download',
      icon: <IconDownload size={16} />,
      onAction: (release) => {
        const url = release.magnetUri || release.link
        if (url) {
          window.open(url, '_blank')
        }
      },
      isVisible: (release) => !!(release.link || release.magnetUri),
      inDropdown: false,
    },
  ], [])

  return (
    <Card>
      <CardHeader className="flex items-center justify-between">
        <div>
          <p className="text-lg font-semibold">Search Results</p>
          <p className="text-small text-default-500">
            Found {results.totalReleases} releases across {results.indexers.length} indexer{results.indexers.length !== 1 ? 's' : ''} in {results.totalElapsedMs}ms
          </p>
        </div>
        <Button size="sm" variant="light" onPress={onClose}>
          <IconX size={16} />
          Close
        </Button>
      </CardHeader>
      <Divider />
      <CardBody className="p-0">
        {/* Per-indexer status */}
        <div className="flex gap-2 flex-wrap px-4 py-3 bg-content2">
          {results.indexers.map((indexer) => (
            <Chip
              key={indexer.indexerId}
              size="sm"
              variant="flat"
              color={indexer.error ? 'danger' : 'success'}
            >
              {indexer.indexerName}: {indexer.error ? 'Error' : `${indexer.releases.length} results`} ({indexer.elapsedMs}ms)
            </Chip>
          ))}
        </div>
        <Divider />

        <div className="p-4">
          <DataTable<FlattenedRelease>
            data={allReleases}
            columns={searchResultColumns}
            getRowKey={(release) => release._uniqueKey}
            defaultSortColumn="seeders"
            defaultSortDirection="desc"
            rowActions={releaseActions}
            paginationMode="pagination"
            defaultPageSize={20}
            pageSizeOptions={[10, 20, 50, 100]}
            searchFn={(release, term) =>
              release.title.toLowerCase().includes(term.toLowerCase()) ||
              release.indexerName.toLowerCase().includes(term.toLowerCase())
            }
            searchPlaceholder="Filter results..."
            isCompact
            emptyContent={
              <div className="py-12 text-center">
                <IconSearch size={48} className="mx-auto text-default-300 mb-4" />
                <p className="text-default-500">No releases found</p>
              </div>
            }
          />
        </div>
      </CardBody>
    </Card>
  )
}

// Security Settings Card Component
function SecuritySettingsCard() {
  const [settings, setSettings] = useState<SecuritySettings | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isInitializing, setIsInitializing] = useState(false)
  const [isRegenerating, setIsRegenerating] = useState(false)
  const [confirmRegenerate, setConfirmRegenerate] = useState(false)
  
  const { isOpen, onOpen, onClose } = useDisclosure()

  const fetchSecuritySettings = useCallback(async () => {
    try {
      const result = await graphqlClient
        .query<{ securitySettings: SecuritySettings }>(SECURITY_SETTINGS_QUERY, {})
        .toPromise()
      if (result.data?.securitySettings) {
        setSettings(result.data.securitySettings)
      }
    } catch (e) {
      console.error('Failed to fetch security settings:', e)
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchSecuritySettings()
  }, [fetchSecuritySettings])

  const handleInitialize = async () => {
    setIsInitializing(true)
    try {
      const result = await graphqlClient
        .mutation<{ initializeEncryptionKey: SecuritySettingsResult }>(
          INITIALIZE_ENCRYPTION_KEY_MUTATION,
          {}
        )
        .toPromise()

      if (result.data?.initializeEncryptionKey.success) {
        setSettings(result.data.initializeEncryptionKey.settings)
        addToast({
          title: 'Encryption Key Initialized',
          description: 'The encryption key has been created.',
          color: 'success',
        })
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(result.data?.initializeEncryptionKey.error || 'Failed to initialize key'),
          color: 'danger',
        })
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    } finally {
      setIsInitializing(false)
    }
  }

  const handleRegenerate = async () => {
    if (!confirmRegenerate) {
      addToast({
        title: 'Confirmation Required',
        description: 'You must confirm that you understand the consequences.',
        color: 'warning',
      })
      return
    }

    setIsRegenerating(true)
    try {
      const result = await graphqlClient
        .mutation<{ regenerateEncryptionKey: SecuritySettingsResult }>(
          REGENERATE_ENCRYPTION_KEY_MUTATION,
          { input: { confirmInvalidation: true } }
        )
        .toPromise()

      if (result.data?.regenerateEncryptionKey.success) {
        setSettings(result.data.regenerateEncryptionKey.settings)
        onClose()
        setConfirmRegenerate(false)
        addToast({
          title: 'Encryption Key Regenerated',
          description: 'All existing indexer credentials are now invalid. Please re-enter them.',
          color: 'warning',
        })
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(result.data?.regenerateEncryptionKey.error || 'Failed to regenerate key'),
          color: 'danger',
        })
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    } finally {
      setIsRegenerating(false)
    }
  }

  if (isLoading) {
    return (
      <Card>
        <CardBody className="flex items-center justify-center py-8">
          <Spinner size="sm" />
        </CardBody>
      </Card>
    )
  }

  return (
    <>
      <Card>
        <CardHeader>
          <p className="font-semibold">Security & Encryption</p>
        </CardHeader>
        <Divider />
        <CardBody className="gap-4">
          {/* Encryption Key Status */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <IconKey size={20} className="text-default-400" />
              <div>
                <p className="font-medium">Indexer Encryption Key</p>
                <p className="text-xs text-default-400">
                  Used to encrypt indexer credentials (cookies, API keys)
                </p>
              </div>
            </div>
            <div className="flex items-center gap-3">
              {settings?.encryptionKeySet ? (
                <>
                  <Chip color="success" variant="flat" size="sm">
                    Active
                  </Chip>
                  <code className="text-xs bg-default-100 px-2 py-1 rounded font-mono">
                    {settings.encryptionKeyPreview}
                  </code>
                </>
              ) : (
                <Chip color="warning" variant="flat" size="sm">
                  Not Set
                </Chip>
              )}
            </div>
          </div>

          <Divider />

          {/* Actions */}
          <div className="flex gap-3">
            {!settings?.encryptionKeySet && (
              <Button
                color="primary"
                startContent={<IconKey size={16} />}
                onPress={handleInitialize}
                isLoading={isInitializing}
              >
                Initialize Key
              </Button>
            )}
            {settings?.encryptionKeySet && (
              <Button
                color="danger"
                variant="flat"
                startContent={<IconRefresh size={16} />}
                onPress={onOpen}
              >
                Regenerate Key
              </Button>
            )}
          </div>

          {/* Warning */}
          <div className="bg-warning-50 dark:bg-warning-900/20 px-4 py-3 rounded-lg">
            <div className="flex items-start gap-2">
              <IconAlertTriangle size={18} className="text-warning-600 mt-0.5 shrink-0" />
              <div className="text-sm text-warning-700 dark:text-warning-400">
                <p className="font-medium mb-1">Important</p>
                <p>
                  The encryption key is used to secure sensitive indexer credentials stored in the
                  database. If you regenerate this key, <strong>all existing indexer credentials
                  will become invalid</strong> and you will need to re-enter them.
                </p>
              </div>
            </div>
          </div>
        </CardBody>
      </Card>

      {/* Regenerate Confirmation Modal */}
      <Modal isOpen={isOpen} onClose={onClose}>
        <ModalContent>
          <ModalHeader className="flex items-center gap-2 text-danger">
            <IconAlertTriangle size={20} />
            Regenerate Encryption Key
          </ModalHeader>
          <ModalBody>
            <p className="text-default-600">
              This action will generate a new encryption key. All existing indexer credentials
              will be invalidated and you will need to re-enter cookies, API keys, and other
              credentials for all configured indexers.
            </p>
            <div className="mt-4">
              <Checkbox
                isSelected={confirmRegenerate}
                onValueChange={setConfirmRegenerate}
                color="danger"
              >
                I understand that all indexer credentials will be invalidated
              </Checkbox>
            </div>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onClose}>
              Cancel
            </Button>
            <Button
              color="danger"
              onPress={handleRegenerate}
              isLoading={isRegenerating}
              isDisabled={!confirmRegenerate}
            >
              Regenerate Key
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </>
  )
}
