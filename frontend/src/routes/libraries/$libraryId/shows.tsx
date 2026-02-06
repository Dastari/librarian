import { createFileRoute } from '@tanstack/react-router'
import { useRef, useCallback } from "react";
import { LibraryShowsTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'
import { useSubscription, gql } from "../../../lib/graphql/client";
import { ChangeAction } from "../../../lib/graphql/generated/graphql";
import { SHOW_CHANGED_SUBSCRIPTION } from "../../../lib/graphql";

export const Route = createFileRoute('/libraries/$libraryId/shows')({
  component: ShowsPage,
})

const SHOW_CHANGED = gql`
  ${SHOW_CHANGED_SUBSCRIPTION}
`;

function ShowsPage() {
  const { library, loading, handleDeleteShowClick, onOpenAddShow } = useLibraryContext()
  // Use ref to store refresh function to avoid re-render loop
  const refreshShowsRef = useRef<(() => void) | null>(null);

  // Subscribe to show changes for this library
  useSubscription<{
    ShowChanged: {
      Id: string;
      Action: ChangeAction;
      Show?: { LibraryId: string } | null;
    };
  }>(
    SHOW_CHANGED,
    {
      variables: {
        Filter: { Actions: ["Created", "Updated", "Deleted"] },
      },
      onData: ({ data }) => {
        const event = data.data?.ShowChanged;
        if (!event) return;

        // SubscriptionFilterInput only supports Id/Actions, so filter by library in client.
        if (event.Show?.LibraryId && event.Show.LibraryId !== library.Id) return;

        // Refresh the shows list on any change
        refreshShowsRef.current?.();
      },
    }
  );

  const handleShowsRefresh = useCallback((refreshFn: () => void) => {
    refreshShowsRef.current = refreshFn;
  }, []);

  return (
    <LibraryShowsTab
      libraryId={library.Id}
      loading={loading}
      onDeleteShow={handleDeleteShowClick}
      onAddShow={onOpenAddShow}
      onRefreshReady={handleShowsRefresh}
    />
  )
}
