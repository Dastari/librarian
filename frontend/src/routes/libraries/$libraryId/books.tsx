import { createFileRoute } from '@tanstack/react-router'
import { useState, useCallback } from 'react'
import { useDisclosure } from '@heroui/modal'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'
import { addToast } from '@heroui/toast'
import { useLibraryContext } from '../$libraryId'
import { LibraryAudiobooksTab, AddAudiobookModal } from '../../../components/library'
import { graphqlClient, DELETE_AUDIOBOOK_MUTATION } from '../../../lib/graphql'

export const Route = createFileRoute('/libraries/$libraryId/books')({
  component: AudiobooksPage,
})

function AudiobooksPage() {
  const ctx = useLibraryContext()
  const { isOpen, onOpen, onClose } = useDisclosure()
  const [refreshKey, setRefreshKey] = useState(0)

  // Delete confirmation state
  const [deleteTarget, setDeleteTarget] = useState<{ id: string; title: string } | null>(null)
  const [isDeleting, setIsDeleting] = useState(false)

  // All hooks must be called before any early returns (Rules of Hooks)
  const handleAudiobookAdded = useCallback(() => {
    onClose()
    setRefreshKey((k) => k + 1)
  }, [onClose])

  const handleDeleteAudiobook = useCallback((audiobookId: string, title: string) => {
    setDeleteTarget({ id: audiobookId, title })
  }, [])

  const confirmDelete = useCallback(async () => {
    if (!deleteTarget) return

    setIsDeleting(true)
    try {
      const result = await graphqlClient
        .mutation<{ deleteAudiobook: { success: boolean; error?: string } }>(
          DELETE_AUDIOBOOK_MUTATION,
          { id: deleteTarget.id }
        )
        .toPromise()

      if (result.data?.deleteAudiobook.success) {
        addToast({
          title: 'Audiobook deleted',
          description: `${deleteTarget.title} has been removed from the library.`,
          color: 'success',
        })
        setRefreshKey((k) => k + 1)
      } else {
        addToast({
          title: 'Delete failed',
          description: result.data?.deleteAudiobook.error || 'Failed to delete audiobook',
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
      setIsDeleting(false)
      setDeleteTarget(null)
    }
  }, [deleteTarget])

  // Early return after all hooks
  if (!ctx?.library) {
    return null
  }

  return (
    <>
      <LibraryAudiobooksTab
        key={refreshKey}
        libraryId={ctx.library.id}
        onAddAudiobook={onOpen}
        onDeleteAudiobook={handleDeleteAudiobook}
      />
      <AddAudiobookModal
        isOpen={isOpen}
        onClose={onClose}
        libraryId={ctx.library.id}
        onAudiobookAdded={handleAudiobookAdded}
      />

      {/* Delete confirmation modal */}
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
