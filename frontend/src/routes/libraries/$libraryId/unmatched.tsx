import { createFileRoute } from '@tanstack/react-router'
import { LibraryUnmatchedFilesTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/unmatched')({
  component: UnmatchedPage,
})

function UnmatchedPage() {
  const { library, loading } = useLibraryContext()

  return (
    <LibraryUnmatchedFilesTab
      libraryId={library.id}
      libraryPath={library.path}
      loading={loading}
    />
  )
}
