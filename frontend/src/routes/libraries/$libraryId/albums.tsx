import { createFileRoute } from '@tanstack/react-router'
import { useState, useCallback } from 'react'
import { useDisclosure } from '@heroui/modal'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'
import { addToast } from '@heroui/toast'
import { useLibraryContext } from '../$libraryId'
import { LibraryAlbumsTab, AddAlbumModal } from '../../../components/library'
import { useMutation, gql } from '../../../lib/graphql/client'

export const Route = createFileRoute('/libraries/$libraryId/albums')({
  component: AlbumsPage,
})

const DELETE_ALBUM = gql`
  mutation DeleteAlbum($Id: String!) {
    DeleteAlbum(Id: $Id) {
      Success
      Error
    }
  }
`

function AlbumsPage() {
  const { library, loading } = useLibraryContext()
  const { isOpen, onOpen, onClose } = useDisclosure()
  const [refreshKey, setRefreshKey] = useState(0)

  const [deleteTarget, setDeleteTarget] = useState<{ id: string; name: string } | null>(null)

  const [deleteAlbum, { loading: isDeleting }] = useMutation<{
    DeleteAlbum: { Success: boolean; Error?: string }
  }>(DELETE_ALBUM)

  const handleAlbumAdded = useCallback(() => {
    onClose()
    setRefreshKey((k) => k + 1)
  }, [onClose])

  const handleDeleteAlbum = useCallback((albumId: string, albumName: string) => {
    setDeleteTarget({ id: albumId, name: albumName })
  }, [])

  const confirmDelete = useCallback(async () => {
    if (!deleteTarget) return

    try {
      const { data } = await deleteAlbum({ variables: { Id: deleteTarget.id } })

      if (data?.DeleteAlbum.Success) {
        addToast({
          title: 'Album deleted',
          description: `${deleteTarget.name} has been removed from the library.`,
          color: 'success',
        })
        setRefreshKey((k) => k + 1)
      } else {
        addToast({
          title: 'Delete failed',
          description: data?.DeleteAlbum.Error || 'Failed to delete album',
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
      setDeleteTarget(null)
    }
  }, [deleteAlbum, deleteTarget])

  return (
    <>
      <LibraryAlbumsTab
        key={refreshKey}
        libraryId={library.Id}
        loading={loading}
        onAddAlbum={onOpen}
        onDeleteAlbum={handleDeleteAlbum}
      />
      <AddAlbumModal
        isOpen={isOpen}
        onClose={onClose}
        libraryId={library.Id}
        onAlbumAdded={handleAlbumAdded}
      />

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
