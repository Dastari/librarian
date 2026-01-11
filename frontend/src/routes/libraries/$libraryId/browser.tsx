import { createFileRoute } from '@tanstack/react-router'
import { Spinner } from '@heroui/spinner'
import { LibraryFileBrowserTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/browser')({
  component: BrowserPage,
})

function BrowserPage() {
  const ctx = useLibraryContext()

  if (!ctx) {
    return (
      <div className="flex justify-center items-center py-12">
        <Spinner size="lg" />
      </div>
    )
  }

  return <LibraryFileBrowserTab libraryPath={ctx.library.path} />
}
