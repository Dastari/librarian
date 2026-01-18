import { createFileRoute } from '@tanstack/react-router'
import { Card, CardBody } from '@heroui/card'
import { IconStack } from '@tabler/icons-react'
import { useLibraryContext } from '../$libraryId'

export const Route = createFileRoute('/libraries/$libraryId/collections')({
  component: CollectionsPage,
})

function CollectionsPage() {
  const ctx = useLibraryContext()

  return (
    <div className="flex flex-col w-full h-full overflow-hidden">
      {/* Toolbar */}
      <div className="flex items-center justify-between gap-4 mb-4 shrink-0">
        <h2 className="text-xl font-semibold">Collections</h2>
      </div>

      {/* Empty State */}
      <Card className="bg-content1/50 border-default-300 border-dashed border-2">
        <CardBody className="py-12 text-center">
          <IconStack size={48} className="mx-auto mb-4 text-purple-400" />
          <h3 className="text-lg font-semibold mb-2">No collections yet</h3>
          <p className="text-default-500 mb-4">
            Movie collections (like "The Matrix Collection" or "MCU") will appear here
            as you add movies that belong to collections.
          </p>
          <p className="text-xs text-default-400">
            Library: {ctx?.library?.name}
          </p>
        </CardBody>
      </Card>
    </div>
  )
}
