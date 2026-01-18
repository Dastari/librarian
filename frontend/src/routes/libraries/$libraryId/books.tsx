import { createFileRoute } from '@tanstack/react-router'
import { Card, CardBody } from '@heroui/card'
import { IconHeadphones, IconPlus } from '@tabler/icons-react'
import { Button } from '@heroui/button'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/books')({
  component: AudiobooksPage,
})

function AudiobooksPage() {
  const ctx = useLibraryContext()

  return (
    <div className="flex flex-col w-full h-full overflow-hidden">
      {/* Toolbar */}
      <div className="flex items-center justify-between gap-4 mb-4 shrink-0">
        <h2 className="text-xl font-semibold">Audiobooks</h2>
        <Button
          color="primary"
          startContent={<IconPlus size={16} />}
        >
          Add Audiobook
        </Button>
      </div>

      {/* Empty State */}
      <Card className="bg-content1/50 border-default-300 border-dashed border-2">
        <CardBody className="py-12 text-center">
          <IconHeadphones size={48} className="mx-auto mb-4 text-orange-400" />
          <h3 className="text-lg font-semibold mb-2">No audiobooks yet</h3>
          <p className="text-default-500 mb-4">
            Add audiobooks to this library to start listening.
          </p>
          <p className="text-xs text-default-400 mb-4">
            Library: {ctx?.library?.name}
          </p>
          <Button color="primary">
            Add Audiobook
          </Button>
        </CardBody>
      </Card>
    </div>
  )
}
