import { createFileRoute, useParams } from '@tanstack/react-router'
import { LibraryTracksTab } from '../../../components/library'

export const Route = createFileRoute('/libraries/$libraryId/tracks')({
  component: TracksPage,
})

function TracksPage() {
  const { libraryId } = useParams({ from: '/libraries/$libraryId/tracks' })

  return <LibraryTracksTab libraryId={libraryId} />
}
