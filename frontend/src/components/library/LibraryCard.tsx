import { useNavigate } from '@tanstack/react-router'
import { Button } from '@heroui/button'
import { Card, CardBody, CardHeader } from '@heroui/card'
import { Chip } from '@heroui/chip'
import { LIBRARY_TYPES, type Library } from '../../lib/graphql'
import { formatBytes } from '../../lib/format'

export interface LibraryCardProps {
  library: Library
  onScan: () => void
  onEdit: () => void
  onDelete: () => void
}

export function LibraryCard({ library, onScan, onEdit, onDelete }: LibraryCardProps) {
  const navigate = useNavigate()
  const typeInfo =
    LIBRARY_TYPES.find((t) => t.value === library.libraryType) || LIBRARY_TYPES[4]

  return (
    <Card className="bg-content1">
      <CardHeader className="flex justify-between items-start">
        <div className="flex items-center gap-3">
          <typeInfo.Icon className="w-8 h-8" />
          <div>
            <h3 className="text-lg font-semibold">{library.name}</h3>
            <p className="text-default-500 text-sm">{typeInfo.label}</p>
          </div>
        </div>
        <div className="flex gap-2">
          {library.watchForChanges && (
            <Chip size="sm" color="secondary" variant="flat">
              Watching
            </Chip>
          )}
          <Chip
            size="sm"
            color={library.autoScan ? 'success' : 'default'}
            variant="flat"
          >
            {library.autoScan ? 'Auto-scan' : 'Manual'}
          </Chip>
        </div>
      </CardHeader>
      <CardBody className="pt-0">
        <div className="space-y-3">
          <div className="text-sm">
            <span className="text-default-500">Path:</span>
            <span className="ml-2 text-default-400 font-mono text-xs">
              {library.path}
            </span>
          </div>

          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-default-500">Files:</span>
              <span className="ml-2">{library.itemCount}</span>
            </div>
            <div>
              <span className="text-default-500">Size:</span>
              <span className="ml-2">{formatBytes(library.totalSizeBytes)}</span>
            </div>
            {library.libraryType === 'TV' && (
              <div>
                <span className="text-default-500">Shows:</span>
                <span className="ml-2">{library.showCount}</span>
              </div>
            )}
          </div>

          {library.lastScannedAt && (
            <div className="text-sm">
              <span className="text-default-500">Last scan:</span>
              <span className="ml-2 text-default-400">
                {new Date(library.lastScannedAt).toLocaleString()}
              </span>
            </div>
          )}

          <div className="flex gap-2 pt-2">
            <Button size="sm" color="primary" variant="flat" onPress={onScan}>
              Scan Now
            </Button>
            <Button size="sm" variant="flat" onPress={onEdit}>
              Settings
            </Button>
            {library.libraryType === 'TV' && (
              <Button
                size="sm"
                variant="flat"
                onPress={() => navigate({ to: '/libraries/$libraryId', params: { libraryId: library.id } })}
              >
                View Shows
              </Button>
            )}
            <Button
              size="sm"
              color="danger"
              variant="flat"
              onPress={onDelete}
            >
              Delete
            </Button>
          </div>
        </div>
      </CardBody>
    </Card>
  )
}
