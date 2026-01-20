import { useState, useMemo } from 'react'
import { Button, ButtonGroup } from '@heroui/button'

// ============================================================================
// Constants
// ============================================================================

const ALPHABET = '#ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('')

// ============================================================================
// Types
// ============================================================================

export interface AlphabetFilterProps {
  /** Currently selected letter (null for "All") */
  selectedLetter: string | null
  /** Set of letters that have items */
  availableLetters: Set<string>
  /** Callback when letter selection changes */
  onLetterChange: (letter: string | null) => void
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Get the first letter of a name for alphabet filtering.
 * Skips common articles (the, a, an) at the start.
 */
export function getFirstLetter(name: string): string {
  const nameLower = name.toLowerCase()
  let sortName = name
  for (const article of ['the ', 'a ', 'an ']) {
    if (nameLower.startsWith(article)) {
      sortName = name.slice(article.length)
      break
    }
  }
  const firstChar = sortName.charAt(0).toUpperCase()
  return /[A-Z]/.test(firstChar) ? firstChar : '#'
}

/**
 * Hook to manage alphabet filter state for a list of items.
 * Returns the selected letter, available letters, filtered items, and change handler.
 */
export function useAlphabetFilter<T>(
  items: T[],
  getItemName: (item: T) => string
) {
  const [selectedLetter, setSelectedLetter] = useState<string | null>(null)

  // Get letters that have items
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    items.forEach((item) => {
      letters.add(getFirstLetter(getItemName(item)))
    })
    return letters
  }, [items, getItemName])

  // Filter items by selected letter
  const filteredItems = useMemo(() => {
    if (!selectedLetter) return items
    return items.filter((item) => getFirstLetter(getItemName(item)) === selectedLetter)
  }, [items, selectedLetter, getItemName])

  // Handle letter click - toggle filter
  const handleLetterChange = (letter: string | null) => {
    setSelectedLetter((prev) => (prev === letter ? null : letter))
  }

  return {
    selectedLetter,
    availableLetters,
    filteredItems,
    onLetterChange: handleLetterChange,
  }
}

// ============================================================================
// Component
// ============================================================================

/**
 * Alphabet filter bar component for A-Z filtering.
 * Designed to be used with DataTable's headerContent prop.
 */
export function AlphabetFilter({
  selectedLetter,
  availableLetters,
  onLetterChange,
}: AlphabetFilterProps) {
  return (
    <div className="flex items-center p-2 bg-content2 rounded-lg overflow-x-auto shrink-0 mb-4">
      <ButtonGroup size="sm" variant="flat">
        <Button
          variant={selectedLetter === null ? 'solid' : 'flat'}
          color={selectedLetter === null ? 'primary' : 'default'}
          onPress={() => onLetterChange(null)}
          className="min-w-8 px-2"
        >
          All
        </Button>
        {ALPHABET.map((letter) => {
          const hasItems = availableLetters.has(letter)
          const isSelected = selectedLetter === letter
          return (
            <Button
              key={letter}
              variant={isSelected ? 'solid' : 'flat'}
              color={isSelected ? 'primary' : 'default'}
              onPress={() => hasItems && onLetterChange(letter)}
              isDisabled={!hasItems}
              className="w-4 min-w-4 lg:w-6 lg:min-w-6 p-0 text-xs font-medium xl:min-w-7 xl:w-7"
            >
              {letter}
            </Button>
          )
        })}
      </ButtonGroup>
    </div>
  )
}
