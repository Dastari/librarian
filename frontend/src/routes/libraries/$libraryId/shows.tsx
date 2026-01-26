import { createFileRoute } from '@tanstack/react-router'
import { LibraryShowsTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/shows')({
  component: ShowsPage,
})

function ShowsPage() {
  const { library, loading, handleDeleteShowClick, onOpenAddShow } = useLibraryContext()

  return (
    <LibraryShowsTab
      libraryId={library.Id}
      loading={loading}
      onDeleteShow={handleDeleteShowClick}
      onAddShow={onOpenAddShow}
    />
  )
}
