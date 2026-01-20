import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useMemo } from 'react'
import { Input, Textarea } from '@heroui/input'
import { Switch } from '@heroui/switch'
import { Button } from '@heroui/button'
import { Card, CardBody, CardHeader } from '@heroui/card'
import { Divider } from '@heroui/divider'
import { Select, SelectItem } from '@heroui/select'
import { Chip } from '@heroui/chip'
import { Spinner } from '@heroui/spinner'
import { Accordion, AccordionItem } from '@heroui/accordion'
import { addToast } from '@heroui/toast'
import {
  graphqlClient,
  LLM_PARSER_SETTINGS_QUERY,
  type LlmParserSettings,
  type SettingsResult,
  type OllamaConnectionResult,
  type TestFilenameParserResult,
  type FilenameParseResult,
} from '../../lib/graphql'
import { SettingsHeader } from '../../components/shared'
import { sanitizeError } from '../../lib/format'
import {
  IconBrain,
  IconServer,
  IconTestPipe,
  IconRefresh,
  IconCheck,
  IconX,
  IconClock,
  IconMovie,
  IconDeviceTv,
  IconMusic,
  IconBook,
} from '@tabler/icons-react'

export const Route = createFileRoute('/settings/parser')({
  component: ParserSettingsPage,
})

const UPDATE_LLM_PARSER_SETTINGS_MUTATION = `
  mutation UpdateLlmParserSettings($input: UpdateLlmParserSettingsInput!) {
    updateLlmParserSettings(input: $input) {
      success
      error
    }
  }
`

const TEST_OLLAMA_CONNECTION_MUTATION = `
  mutation TestOllamaConnection($url: String) {
    testOllamaConnection(url: $url) {
      success
      availableModels
      error
    }
  }
`

const TEST_FILENAME_PARSER_MUTATION = `
  mutation TestFilenameParser($filename: String!) {
    testFilenameParser(filename: $filename) {
      regexResult {
        mediaType
        title
        year
        season
        episode
        episodeEnd
        resolution
        source
        videoCodec
        audio
        hdr
        releaseGroup
        edition
        completeSeries
        confidence
      }
      regexTimeMs
      llmResult {
        mediaType
        title
        year
        season
        episode
        episodeEnd
        resolution
        source
        videoCodec
        audio
        hdr
        releaseGroup
        edition
        completeSeries
        confidence
      }
      llmTimeMs
      llmError
    }
  }
`

function ParserSettingsPage() {
  const [originalSettings, setOriginalSettings] = useState<LlmParserSettings | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isSaving, setIsSaving] = useState(false)
  const [isTesting, setIsTesting] = useState(false)
  const [isParsing, setIsParsing] = useState(false)

  // Form state
  const [enabled, setEnabled] = useState(false)
  const [ollamaUrl, setOllamaUrl] = useState('http://localhost:11434')
  const [ollamaModel, setOllamaModel] = useState('qwen2.5-coder:7b')
  const [timeoutSeconds, setTimeoutSeconds] = useState(30)
  const [temperature, setTemperature] = useState(0.1)
  const [maxTokens, setMaxTokens] = useState(256)
  const [promptTemplate, setPromptTemplate] = useState('')
  const [confidenceThreshold, setConfidenceThreshold] = useState(0.7)
  // Library-type-specific models
  const [modelMovies, setModelMovies] = useState<string>('')
  const [modelTv, setModelTv] = useState<string>('')
  const [modelMusic, setModelMusic] = useState<string>('')
  const [modelAudiobooks, setModelAudiobooks] = useState<string>('')

  // Connection test state
  const [availableModels, setAvailableModels] = useState<string[]>([])
  const [connectionStatus, setConnectionStatus] = useState<'untested' | 'success' | 'error'>('untested')
  const [connectionError, setConnectionError] = useState<string | null>(null)

  // Parser test state
  const [testFilename, setTestFilename] = useState('The.Matrix.1999.REMASTERED.2160p.UHD.BluRay.x265.10bit.HDR.TrueHD.7.1.Atmos-FGT')
  const [parseResult, setParseResult] = useState<TestFilenameParserResult | null>(null)

  // Track changes
  const hasChanges = useMemo(() => {
    if (!originalSettings) return false
    return (
      enabled !== originalSettings.enabled ||
      ollamaUrl !== originalSettings.ollamaUrl ||
      ollamaModel !== originalSettings.ollamaModel ||
      timeoutSeconds !== originalSettings.timeoutSeconds ||
      temperature !== originalSettings.temperature ||
      maxTokens !== originalSettings.maxTokens ||
      promptTemplate !== originalSettings.promptTemplate ||
      confidenceThreshold !== originalSettings.confidenceThreshold ||
      modelMovies !== (originalSettings.modelMovies || '') ||
      modelTv !== (originalSettings.modelTv || '') ||
      modelMusic !== (originalSettings.modelMusic || '') ||
      modelAudiobooks !== (originalSettings.modelAudiobooks || '')
    )
  }, [originalSettings, enabled, ollamaUrl, ollamaModel, timeoutSeconds, temperature, maxTokens, promptTemplate, confidenceThreshold, modelMovies, modelTv, modelMusic, modelAudiobooks])

  const fetchSettings = useCallback(async () => {
    try {
      const result = await graphqlClient.query<{ llmParserSettings: LlmParserSettings }>(LLM_PARSER_SETTINGS_QUERY, {}).toPromise()
      console.log('[Parser Settings] Query result:', result)
      if (result.data?.llmParserSettings) {
        const s = result.data.llmParserSettings
        console.log('[Parser Settings] Loaded settings:', s)
        setOriginalSettings(s)
        setEnabled(s.enabled)
        setOllamaUrl(s.ollamaUrl)
        setOllamaModel(s.ollamaModel)
        setTimeoutSeconds(s.timeoutSeconds)
        setTemperature(s.temperature)
        setMaxTokens(s.maxTokens)
        setPromptTemplate(s.promptTemplate)
        setConfidenceThreshold(s.confidenceThreshold)
        // Library-type-specific models
        setModelMovies(s.modelMovies || '')
        setModelTv(s.modelTv || '')
        setModelMusic(s.modelMusic || '')
        setModelAudiobooks(s.modelAudiobooks || '')
      } else {
        console.warn('[Parser Settings] No data returned from query:', result)
        addToast({
          title: 'Warning',
          description: 'Could not load LLM parser settings',
          color: 'warning',
        })
      }
      if (result.error) {
        console.error('[Parser Settings] Query error:', result.error)
        addToast({
          title: 'Error',
          description: sanitizeError(result.error),
          color: 'danger',
        })
      }
    } catch (e) {
      console.error('[Parser Settings] Exception:', e)
      addToast({
        title: 'Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchSettings()
  }, [fetchSettings])

  const handleSave = async () => {
    setIsSaving(true)
    try {
      const result = await graphqlClient
        .mutation<{ updateLlmParserSettings: SettingsResult }>(UPDATE_LLM_PARSER_SETTINGS_MUTATION, {
          input: {
            enabled,
            ollamaUrl,
            ollamaModel,
            timeoutSeconds,
            temperature,
            maxTokens,
            promptTemplate,
            confidenceThreshold,
            modelMovies: modelMovies || null,
            modelTv: modelTv || null,
            modelMusic: modelMusic || null,
            modelAudiobooks: modelAudiobooks || null,
          },
        })
        .toPromise()

      if (result.data?.updateLlmParserSettings.success) {
        setOriginalSettings({
          enabled,
          ollamaUrl,
          ollamaModel,
          timeoutSeconds,
          temperature,
          maxTokens,
          promptTemplate,
          confidenceThreshold,
          modelMovies: modelMovies || null,
          modelTv: modelTv || null,
          modelMusic: modelMusic || null,
          modelAudiobooks: modelAudiobooks || null,
          promptMovies: null,
          promptTv: null,
          promptMusic: null,
          promptAudiobooks: null,
        })
        addToast({
          title: 'Settings saved',
          description: 'LLM parser settings have been updated',
          color: 'success',
        })
      } else {
        addToast({
          title: 'Error',
          description: result.data?.updateLlmParserSettings.error || 'Failed to save settings',
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
      setIsSaving(false)
    }
  }

  const handleTestConnection = async () => {
    setIsTesting(true)
    setConnectionStatus('untested')
    setConnectionError(null)
    try {
      const result = await graphqlClient
        .mutation<{ testOllamaConnection: OllamaConnectionResult }>(TEST_OLLAMA_CONNECTION_MUTATION, {
          url: ollamaUrl,
        })
        .toPromise()

      if (result.data?.testOllamaConnection.success) {
        setConnectionStatus('success')
        setAvailableModels(result.data.testOllamaConnection.availableModels)
        addToast({
          title: 'Connection successful',
          description: `Found ${result.data.testOllamaConnection.availableModels.length} models`,
          color: 'success',
        })
      } else {
        setConnectionStatus('error')
        setConnectionError(result.data?.testOllamaConnection.error || 'Connection failed')
        addToast({
          title: 'Connection failed',
          description: result.data?.testOllamaConnection.error || 'Could not connect to Ollama',
          color: 'danger',
        })
      }
    } catch (e) {
      setConnectionStatus('error')
      setConnectionError(sanitizeError(e))
      addToast({
        title: 'Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    } finally {
      setIsTesting(false)
    }
  }

  const handleTestParser = async () => {
    if (!testFilename.trim()) return
    setIsParsing(true)
    setParseResult(null)
    try {
      const result = await graphqlClient
        .mutation<{ testFilenameParser: TestFilenameParserResult }>(TEST_FILENAME_PARSER_MUTATION, {
          filename: testFilename,
        })
        .toPromise()

      if (result.data?.testFilenameParser) {
        setParseResult(result.data.testFilenameParser)
      }
      if (result.error) {
        addToast({
          title: 'Error',
          description: sanitizeError(result.error),
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
      setIsParsing(false)
    }
  }

  if (isLoading) {
    return (
      <div className="flex justify-center items-center h-64">
        <Spinner size="lg" />
      </div>
    )
  }

  return (
    <div className="grow overflow-y-auto overflow-x-hidden pb-8" style={{ scrollbarGutter: 'stable' }}>
      <SettingsHeader
        title="Filename Parser"
        subtitle="Configure regex and optional LLM-based filename parsing"
        hasChanges={hasChanges}
        isSaving={isSaving}
        onSave={handleSave}
      />

      {/* LLM Settings */}
      <Card className="mb-6">
        <CardHeader className="flex gap-3">
          <IconBrain size={24} className="text-cyan-400" />
          <div className="flex flex-col">
            <p className="text-lg font-semibold">LLM Parser (Ollama)</p>
            <p className="text-small text-default-500">
              Use a local LLM as fallback for complex filenames
            </p>
          </div>
          <div className="ml-auto">
            <Switch isSelected={enabled} onValueChange={setEnabled} />
          </div>
        </CardHeader>
        <Divider />
        <CardBody>
          <Accordion 
            selectionMode="multiple" 
            defaultExpandedKeys={['connection']}
            variant="light"
            className="px-0"
          >
            {/* Connection Section */}
            <AccordionItem
              key="connection"
              aria-label="Connection"
              title={
                <div className="flex items-center gap-2">
                  <IconServer size={18} className="text-blue-400" />
                  <span className="font-medium">Connection</span>
                  {connectionStatus === 'success' && (
                    <Chip size="sm" color="success" variant="flat">Connected</Chip>
                  )}
                </div>
              }
              subtitle="Ollama server URL and model selection"
              isDisabled={!enabled}
            >
              <div className="space-y-4 pb-2">
                <div className="flex gap-2">
                  <Input
                    label="Ollama URL"
                    placeholder="http://localhost:11434"
                    value={ollamaUrl}
                    onChange={(e) => setOllamaUrl(e.target.value)}
                    startContent={<IconServer size={16} className="text-default-400" />}
                    isDisabled={!enabled}
                    className="flex-1"
                  />
                  <Button
                    color={connectionStatus === 'success' ? 'success' : connectionStatus === 'error' ? 'danger' : 'default'}
                    variant={connectionStatus === 'untested' ? 'flat' : 'solid'}
                    isLoading={isTesting}
                    onPress={handleTestConnection}
                    isDisabled={!enabled}
                    className="self-end"
                  >
                    {connectionStatus === 'success' ? <IconCheck size={16} /> : connectionStatus === 'error' ? <IconX size={16} /> : 'Test'}
                  </Button>
                </div>

                {connectionError && (
                  <Card className="bg-danger/10 border border-danger/20">
                    <CardBody className="py-2 px-3">
                      <p className="text-danger text-sm">{connectionError}</p>
                    </CardBody>
                  </Card>
                )}

                {availableModels.length > 0 ? (
                  <Select
                    label="Default Model"
                    selectedKeys={[ollamaModel]}
                    onSelectionChange={(keys) => {
                      const selected = Array.from(keys)[0] as string
                      if (selected) setOllamaModel(selected)
                    }}
                    isDisabled={!enabled}
                    description={`${availableModels.length} models available`}
                  >
                    {availableModels.map((model) => (
                      <SelectItem key={model}>{model}</SelectItem>
                    ))}
                  </Select>
                ) : (
                  <Input
                    label="Default Model"
                    placeholder="qwen2.5-coder:7b"
                    value={ollamaModel}
                    onChange={(e) => setOllamaModel(e.target.value)}
                    isDisabled={!enabled}
                    description="Test connection to see available models"
                  />
                )}
              </div>
            </AccordionItem>

            {/* Library-Type Models */}
            <AccordionItem
              key="library-models"
              aria-label="Library-Type Models"
              title={
                <div className="flex items-center gap-2">
                  <IconMovie size={18} className="text-purple-400" />
                  <span className="font-medium">Library-Type Models</span>
                </div>
              }
              subtitle="Use different models for different content types"
              isDisabled={!enabled}
            >
              <div className="space-y-4 pb-2">
                <p className="text-small text-default-500">
                  Override the default model for specific library types. Leave empty to use the default.
                </p>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  {availableModels.length > 0 ? (
                    <>
                      <Select
                        label="Movies"
                        selectedKeys={modelMovies ? [modelMovies] : ['__default__']}
                        onSelectionChange={(keys) => {
                          const selected = Array.from(keys)[0] as string
                          setModelMovies(selected === '__default__' ? '' : selected || '')
                        }}
                        isDisabled={!enabled}
                        startContent={<IconMovie size={16} className="text-purple-400" />}
                        items={[{ key: '__default__', label: 'Use default model', isDefault: true }, ...availableModels.map(m => ({ key: m, label: m, isDefault: false }))]}
                      >
                        {(item) => (
                          <SelectItem key={item.key} className={item.isDefault ? 'text-default-500' : ''}>
                            {item.label}
                          </SelectItem>
                        )}
                      </Select>
                      <Select
                        label="TV Shows"
                        selectedKeys={modelTv ? [modelTv] : ['__default__']}
                        onSelectionChange={(keys) => {
                          const selected = Array.from(keys)[0] as string
                          setModelTv(selected === '__default__' ? '' : selected || '')
                        }}
                        isDisabled={!enabled}
                        startContent={<IconDeviceTv size={16} className="text-blue-400" />}
                        items={[{ key: '__default__', label: 'Use default model', isDefault: true }, ...availableModels.map(m => ({ key: m, label: m, isDefault: false }))]}
                      >
                        {(item) => (
                          <SelectItem key={item.key} className={item.isDefault ? 'text-default-500' : ''}>
                            {item.label}
                          </SelectItem>
                        )}
                      </Select>
                      <Select
                        label="Audiobooks"
                        selectedKeys={modelAudiobooks ? [modelAudiobooks] : ['__default__']}
                        onSelectionChange={(keys) => {
                          const selected = Array.from(keys)[0] as string
                          setModelAudiobooks(selected === '__default__' ? '' : selected || '')
                        }}
                        isDisabled={!enabled}
                        startContent={<IconBook size={16} className="text-amber-400" />}
                        items={[{ key: '__default__', label: 'Use default model', isDefault: true }, ...availableModels.map(m => ({ key: m, label: m, isDefault: false }))]}
                      >
                        {(item) => (
                          <SelectItem key={item.key} className={item.isDefault ? 'text-default-500' : ''}>
                            {item.label}
                          </SelectItem>
                        )}
                      </Select>
                      <Select
                        label="Music"
                        selectedKeys={modelMusic ? [modelMusic] : ['__default__']}
                        onSelectionChange={(keys) => {
                          const selected = Array.from(keys)[0] as string
                          setModelMusic(selected === '__default__' ? '' : selected || '')
                        }}
                        isDisabled={!enabled}
                        startContent={<IconMusic size={16} className="text-success" />}
                        items={[{ key: '__default__', label: 'Use default model', isDefault: true }, ...availableModels.map(m => ({ key: m, label: m, isDefault: false }))]}
                      >
                        {(item) => (
                          <SelectItem key={item.key} className={item.isDefault ? 'text-default-500' : ''}>
                            {item.label}
                          </SelectItem>
                        )}
                      </Select>
                    </>
                  ) : (
                    <>
                      <Input
                        label="Movies"
                        placeholder="Use default"
                        value={modelMovies}
                        onChange={(e) => setModelMovies(e.target.value)}
                        isDisabled={!enabled}
                        startContent={<IconMovie size={16} className="text-purple-400" />}
                      />
                      <Input
                        label="TV Shows"
                        placeholder="Use default"
                        value={modelTv}
                        onChange={(e) => setModelTv(e.target.value)}
                        isDisabled={!enabled}
                        startContent={<IconDeviceTv size={16} className="text-blue-400" />}
                      />
                      <Input
                        label="Audiobooks"
                        placeholder="Use default"
                        value={modelAudiobooks}
                        onChange={(e) => setModelAudiobooks(e.target.value)}
                        isDisabled={!enabled}
                        startContent={<IconBook size={16} className="text-amber-400" />}
                      />
                      <Input
                        label="Music"
                        placeholder="Use default"
                        value={modelMusic}
                        onChange={(e) => setModelMusic(e.target.value)}
                        isDisabled={!enabled}
                        startContent={<IconMusic size={16} className="text-success" />}
                      />
                    </>
                  )}
                </div>
              </div>
            </AccordionItem>

            {/* Advanced Settings */}
            <AccordionItem
              key="advanced"
              aria-label="Advanced Settings"
              title={
                <div className="flex items-center gap-2">
                  <IconBrain size={18} className="text-default-500" />
                  <span className="font-medium">Advanced Settings</span>
                </div>
              }
              subtitle="Timeout, temperature, and prompt configuration"
              isDisabled={!enabled}
            >
              <div className="space-y-4 pb-2">
                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                  <Input
                    type="number"
                    label="Timeout (seconds)"
                    value={timeoutSeconds.toString()}
                    onChange={(e) => setTimeoutSeconds(parseInt(e.target.value) || 30)}
                    isDisabled={!enabled}
                    description="Max wait time for LLM response"
                  />
                  <Input
                    type="number"
                    label="Temperature"
                    value={temperature.toString()}
                    onChange={(e) => setTemperature(parseFloat(e.target.value) || 0.1)}
                    step={0.1}
                    min={0}
                    max={2}
                    isDisabled={!enabled}
                    description="Lower = more deterministic"
                  />
                  <Input
                    type="number"
                    label="Max Tokens"
                    value={maxTokens.toString()}
                    onChange={(e) => setMaxTokens(parseInt(e.target.value) || 256)}
                    isDisabled={!enabled}
                    description="Maximum output length"
                  />
                </div>

                <Input
                  type="number"
                  label="Confidence Threshold"
                  value={confidenceThreshold.toString()}
                  onChange={(e) => setConfidenceThreshold(parseFloat(e.target.value) || 0.7)}
                  step={0.1}
                  min={0}
                  max={1}
                  isDisabled={!enabled}
                  description="Use LLM when regex confidence is below this value (0.0 - 1.0)"
                  className="max-w-xs"
                />

                <Textarea
                  label="Default Prompt Template"
                  placeholder="Parse this media filename..."
                  value={promptTemplate}
                  onChange={(e) => setPromptTemplate(e.target.value)}
                  isDisabled={!enabled}
                  minRows={6}
                  description="Use {filename} as placeholder. Models with baked-in system prompts may ignore this."
                />
              </div>
            </AccordionItem>
          </Accordion>
        </CardBody>
      </Card>

      {/* Parser Test */}
      <Card>
        <CardHeader className="flex gap-3">
          <IconTestPipe size={24} className="text-blue-400" />
          <div className="flex flex-col">
            <p className="text-lg font-semibold">Test Parser</p>
            <p className="text-small text-default-500">
              Compare regex and LLM parsing results side by side
            </p>
          </div>
        </CardHeader>
        <Divider />
        <CardBody className="space-y-4">
          <div className="flex gap-2">
            <Input
              label="Test Filename"
              placeholder="Movie.Name.2024.1080p.BluRay.x264-GROUP"
              value={testFilename}
              onChange={(e) => setTestFilename(e.target.value)}
              className="flex-1"
            />
            <Button
              color="primary"
              isLoading={isParsing}
              onPress={handleTestParser}
              className="self-end"
              startContent={!isParsing && <IconRefresh size={16} />}
            >
              Parse
            </Button>
          </div>

          {parseResult && (
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
              <ParseResultCard
                title="Regex Parser"
                result={parseResult.regexResult}
                timeMs={parseResult.regexTimeMs}
                variant="regex"
              />
              <ParseResultCard
                title="LLM Parser"
                result={parseResult.llmResult}
                timeMs={parseResult.llmTimeMs}
                error={parseResult.llmError}
                variant="llm"
              />
            </div>
          )}
        </CardBody>
      </Card>
    </div>
  )
}

interface ParseResultCardProps {
  title: string
  result: FilenameParseResult | null
  timeMs: number | null
  error?: string | null
  variant?: 'regex' | 'llm'
}

function ParseResultCard({ title, result, timeMs, error, variant = 'regex' }: ParseResultCardProps) {
  const headerIcon = variant === 'llm' 
    ? <IconBrain size={18} className="text-cyan-400" />
    : <IconRefresh size={18} className="text-blue-400" />

  if (error) {
    return (
      <Card>
        <CardHeader className="flex justify-between items-center gap-2">
          <div className="flex items-center gap-2">
            {headerIcon}
            <span className="font-semibold">{title}</span>
          </div>
          <Chip color="danger" size="sm" variant="flat">Error</Chip>
        </CardHeader>
        <Divider />
        <CardBody>
          <p className="text-danger text-sm">{error}</p>
        </CardBody>
      </Card>
    )
  }

  if (!result) {
    return (
      <Card>
        <CardHeader className="flex justify-between items-center gap-2">
          <div className="flex items-center gap-2">
            {headerIcon}
            <span className="font-semibold">{title}</span>
          </div>
          <Chip color="default" size="sm" variant="flat">Disabled</Chip>
        </CardHeader>
        <Divider />
        <CardBody>
          <p className="text-default-500 text-sm">Enable LLM parsing to compare results</p>
        </CardBody>
      </Card>
    )
  }

  const confidenceColor = result.confidence >= 0.8 ? 'success' : result.confidence >= 0.5 ? 'warning' : 'danger'

  return (
    <Card>
      <CardHeader className="flex justify-between items-center gap-2">
        <div className="flex items-center gap-2">
          {headerIcon}
          <span className="font-semibold">{title}</span>
        </div>
        <div className="flex gap-2 items-center">
          {timeMs !== null && (
            <Chip size="sm" variant="flat" startContent={<IconClock size={12} />}>
              {timeMs.toFixed(1)}ms
            </Chip>
          )}
          <Chip color={confidenceColor} size="sm" variant="flat">
            {(result.confidence * 100).toFixed(0)}%
          </Chip>
        </div>
      </CardHeader>
      <Divider />
      <CardBody>
        <div className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
          <ParseField label="Type" value={result.mediaType} highlight />
          <ParseField label="Title" value={result.title} highlight />
          <ParseField label="Year" value={result.year?.toString()} />
          <ParseField label="Season" value={result.season?.toString()} />
          <ParseField label="Episode" value={result.episode?.toString()} />
          <ParseField label="Resolution" value={result.resolution} />
          <ParseField label="Source" value={result.source} />
          <ParseField label="Codec" value={result.videoCodec} />
          <ParseField label="Audio" value={result.audio} />
          <ParseField label="HDR" value={result.hdr} />
          <ParseField label="Group" value={result.releaseGroup} />
          <ParseField label="Edition" value={result.edition} />
        </div>
      </CardBody>
    </Card>
  )
}

interface ParseFieldProps {
  label: string
  value: string | null | undefined
  highlight?: boolean
}

function ParseField({ label, value, highlight }: ParseFieldProps) {
  return (
    <div className="flex justify-between">
      <span className="text-default-500">{label}</span>
      <span className={value ? (highlight ? 'text-foreground font-medium' : 'text-foreground') : 'text-default-400'}>
        {value || '—'}
      </span>
    </div>
  )
}
