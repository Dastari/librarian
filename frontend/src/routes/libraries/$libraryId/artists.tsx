import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useLibraryContext } from '../$libraryId'
import { LibraryArtistsTab } from '../../../components/library'

export const Route = createFileRoute('/libraries/$libraryId/artists')({
  component: ArtistsPage,
})

function ArtistsPage() {
  const { library, loading } = useLibraryContext()
  const navigate = useNavigate()

  const handleSelectArtist = (_artistId: string) => {
    // Navigate to albums tab filtered by artist (future enhancement)
    // For now, just navigate to albums
    navigate({ to: '/libraries/$libraryId/albums', params: { libraryId: library.id } })
  }

  return (
    <LibraryArtistsTab
      libraryId={library.id}
      loading={loading}
      onSelectArtist={handleSelectArtist}
    />
  )
}
