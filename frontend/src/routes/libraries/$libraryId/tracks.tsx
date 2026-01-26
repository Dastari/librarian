import { createFileRoute } from '@tanstack/react-router'
import { LibraryTracksTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/tracks')({
  component: TracksPage,
})

function TracksPage() {
  const { library, loading } = useLibraryContext()

  return <LibraryTracksTab libraryId={library.Id} loading={loading} />
}
