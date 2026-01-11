import { createFileRoute, redirect } from '@tanstack/react-router'

// Redirect /libraries/$libraryId to /libraries/$libraryId/shows
export const Route = createFileRoute('/libraries/$libraryId/')({
  beforeLoad: ({ params }) => {
    throw redirect({
      to: '/libraries/$libraryId/shows',
      params: { libraryId: params.libraryId },
    })
  },
})
