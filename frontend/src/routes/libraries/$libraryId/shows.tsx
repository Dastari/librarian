import { createFileRoute } from '@tanstack/react-router'
import { Spinner } from '@heroui/spinner'
import { LibraryShowsTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/shows')({
  component: ShowsPage,
})

function ShowsPage() {
  const ctx = useLibraryContext()

  if (!ctx) {
    return (
      <div className="flex justify-center items-center py-12">
        <Spinner size="lg" />
      </div>
    )
  }

  const { tvShows, handleDeleteShowClick, onOpenAddShow } = ctx

  return (
    <LibraryShowsTab
      shows={tvShows}
      onDeleteShow={handleDeleteShowClick}
      onAddShow={onOpenAddShow}
    />
  )
}
