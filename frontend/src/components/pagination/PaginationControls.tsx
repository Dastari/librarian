import { Button } from '@heroui/button';
import { IconChevronLeft, IconChevronRight } from '@tabler/icons-react';
import type { PageInfo } from '../../lib/graphql/types';

interface PaginationControlsProps {
  pageInfo: PageInfo | null | undefined;
  onNext: () => void;
  onPrev: () => void;
  isLoading?: boolean;
  className?: string;
}

export function PaginationControls({
  pageInfo,
  onNext,
  onPrev,
  isLoading,
  className,
}: PaginationControlsProps) {
  if (!pageInfo) {
    return null;
  }

  const { hasPreviousPage, hasNextPage, totalCount } = pageInfo;

  if (!hasPreviousPage && !hasNextPage) {
    return totalCount !== null && totalCount > 0 ? (
      <div className={`text-sm text-default-500 ${className || ''}`}>
        {totalCount} {totalCount === 1 ? 'item' : 'items'}
      </div>
    ) : null;
  }

  return (
    <div className={`flex items-center gap-3 ${className || ''}`}>
      <Button
        size="sm"
        variant="flat"
        isDisabled={!hasPreviousPage || isLoading}
        onPress={onPrev}
        startContent={<IconChevronLeft size={16} />}
      >
        Previous
      </Button>

      {totalCount !== null && (
        <span className="text-sm text-default-500">
          {totalCount} {totalCount === 1 ? 'item' : 'items'}
        </span>
      )}

      <Button
        size="sm"
        variant="flat"
        isDisabled={!hasNextPage || isLoading}
        onPress={onNext}
        endContent={<IconChevronRight size={16} />}
      >
        Next
      </Button>
    </div>
  );
}
