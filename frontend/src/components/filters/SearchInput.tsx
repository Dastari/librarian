import { Input } from '@heroui/input';
import { IconSearch, IconX } from '@tabler/icons-react';
import { useState, useCallback, useEffect } from 'react';

interface SearchInputProps {
  /** Placeholder text */
  placeholder?: string;
  /** Current search value */
  value: string;
  /** Called when search value changes (debounced) */
  onChange: (value: string) => void;
  /** Debounce delay in ms (default: 300) */
  debounceMs?: number;
  /** Additional class names */
  className?: string;
}

/**
 * Debounced search input for filtering lists
 */
export function SearchInput({
  placeholder = 'Search...',
  value,
  onChange,
  debounceMs = 300,
  className,
}: SearchInputProps) {
  const [localValue, setLocalValue] = useState(value);

  // Sync local value when external value changes
  useEffect(() => {
    setLocalValue(value);
  }, [value]);

  // Debounce the onChange callback
  useEffect(() => {
    const timer = setTimeout(() => {
      if (localValue !== value) {
        onChange(localValue);
      }
    }, debounceMs);

    return () => clearTimeout(timer);
  }, [localValue, value, onChange, debounceMs]);

  const handleClear = useCallback(() => {
    setLocalValue('');
    onChange('');
  }, [onChange]);

  return (
    <Input
      placeholder={placeholder}
      value={localValue}
      onValueChange={setLocalValue}
      startContent={<IconSearch size={18} className="text-default-400" />}
      endContent={
        localValue ? (
          <button
            onClick={handleClear}
            className="p-1 rounded-full hover:bg-default-200 transition-colors"
          >
            <IconX size={14} className="text-default-400" />
          </button>
        ) : null
      }
      className={className}
      classNames={{
        inputWrapper: 'bg-default-100',
      }}
    />
  );
}
