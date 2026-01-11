import { createFileRoute } from '@tanstack/react-router'
import { Spinner } from '@heroui/spinner'
import { LibraryUnmatchedFilesTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/unmatched')({
  component: UnmatchedPage,
})

function UnmatchedPage() {
  const ctx = useLibraryContext()

  if (!ctx) {
    return (
      <div className="flex justify-center items-center py-12">
        <Spinner size="lg" />
      </div>
    )
  }

  return (
    <LibraryUnmatchedFilesTab
      libraryId={ctx.library.id}
      libraryPath={ctx.library.path}
    />
  )
}
