import { createFileRoute } from '@tanstack/react-router'
import { LibraryFileBrowserTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/browser')({
  component: BrowserPage,
})

function BrowserPage() {
  const { library, loading } = useLibraryContext()

  return <LibraryFileBrowserTab libraryPath={library.path} loading={loading} />
}
