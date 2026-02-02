import { createFileRoute } from '@tanstack/react-router'
import { LibrarySettingsTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/settings')({
  component: SettingsPage,
})

function SettingsPage() {
  const { library, handleUpdateLibrary, actionLoading } = useLibraryContext()

  return (
      <LibrarySettingsTab
        library={library}
        onSave={handleUpdateLibrary}
        isLoading={actionLoading}
      />
  )
}
