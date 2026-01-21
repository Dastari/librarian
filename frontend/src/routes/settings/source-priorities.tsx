import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import { Card, CardBody, CardHeader } from '@heroui/card'
import { Button } from '@heroui/button'
import { Chip } from '@heroui/chip'
import { Divider } from '@heroui/divider'
import { Switch } from '@heroui/switch'
import { Spinner } from '@heroui/spinner'
import { Tabs, Tab } from '@heroui/tabs'
import { addToast } from '@heroui/toast'
import {
  IconArrowUp,
  IconArrowDown,
  IconDeviceFloppy,
  IconTrash,
  IconCheck,
  IconX,
  IconRss,
  IconDeviceTv,
  IconMovie,
  IconMusic,
  IconBook,
} from '@tabler/icons-react'
import { graphqlClient } from '../../lib/graphql'
import { sanitizeError } from '../../lib/format'
import { InlineError } from '../../components/shared'

export const Route = createFileRoute('/settings/source-priorities')({
  component: SourcePrioritiesPage,
})

// Types
interface SourceRef {
  sourceType: string
  id: string
}

interface SourcePriorityRule {
  id: string
  libraryType: string | null
  libraryId: string | null
  priorityOrder: SourceRef[]
  searchAllSources: boolean
  enabled: boolean
}

interface AvailableSource {
  sourceType: string
  id: string
  name: string
  enabled: boolean
  isHealthy: boolean
}

// GraphQL Queries
const PRIORITY_RULES_QUERY = `
  query SourcePriorityRules {
    sourcePriorityRules {
      id
      libraryType
      libraryId
      priorityOrder {
        sourceType
        id
      }
      searchAllSources
      enabled
    }
  }
`

const AVAILABLE_SOURCES_QUERY = `
  query AvailableSources {
    availableSources {
      sourceType
      id
      name
      enabled
      isHealthy
    }
  }
`

const SET_PRIORITY_RULE_MUTATION = `
  mutation SetSourcePriorityRule($input: SetPriorityRuleInput!) {
    setSourcePriorityRule(input: $input) {
      success
      error
      rule {
        id
        libraryType
        priorityOrder {
          sourceType
          id
        }
        searchAllSources
        enabled
      }
    }
  }
`

const DELETE_PRIORITY_RULE_MUTATION = `
  mutation DeleteSourcePriorityRule($libraryType: String, $libraryId: String) {
    deleteSourcePriorityRule(libraryType: $libraryType, libraryId: $libraryId) {
      success
      error
    }
  }
`

const LIBRARY_TYPES = [
  { key: 'default', label: 'Default', icon: IconRss, color: 'text-default-400' },
  { key: 'tv', label: 'TV Shows', icon: IconDeviceTv, color: 'text-blue-400' },
  { key: 'movies', label: 'Movies', icon: IconMovie, color: 'text-purple-400' },
  { key: 'music', label: 'Music', icon: IconMusic, color: 'text-green-400' },
  { key: 'audiobooks', label: 'Audiobooks', icon: IconBook, color: 'text-amber-400' },
]

function SourcePrioritiesPage() {
  const [rules, setRules] = useState<SourcePriorityRule[]>([])
  const [sources, setSources] = useState<AvailableSource[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [selectedTab, setSelectedTab] = useState('default')
  const [saving, setSaving] = useState(false)

  // Local state for editing
  const [editedOrder, setEditedOrder] = useState<string[]>([])
  const [searchAll, setSearchAll] = useState(false)
  const [hasChanges, setHasChanges] = useState(false)

  // Load data
  const loadData = useCallback(async () => {
    try {
      setLoading(true)
      
      const [rulesResult, sourcesResult] = await Promise.all([
        graphqlClient
          .query<{ sourcePriorityRules: SourcePriorityRule[] }>(PRIORITY_RULES_QUERY, {})
          .toPromise(),
        graphqlClient
          .query<{ availableSources: AvailableSource[] }>(AVAILABLE_SOURCES_QUERY, {})
          .toPromise(),
      ])

      if (rulesResult.error) throw new Error(rulesResult.error.message)
      if (sourcesResult.error) throw new Error(sourcesResult.error.message)

      setRules(rulesResult.data?.sourcePriorityRules || [])
      setSources(sourcesResult.data?.availableSources || [])
      setError(null)
    } catch (err) {
      setError(sanitizeError(err))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    loadData()
  }, [loadData])

  // Get current rule for selected tab
  const getCurrentRule = useCallback(() => {
    const libraryType = selectedTab === 'default' ? null : selectedTab
    return rules.find((r) => 
      (r.libraryType === libraryType || (r.libraryType === null && libraryType === null)) &&
      r.libraryId === null
    )
  }, [rules, selectedTab])

  // Update local state when tab changes
  useEffect(() => {
    const rule = getCurrentRule()
    
    if (rule) {
      setEditedOrder(rule.priorityOrder.map((s) => s.id))
      setSearchAll(rule.searchAllSources)
    } else {
      // Default to all enabled sources
      setEditedOrder(sources.filter((s) => s.enabled).map((s) => s.id))
      setSearchAll(true)
    }
    setHasChanges(false)
  }, [selectedTab, rules, sources, getCurrentRule])

  // Get source by ID
  const getSource = (id: string) => sources.find((s) => s.id === id)

  // Get available sources not in order
  const getUnusedSources = () => {
    return sources.filter((s) => !editedOrder.includes(s.id))
  }

  // Move source up/down
  const handleMove = (index: number, direction: 'up' | 'down') => {
    const newIndex = direction === 'up' ? index - 1 : index + 1
    if (newIndex < 0 || newIndex >= editedOrder.length) return

    const newOrder = [...editedOrder]
    const [moved] = newOrder.splice(index, 1)
    newOrder.splice(newIndex, 0, moved)
    setEditedOrder(newOrder)
    setHasChanges(true)
  }

  // Add source to order
  const handleAdd = (sourceId: string) => {
    setEditedOrder([...editedOrder, sourceId])
    setHasChanges(true)
  }

  // Remove source from order
  const handleRemove = (sourceId: string) => {
    setEditedOrder(editedOrder.filter((id) => id !== sourceId))
    setHasChanges(true)
  }

  // Save changes
  const handleSave = async () => {
    try {
      setSaving(true)

      const libraryType = selectedTab === 'default' ? null : selectedTab

      const result = await graphqlClient
        .mutation<{
          setSourcePriorityRule: { success: boolean; error: string | null }
        }>(SET_PRIORITY_RULE_MUTATION, {
          input: {
            libraryType,
            priorityOrder: editedOrder.map((id) => {
              const source = getSource(id)
              return {
                sourceType: source?.sourceType || 'TORRENT_INDEXER',
                id,
              }
            }),
            searchAllSources: searchAll,
          },
        })
        .toPromise()

      if (result.error || !result.data?.setSourcePriorityRule.success) {
        throw new Error(result.data?.setSourcePriorityRule.error || result.error?.message || 'Failed to save')
      }

      addToast({ title: 'Priority rule saved', color: 'success' })
      setHasChanges(false)
      loadData()
    } catch (err) {
      addToast({ title: 'Error', description: sanitizeError(err), color: 'danger' })
    } finally {
      setSaving(false)
    }
  }

  // Delete rule
  const handleDelete = async () => {
    if (!confirm('Delete this priority rule? It will fall back to the default.')) return

    try {
      const libraryType = selectedTab === 'default' ? null : selectedTab

      const result = await graphqlClient
        .mutation<{ deleteSourcePriorityRule: { success: boolean; error: string | null } }>(
          DELETE_PRIORITY_RULE_MUTATION,
          { libraryType }
        )
        .toPromise()

      if (result.error || !result.data?.deleteSourcePriorityRule.success) {
        throw new Error(result.data?.deleteSourcePriorityRule.error || result.error?.message)
      }

      addToast({ title: 'Rule deleted', color: 'success' })
      loadData()
    } catch (err) {
      addToast({ title: 'Error', description: sanitizeError(err), color: 'danger' })
    }
  }

  // Reset to default
  const handleReset = () => {
    const rule = getCurrentRule()
    if (rule) {
      setEditedOrder(rule.priorityOrder.map((s) => s.id))
      setSearchAll(rule.searchAllSources)
    } else {
      setEditedOrder(sources.filter((s) => s.enabled).map((s) => s.id))
      setSearchAll(true)
    }
    setHasChanges(false)
  }

  if (loading) {
    return (
      <div className="flex justify-center items-center h-64">
        <Spinner size="lg" />
      </div>
    )
  }

  const unusedSources = getUnusedSources()

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Source Priorities</h1>
          <p className="text-default-500">
            Configure which sources to search first when hunting for media
          </p>
        </div>
        <div className="flex gap-2">
          {hasChanges && (
            <Button variant="flat" onPress={handleReset}>
              Reset
            </Button>
          )}
          <Button
            color="primary"
            startContent={<IconDeviceFloppy size={16} />}
            isLoading={saving}
            isDisabled={!hasChanges}
            onPress={handleSave}
          >
            Save Changes
          </Button>
        </div>
      </div>

      {error && <InlineError message={error} />}

      {sources.length === 0 ? (
        <Card>
          <CardBody className="text-center py-12">
            <IconRss size={48} className="mx-auto text-default-300 mb-4" />
            <p className="text-default-500">No sources configured</p>
            <p className="text-default-400 text-sm">
              Add indexers or Usenet providers first
            </p>
          </CardBody>
        </Card>
      ) : (
        <>
          <Tabs
            selectedKey={selectedTab}
            onSelectionChange={(key) => setSelectedTab(key as string)}
            color="primary"
            variant="underlined"
          >
            {LIBRARY_TYPES.map((type) => {
              const Icon = type.icon
              const hasRule = rules.some(
                (r) =>
                  (r.libraryType === (type.key === 'default' ? null : type.key)) &&
                  r.libraryId === null
              )
              return (
                <Tab
                  key={type.key}
                  title={
                    <div className="flex items-center gap-2">
                      <Icon size={16} className={type.color} />
                      <span>{type.label}</span>
                      {hasRule && type.key !== 'default' && (
                        <Chip size="sm" variant="flat" color="primary">
                          Custom
                        </Chip>
                      )}
                    </div>
                  }
                />
              )
            })}
          </Tabs>

          <Card>
            <CardHeader className="flex justify-between items-center">
              <div>
                <h3 className="font-medium">Priority Order</h3>
                <p className="text-sm text-default-500">
                  Drag sources to reorder. Higher sources are searched first.
                </p>
              </div>
              <div className="flex items-center gap-4">
                <Switch
                  isSelected={searchAll}
                  onValueChange={(v) => {
                    setSearchAll(v)
                    setHasChanges(true)
                  }}
                  size="sm"
                >
                  Search all sources
                </Switch>
                {getCurrentRule() && selectedTab !== 'default' && (
                  <Button
                    size="sm"
                    variant="flat"
                    color="danger"
                    startContent={<IconTrash size={14} />}
                    onPress={handleDelete}
                  >
                    Remove Override
                  </Button>
                )}
              </div>
            </CardHeader>
            <Divider />
            <CardBody className="gap-2">
              {editedOrder.length === 0 ? (
                <div className="text-center py-8 text-default-400">
                  No sources in priority order. Add sources below.
                </div>
              ) : (
                editedOrder.map((sourceId, index) => {
                  const source = getSource(sourceId)
                  if (!source) return null

                  return (
                    <div
                      key={sourceId}
                      className="flex items-center gap-3 p-3 bg-content2 rounded-lg"
                    >
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
                          isDisabled={index === editedOrder.length - 1}
                          onPress={() => handleMove(index, 'down')}
                        >
                          <IconArrowDown size={14} />
                        </Button>
                      </div>

                      <span className="w-6 text-center text-default-400 font-mono">
                        {index + 1}
                      </span>

                      <div className="flex-1">
                        <div className="flex items-center gap-2">
                          <span className="font-medium">{source.name}</span>
                          <Chip
                            size="sm"
                            variant="flat"
                            color={source.sourceType === 'USENET_INDEXER' ? 'secondary' : 'primary'}
                          >
                            {source.sourceType === 'USENET_INDEXER' ? 'Usenet' : 'Torrent'}
                          </Chip>
                          {!source.enabled && (
                            <Chip size="sm" variant="flat" color="warning">
                              Disabled
                            </Chip>
                          )}
                          {!source.isHealthy && source.enabled && (
                            <Chip size="sm" variant="flat" color="danger">
                              Error
                            </Chip>
                          )}
                        </div>
                      </div>

                      <Button
                        isIconOnly
                        size="sm"
                        variant="light"
                        color="danger"
                        onPress={() => handleRemove(sourceId)}
                      >
                        <IconX size={14} />
                      </Button>
                    </div>
                  )
                })
              )}

              {!searchAll && editedOrder.length > 0 && (
                <div className="text-sm text-default-400 text-center py-2">
                  Search will stop after finding results from the first source
                </div>
              )}
            </CardBody>
          </Card>

          {unusedSources.length > 0 && (
            <Card>
              <CardHeader>
                <h3 className="font-medium">Available Sources</h3>
              </CardHeader>
              <Divider />
              <CardBody>
                <div className="flex flex-wrap gap-2">
                  {unusedSources.map((source) => (
                    <Chip
                      key={source.id}
                      variant="bordered"
                      onClose={() => handleAdd(source.id)}
                      endContent={
                        <Button
                          isIconOnly
                          size="sm"
                          variant="light"
                          className="min-w-0 w-4 h-4"
                          onPress={() => handleAdd(source.id)}
                        >
                          <IconCheck size={12} />
                        </Button>
                      }
                      className="cursor-pointer"
                      onClick={() => handleAdd(source.id)}
                    >
                      {source.name}
                    </Chip>
                  ))}
                </div>
              </CardBody>
            </Card>
          )}
        </>
      )}
    </div>
  )
}
