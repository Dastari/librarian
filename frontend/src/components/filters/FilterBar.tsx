import { Button } from '@heroui/button';
import { Select, SelectItem } from '@heroui/select';
import { Switch } from '@heroui/switch';
import { IconFilter, IconX } from '@tabler/icons-react';
import { SearchInput } from './SearchInput';

interface FilterOption {
  key: string;
  label: string;
}

interface FilterBarProps {
  /** Search input value */
  searchValue: string;
  /** Called when search changes */
  onSearchChange: (value: string) => void;
  /** Search placeholder */
  searchPlaceholder?: string;

  /** Optional status filter options */
  statusOptions?: FilterOption[];
  /** Current status filter value */
  statusValue?: string;
  /** Called when status changes */
  onStatusChange?: (value: string) => void;

  /** Show monitored filter toggle */
  showMonitoredFilter?: boolean;
  /** Current monitored filter value (null = all, true = monitored only, false = unmonitored only) */
  monitoredValue?: boolean | null;
  /** Called when monitored filter changes */
  onMonitoredChange?: (value: boolean | null) => void;

  /** Show hasFile filter toggle */
  showHasFileFilter?: boolean;
  /** Current hasFile filter value */
  hasFileValue?: boolean | null;
  /** Called when hasFile filter changes */
  onHasFileChange?: (value: boolean | null) => void;

  /** Called when all filters are cleared */
  onClearAll?: () => void;

  /** Additional class names */
  className?: string;
}

/**
 * Reusable filter bar for library grids
 */
export function FilterBar({
  searchValue,
  onSearchChange,
  searchPlaceholder = 'Search...',
  statusOptions,
  statusValue,
  onStatusChange,
  showMonitoredFilter,
  monitoredValue,
  onMonitoredChange,
  showHasFileFilter,
  hasFileValue,
  onHasFileChange,
  onClearAll,
  className,
}: FilterBarProps) {
  const hasActiveFilters =
    searchValue ||
    statusValue ||
    monitoredValue !== null ||
    hasFileValue !== null;

  return (
    <div className={`flex flex-wrap items-center gap-3 ${className || ''}`}>
      {/* Search Input */}
      <SearchInput
        placeholder={searchPlaceholder}
        value={searchValue}
        onChange={onSearchChange}
        className="w-64"
      />

      {/* Status Filter */}
      {statusOptions && onStatusChange && (
        <Select
          placeholder="All statuses"
          selectedKeys={statusValue ? [statusValue] : []}
          onSelectionChange={(keys) => {
            if (keys === 'all') return;
            const selected = Array.from(keys)[0];
            onStatusChange(selected?.toString() || '');
          }}
          className="w-40"
          size="sm"
          startContent={<IconFilter size={16} className="text-default-400" />}
        >
          {statusOptions.map((option) => (
            <SelectItem key={option.key}>{option.label}</SelectItem>
          ))}
        </Select>
      )}

      {/* Monitored Toggle */}
      {showMonitoredFilter && onMonitoredChange && (
        <div className="flex items-center gap-2">
          <Switch
            size="sm"
            isSelected={monitoredValue === true}
            onValueChange={(checked: boolean) => {
              if (checked) {
                onMonitoredChange(true);
              } else if (monitoredValue === true) {
                onMonitoredChange(null);
              }
            }}
          >
            <span className="text-sm text-default-600">Monitored</span>
          </Switch>
        </div>
      )}

      {/* Has File Toggle */}
      {showHasFileFilter && onHasFileChange && (
        <div className="flex items-center gap-2">
          <Switch
            size="sm"
            isSelected={hasFileValue === true}
            onValueChange={(checked: boolean) => {
              if (checked) {
                onHasFileChange(true);
              } else if (hasFileValue === true) {
                onHasFileChange(null);
              }
            }}
          >
            <span className="text-sm text-default-600">Has file</span>
          </Switch>
        </div>
      )}

      {/* Clear All Button */}
      {hasActiveFilters && onClearAll && (
        <Button
          size="sm"
          variant="flat"
          color="default"
          startContent={<IconX size={14} />}
          onPress={onClearAll}
        >
          Clear filters
        </Button>
      )}
    </div>
  );
}
