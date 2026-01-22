import { createFileRoute } from '@tanstack/react-router'
import { useState, useCallback } from 'react'
import { useDisclosure } from '@heroui/modal'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'
import { addToast } from '@heroui/toast'
import { useLibraryContext } from '../$libraryId'
import { LibraryAlbumsTab, AddAlbumModal } from '../../../components/library'
import { graphqlClient, DELETE_ALBUM_MUTATION } from '../../../lib/graphql'

export const Route = createFileRoute('/libraries/$libraryId/albums')({
  component: AlbumsPage,
})

function AlbumsPage() {
  const { library, loading } = useLibraryContext()
  const { isOpen, onOpen, onClose } = useDisclosure()
  const [refreshKey, setRefreshKey] = useState(0)

  // Delete confirmation state
  const [deleteTarget, setDeleteTarget] = useState<{ id: string; name: string } | null>(null)
  const [isDeleting, setIsDeleting] = useState(false)

  const handleAlbumAdded = useCallback(() => {
    onClose()
    setRefreshKey((k) => k + 1)
  }, [onClose])

  const handleDeleteAlbum = useCallback((albumId: string, albumName: string) => {
    setDeleteTarget({ id: albumId, name: albumName })
  }, [])

  const confirmDelete = useCallback(async () => {
    if (!deleteTarget) return

    setIsDeleting(true)
    try {
      const result = await graphqlClient
        .mutation<{ deleteAlbum: { success: boolean; error?: string } }>(
          DELETE_ALBUM_MUTATION,
          { id: deleteTarget.id }
        )
        .toPromise()

      if (result.data?.deleteAlbum.success) {
        addToast({
          title: 'Album deleted',
          description: `${deleteTarget.name} has been removed from the library.`,
          color: 'success',
        })
        setRefreshKey((k) => k + 1)
      } else {
        addToast({
          title: 'Delete failed',
          description: result.data?.deleteAlbum.error || 'Failed to delete album',
          color: 'danger',
        })
      }
    } catch (err) {
      console.error('Failed to delete album:', err)
      addToast({
        title: 'Delete failed',
        description: 'An error occurred while deleting the album.',
        color: 'danger',
      })
    } finally {
      setIsDeleting(false)
      setDeleteTarget(null)
    }
  }, [deleteTarget])

  return (
    <>
      <LibraryAlbumsTab
        key={refreshKey}
        libraryId={library.id}
        loading={loading}
        onAddAlbum={onOpen}
        onDeleteAlbum={handleDeleteAlbum}
      />
      <AddAlbumModal
        isOpen={isOpen}
        onClose={onClose}
        libraryId={library.id}
        onAlbumAdded={handleAlbumAdded}
      />

      {/* Delete confirmation modal */}
      <Modal isOpen={!!deleteTarget} onClose={() => setDeleteTarget(null)}>
        <ModalContent>
          <ModalHeader>Delete Album</ModalHeader>
          <ModalBody>
            <p>
              Are you sure you want to delete <strong>{deleteTarget?.name}</strong>?
            </p>
            <p className="text-sm text-default-500 mt-2">
              This will remove the album from the library. Associated files will not be deleted.
            </p>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={() => setDeleteTarget(null)}>
              Cancel
            </Button>
            <Button
              color="danger"
              onPress={confirmDelete}
              isLoading={isDeleting}
            >
              Delete
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </>
  )
}
