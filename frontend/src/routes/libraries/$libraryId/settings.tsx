import { createFileRoute } from '@tanstack/react-router'
import { LibrarySettingsTab } from '../../../components/library'
import { useLibraryContext } from '../$libraryId'
import { ShimmerLoader } from '../../../components/shared/ShimmerLoader'
import { libraryTemplate } from '../../../lib/template-data'

export const Route = createFileRoute('/libraries/$libraryId/settings')({
  component: SettingsPage,
})

function SettingsPage() {
  const { library, loading, handleUpdateLibrary, actionLoading } = useLibraryContext()

  return (
    <ShimmerLoader loading={loading} delay={500} templateProps={{ library: libraryTemplate }}>
      <LibrarySettingsTab
        library={library}
        onSave={handleUpdateLibrary}
        isLoading={actionLoading}
      />
    </ShimmerLoader>
  )
}
