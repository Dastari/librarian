import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useLibraryContext } from '../$libraryId'
import { LibraryAuthorsTab } from '../../../components/library'

export const Route = createFileRoute('/libraries/$libraryId/authors')({
  component: AuthorsPage,
})

function AuthorsPage() {
  const ctx = useLibraryContext()
  const navigate = useNavigate()

  if (!ctx?.library) {
    return null
  }

  const handleSelectAuthor = (_authorId: string) => {
    // Navigate to books tab filtered by author (future enhancement)
    // For now, just navigate to books
    navigate({ to: '/libraries/$libraryId/books', params: { libraryId: ctx.library.id } })
  }

  return (
    <LibraryAuthorsTab
      libraryId={ctx.library.id}
      onSelectAuthor={handleSelectAuthor}
    />
  )
}
