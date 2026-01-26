import { useState, useEffect } from 'react'
import { Select, SelectItem } from '@heroui/select'
import { Input } from '@heroui/input'
import { Switch } from '@heroui/switch'
import { Spinner } from '@heroui/spinner'
import { IconTemplate, IconPencil } from '@tabler/icons-react'
import type { NamingPattern } from '../../lib/graphql/generated/graphql'
import { graphqlClient, NAMING_PATTERNS_QUERY } from '../../lib/graphql'
import { previewNamingPattern } from '../../lib/format'

interface NamingPatternSelectorProps {
  value: string | null
  onChange: (pattern: string | null) => void
  /** Label for the input group */
  label?: string
  /** Whether the parent is disabled */
  isDisabled?: boolean
  /** Library type to filter patterns by (tv, movies, music, audiobooks) */
  libraryType?: string
}

// Variable descriptions by library type
const VARIABLE_DESCRIPTIONS: Record<string, string> = {
  tv: '{show}, {season}, {season:02}, {episode}, {episode:02}, {title}, {ext}, {year}',
  movies: '{title}, {year}, {quality}, {ext}, {original}',
  music: '{artist}, {album}, {year}, {track}, {track:02}, {title}, {ext}, {original}',
  audiobooks: '{author}, {title}, {series}, {chapter}, {chapter:02}, {narrator}, {ext}, {original}',
}

// Placeholder patterns by library type
const PLACEHOLDER_PATTERNS: Record<string, string> = {
  tv: '{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}',
  movies: '{title} ({year})/{title} ({year}).{ext}',
  music: '{artist}/{album} ({year})/{track:02} - {title}.{ext}',
  audiobooks: '{author}/{title}/{chapter:02} - {chapter_title}.{ext}',
}

/**
 * A selector for naming patterns.
 * Shows a dropdown of preset patterns with an option to enter a custom pattern.
 * Filters patterns by library type when provided.
 */
export function NamingPatternSelector({
  value,
  onChange,
  label = 'File Naming Pattern',
  isDisabled = false,
  libraryType,
}: NamingPatternSelectorProps) {
  const [allPatterns, setAllPatterns] = useState<NamingPattern[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [useCustom, setUseCustom] = useState(false)
  const [customPattern, setCustomPattern] = useState('')

  // Filter patterns by library type
  const patterns = libraryType
    ? allPatterns.filter(p => p.LibraryType === libraryType)
    : allPatterns

  // Fetch available patterns
  useEffect(() => {
    const fetchPatterns = async () => {
      try {
        const result = await graphqlClient
          .query<{ NamingPatterns: { Edges: Array<{ Node: NamingPattern }> } }>(NAMING_PATTERNS_QUERY, {})
          .toPromise()
        
        if (result.data?.NamingPatterns?.Edges) {
          const nodes = result.data.NamingPatterns.Edges.map(e => e.Node)
          setAllPatterns(nodes)
          
          // Check if current value matches a preset or is custom
          if (value) {
            const filteredPatterns = libraryType
              ? nodes.filter(p => p.LibraryType === libraryType)
              : nodes
            const matchingPreset = filteredPatterns.find(
              (p) => p.Pattern === value
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
  }, [value, libraryType])

  // Find the currently selected pattern ID based on the pattern string
  const selectedPatternId = patterns.find(p => p.Pattern === value)?.Id || ''
  
  // Get appropriate description and placeholder for this library type
  const variableDescription = VARIABLE_DESCRIPTIONS[libraryType || 'tv'] || VARIABLE_DESCRIPTIONS.tv
  const placeholderPattern = PLACEHOLDER_PATTERNS[libraryType || 'tv'] || PLACEHOLDER_PATTERNS.tv

  const handlePatternSelect = (patternId: string) => {
    const pattern = patterns.find(p => p.Id === patternId)
    if (pattern) {
      onChange(pattern.Pattern)
    }
  }

  const handleCustomToggle = (checked: boolean) => {
    setUseCustom(checked)
    if (!checked) {
      // Switch back to preset - use default pattern
      const defaultPattern = patterns.find(p => p.IsDefault) || patterns[0]
      if (defaultPattern) {
        onChange(defaultPattern.Pattern)
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
          placeholder={placeholderPattern}
          startContent={<IconPencil size={16} className="text-default-400" />}
          isDisabled={isDisabled}
          size="sm"
          description={`Available variables: ${variableDescription}`}
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
              key={pattern.Id}
              textValue={pattern.Name}
              description={pattern.Description || pattern.Pattern}
            >
              <div className="flex flex-col">
                <span className="font-medium">
                  {pattern.Name}
                  {pattern.IsDefault && (
                    <span className="ml-2 text-xs text-primary">(Default)</span>
                  )}
                </span>
                <span className="text-xs text-default-400 truncate max-w-xs">
                  {pattern.Description || pattern.Pattern}
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
          {previewNamingPattern(value, libraryType)}
        </div>
      )}
    </div>
  )
}
