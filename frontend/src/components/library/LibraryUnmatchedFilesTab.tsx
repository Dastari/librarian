import { Card, CardBody } from '@heroui/card'
import { Button } from '@heroui/button'

interface LibraryUnmatchedFilesTabProps {
  libraryId: string
  libraryPath: string
}

export function LibraryUnmatchedFilesTab({ libraryId: _libraryId, libraryPath }: LibraryUnmatchedFilesTabProps) {
  return (
    <div className="space-y-4 w-full">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">Unmatched Files</h2>
          <p className="text-sm text-default-500">
            Files found in the library that couldn't be matched to a show
          </p>
        </div>
        <Button color="primary" variant="flat" size="sm">
          Scan for Unmatched
        </Button>
      </div>

      <Card className="bg-content1/50 border-default-300 border-dashed border-2">
        <CardBody className="py-12 text-center">
          <span className="text-5xl mb-4 block">✅</span>
          <h3 className="text-lg font-semibold mb-2">No unmatched files</h3>
          <p className="text-default-500 mb-4">
            All files in this library have been matched to shows.
          </p>
          <p className="text-xs text-default-400">
            Library path: <code className="bg-content2 px-2 py-1 rounded">{libraryPath}</code>
          </p>
        </CardBody>
      </Card>

      {/* Future: List of unmatched files with actions to manually match */}
    </div>
  )
}
