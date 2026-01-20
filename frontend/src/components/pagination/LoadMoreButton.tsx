import { Button } from '@heroui/button';
import { Spinner } from '@heroui/spinner';
import { IconChevronDown } from '@tabler/icons-react';
import type { PageInfo } from '../../lib/graphql/types';

interface LoadMoreButtonProps {
  pageInfo: PageInfo | null | undefined;
  onLoadMore: () => void;
  isLoading?: boolean;
  className?: string;
}

export function LoadMoreButton({
  pageInfo,
  onLoadMore,
  isLoading,
  className,
}: LoadMoreButtonProps) {
  if (!pageInfo?.hasNextPage) {
    return null;
  }

  return (
    <div className={`flex justify-center ${className || ''}`}>
      <Button
        variant="flat"
        isDisabled={isLoading}
        onPress={onLoadMore}
        startContent={
          isLoading ? <Spinner size="sm" /> : <IconChevronDown size={18} />
        }
      >
        {isLoading ? 'Loading...' : 'Load more'}
      </Button>
    </div>
  );
}
