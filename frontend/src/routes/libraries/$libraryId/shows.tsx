import { createFileRoute, useParams } from '@tanstack/react-router'
import { LibraryShowsTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/shows')({
  component: ShowsPage,
})

function ShowsPage() {
  const { libraryId } = useParams({ from: '/libraries/$libraryId/shows' })
  const ctx = useLibraryContext()

  // Only need context for callbacks (delete, add)
  const handleDeleteShowClick = ctx?.handleDeleteShowClick ?? (() => {})
  const onOpenAddShow = ctx?.onOpenAddShow ?? (() => {})

  return (
    <LibraryShowsTab
      libraryId={libraryId}
      onDeleteShow={handleDeleteShowClick}
      onAddShow={onOpenAddShow}
    />
  )
}
