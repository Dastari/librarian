import { createFileRoute } from '@tanstack/react-router'
import { useRef, useCallback, useEffect } from "react";
import { LibraryShowsTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'
 

export const Route = createFileRoute('/libraries/$libraryId/shows')({
  component: ShowsPage,
})

function ShowsPage() {
  const {
    library,
    loading,
    handleDeleteShowClick,
    onOpenAddShow,
    mediaRefreshToken,
  } = useLibraryContext()
  // Use ref to store refresh function to avoid re-render loop
  const refreshShowsRef = useRef<(() => void) | null>(null);

  const handleShowsRefresh = useCallback((refreshFn: () => void) => {
    refreshShowsRef.current = refreshFn;
  }, []);

  useEffect(() => {
    if (!library?.Id) return;
    refreshShowsRef.current?.();
  }, [library?.Id, mediaRefreshToken]);

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
