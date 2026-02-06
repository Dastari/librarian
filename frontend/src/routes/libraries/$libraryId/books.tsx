import { createFileRoute } from '@tanstack/react-router'
import { useState, useCallback } from 'react'
import { useDisclosure } from '@heroui/modal'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'
import { addToast } from '@heroui/toast'
import { useLibraryContext } from '../$libraryId'
import { LibraryAudiobooksTab, AddAudiobookModal } from '../../../components/library'
import { useMutation, gql } from '../../../lib/graphql/client'

export const Route = createFileRoute('/libraries/$libraryId/books')({
  component: AudiobooksPage,
})

const DELETE_AUDIOBOOK = gql`
  mutation DeleteAudiobook($Id: String!) {
    DeleteAudiobook(Id: $Id) {
      Success
      Error
    }
  }
`

function AudiobooksPage() {
  const { library, loading } = useLibraryContext()
  const { isOpen, onOpen, onClose } = useDisclosure()
  const [refreshKey, setRefreshKey] = useState(0)

  const [deleteTarget, setDeleteTarget] = useState<{ id: string; title: string } | null>(null)

  const [deleteAudiobook, { loading: isDeleting }] = useMutation<{
    DeleteAudiobook: { Success: boolean; Error?: string }
  }>(DELETE_AUDIOBOOK)

  const handleAudiobookAdded = useCallback(() => {
    onClose()
    setRefreshKey((k) => k + 1)
  }, [onClose])

  const handleDeleteAudiobook = useCallback((audiobookId: string, title: string) => {
    setDeleteTarget({ id: audiobookId, title })
  }, [])

  const confirmDelete = useCallback(async () => {
    if (!deleteTarget) return

    try {
      const { data } = await deleteAudiobook({ variables: { Id: deleteTarget.id } })

      if (data?.DeleteAudiobook.Success) {
        addToast({
          title: 'Audiobook deleted',
          description: `${deleteTarget.title} has been removed from the library.`,
          color: 'success',
        })
        setRefreshKey((k) => k + 1)
      } else {
        addToast({
          title: 'Delete failed',
          description: data?.DeleteAudiobook.Error || 'Failed to delete audiobook',
          color: 'danger',
        })
      }
    } catch (err) {
      console.error('Failed to delete audiobook:', err)
      addToast({
        title: 'Delete failed',
        description: 'An error occurred while deleting the audiobook.',
        color: 'danger',
      })
    } finally {
      setDeleteTarget(null)
    }
  }, [deleteAudiobook, deleteTarget])

  return (
    <>
      <LibraryAudiobooksTab
        key={refreshKey}
        libraryId={library.Id}
        loading={loading}
        onAddAudiobook={onOpen}
        onDeleteAudiobook={handleDeleteAudiobook}
      />
      <AddAudiobookModal
        isOpen={isOpen}
        onClose={onClose}
        libraryId={library.Id}
        onAudiobookAdded={handleAudiobookAdded}
      />

      <Modal isOpen={!!deleteTarget} onClose={() => setDeleteTarget(null)}>
        <ModalContent>
          <ModalHeader>Delete Audiobook</ModalHeader>
          <ModalBody>
            <p>
              Are you sure you want to delete <strong>{deleteTarget?.title}</strong>?
            </p>
            <p className="text-sm text-default-500 mt-2">
              This will remove the audiobook from the library. Associated files will not be deleted.
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
