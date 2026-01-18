import { useState, useEffect } from 'react'
import { Select, SelectItem } from '@heroui/select'
import { Input } from '@heroui/input'
import { Switch } from '@heroui/switch'
import { Spinner } from '@heroui/spinner'
import { IconTemplate, IconPencil } from '@tabler/icons-react'
import { graphqlClient, NAMING_PATTERNS_QUERY, type NamingPattern } from '../../lib/graphql'
import { previewNamingPattern } from '../../lib/format'

interface NamingPatternSelectorProps {
  value: string | null
  onChange: (pattern: string | null) => void
  /** Label for the input group */
  label?: string
  /** Whether the parent is disabled */
  isDisabled?: boolean
}

/**
 * A selector for naming patterns.
 * Shows a dropdown of preset patterns with an option to enter a custom pattern.
 */
export function NamingPatternSelector({
  value,
  onChange,
  label = 'File Naming Pattern',
  isDisabled = false,
}: NamingPatternSelectorProps) {
  const [patterns, setPatterns] = useState<NamingPattern[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [useCustom, setUseCustom] = useState(false)
  const [customPattern, setCustomPattern] = useState('')

  // Fetch available patterns
  useEffect(() => {
    const fetchPatterns = async () => {
      try {
        const result = await graphqlClient
          .query<{ namingPatterns: NamingPattern[] }>(NAMING_PATTERNS_QUERY, {})
          .toPromise()
        
        if (result.data?.namingPatterns) {
          setPatterns(result.data.namingPatterns)
          
          // Check if current value matches a preset or is custom
          if (value) {
            const matchingPreset = result.data.namingPatterns.find(
              (p) => p.pattern === value
            )
            if (!matchingPreset) {
              setUseCustom(true)
              setCustomPattern(value)
            }
          }
        }
      } catch (error) {
        console.error('Failed to fetch naming patterns:', error)
      } finally {
        setIsLoading(false)
      }
    }

    fetchPatterns()
  }, [value])

  // Find the currently selected pattern ID based on the pattern string
  const selectedPatternId = patterns.find(p => p.pattern === value)?.id || ''

  const handlePatternSelect = (patternId: string) => {
    const pattern = patterns.find(p => p.id === patternId)
    if (pattern) {
      onChange(pattern.pattern)
    }
  }

  const handleCustomToggle = (checked: boolean) => {
    setUseCustom(checked)
    if (!checked) {
      // Switch back to preset - use default pattern
      const defaultPattern = patterns.find(p => p.isDefault) || patterns[0]
      if (defaultPattern) {
        onChange(defaultPattern.pattern)
      }
    } else {
      // Switch to custom - keep current value or use current preset value
      setCustomPattern(value || '')
    }
  }

  const handleCustomPatternChange = (newPattern: string) => {
    setCustomPattern(newPattern)
    onChange(newPattern)
  }

  if (isLoading) {
    return (
      <div className="flex items-center gap-2">
        <Spinner size="sm" />
        <span className="text-sm text-default-500">Loading patterns...</span>
      </div>
    )
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <label className="text-sm font-medium">{label}</label>
        <Switch
          size="sm"
          isSelected={useCustom}
          onValueChange={handleCustomToggle}
          isDisabled={isDisabled}
        >
          <span className="text-xs">Custom Pattern</span>
        </Switch>
      </div>

      {useCustom ? (
        <Input
          label="Custom Pattern"
          labelPlacement="inside"
          variant="flat"
          value={customPattern}
          onValueChange={handleCustomPatternChange}
          placeholder="{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}"
          startContent={<IconPencil size={16} className="text-default-400" />}
          isDisabled={isDisabled}
          size="sm"
          description="Available variables: {show}, {season}, {season:02}, {episode}, {episode:02}, {title}, {ext}, {year}"
          classNames={{
            label: 'text-sm font-medium text-primary!',
          }}
        />
      ) : (
        <Select
          selectedKeys={selectedPatternId ? [selectedPatternId] : []}
          onSelectionChange={(keys) => {
            const selected = Array.from(keys)[0] as string
            if (selected) handlePatternSelect(selected)
          }}
          placeholder="Select a naming pattern"
          startContent={<IconTemplate size={16} className="text-default-400" />}
          isDisabled={isDisabled}
          size="sm"
          classNames={{
            trigger: "min-h-10",
          }}
        >
          {patterns.map((pattern) => (
            <SelectItem
              key={pattern.id}
              textValue={pattern.name}
              description={pattern.description || pattern.pattern}
            >
              <div className="flex flex-col">
                <span className="font-medium">
                  {pattern.name}
                  {pattern.isDefault && (
                    <span className="ml-2 text-xs text-primary">(Default)</span>
                  )}
                </span>
                <span className="text-xs text-default-400 truncate max-w-xs">
                  {pattern.description || pattern.pattern}
                </span>
              </div>
            </SelectItem>
          ))}
        </Select>
      )}

      {/* Preview */}
      {value && (
        <div className="text-xs text-default-400 bg-default-100 p-2 rounded font-mono break-all">
          <span className="text-default-500">Preview: </span>
          {previewNamingPattern(value)}
        </div>
      )}
    </div>
  )
}
