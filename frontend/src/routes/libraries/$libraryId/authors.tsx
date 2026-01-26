import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useLibraryContext } from '../$libraryId'
import { LibraryAuthorsTab } from '../../../components/library'

export const Route = createFileRoute('/libraries/$libraryId/authors')({
  component: AuthorsPage,
})

function AuthorsPage() {
  const { library, loading } = useLibraryContext()
  const navigate = useNavigate()

  const handleSelectAuthor = (_authorId: string) => {
    // Navigate to books tab filtered by author (future enhancement)
    // For now, just navigate to books
    navigate({ to: '/libraries/$libraryId/books', params: { libraryId: library.Id } })
  }

  return (
    <LibraryAuthorsTab
      libraryId={library.Id}
      loading={loading}
      onSelectAuthor={handleSelectAuthor}
    />
  )
}
