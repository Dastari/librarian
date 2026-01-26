import { createFileRoute } from '@tanstack/react-router'
import { useState } from 'react'
import { useDisclosure } from '@heroui/modal'
import { addToast } from '@heroui/toast'
import { LibraryMoviesTab, AddMovieModal } from '../../../components/library'
import { ConfirmModal } from '../../../components/ConfirmModal'
import { useLibraryContext } from '../$libraryId'
import { graphqlClient, DELETE_MOVIE_MUTATION } from '../../../lib/graphql'
import { sanitizeError } from '../../../lib/format'

export const Route = createFileRoute('/libraries/$libraryId/movies')({
  component: MoviesPage,
})

function MoviesPage() {
  const { library, loading, fetchData } = useLibraryContext()
  const { isOpen: isConfirmOpen, onOpen: onConfirmOpen, onClose: onConfirmClose } = useDisclosure()
  const { isOpen: isAddOpen, onOpen: onAddOpen, onClose: onAddClose } = useDisclosure()
  const [movieToDelete, setMovieToDelete] = useState<{ id: string; title: string } | null>(null)

  const handleDeleteMovieClick = (movieId: string, movieTitle: string) => {
    setMovieToDelete({ id: movieId, title: movieTitle })
    onConfirmOpen()
  }

  const handleDeleteMovie = async () => {
    if (!movieToDelete) return

    try {
      const { data, error } = await graphqlClient
        .mutation<{ deleteMovie: { success: boolean; error: string | null } }>(
          DELETE_MOVIE_MUTATION,
          { id: movieToDelete.id }
        )
        .toPromise()

      if (error || !data?.deleteMovie.success) {
        addToast({
          title: 'Error',
          description: sanitizeError(data?.deleteMovie.error || 'Failed to delete movie'),
          color: 'danger',
        })
        onConfirmClose()
        return
      }

      addToast({
        title: 'Deleted',
        description: `"${movieToDelete.title}" removed from library`,
        color: 'success',
      })

      await fetchData()
    } catch (err) {
      console.error('Failed to delete movie:', err)
    }
    onConfirmClose()
  }

  return (
    <>
      <LibraryMoviesTab
        libraryId={library.Id}
        loading={loading}
        onDeleteMovie={handleDeleteMovieClick}
        onAddMovie={onAddOpen}
      />

      {/* Add Movie Modal */}
      <AddMovieModal
        isOpen={isAddOpen}
        onClose={onAddClose}
        libraryId={library.Id}
        onAdded={fetchData}
      />

      {/* Confirm Delete Modal */}
      <ConfirmModal
        isOpen={isConfirmOpen}
        onClose={onConfirmClose}
        onConfirm={handleDeleteMovie}
        title="Delete Movie"
        message={`Are you sure you want to delete "${movieToDelete?.title}"?`}
        description="This will remove the movie from your library. Downloaded files will not be deleted."
        confirmLabel="Delete"
        confirmColor="danger"
      />
    </>
  )
}
