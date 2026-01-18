import { createFileRoute } from '@tanstack/react-router'
import { Spinner } from '@heroui/spinner'
import { LibrarySettingsTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/settings')({
  component: SettingsPage,
})

function SettingsPage() {
  const ctx = useLibraryContext()

  if (!ctx) {
    return (
      <div className="flex justify-center items-center py-12">
        <Spinner size="lg" />
      </div>
    )
  }

  const { library, handleUpdateLibrary, actionLoading } = ctx

  return (
    <LibrarySettingsTab
      library={library}
      onSave={handleUpdateLibrary}
      isLoading={actionLoading}
    />
  )
}
