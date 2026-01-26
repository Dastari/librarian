import { createFileRoute, redirect } from '@tanstack/react-router'
import { graphqlClient, LIBRARY_QUERY } from '../../../lib/graphql'

// Redirect /libraries/$libraryId to the appropriate default tab based on library type
export const Route = createFileRoute('/libraries/$libraryId/')({
  loader: async ({ params }) => {
    // Fetch library to determine type
    const result = await graphqlClient
      .query<{
        Library: import('../../../lib/graphql/generated/graphql').Library | null
      }>(LIBRARY_QUERY, { Id: params.libraryId })
      .toPromise()

    const library = result.data?.Library ?? null
    if (!library) {
      // Library not found, redirect to shows as fallback (will show error)
      throw redirect({
        to: '/libraries/$libraryId/shows',
        params: { libraryId: params.libraryId },
      })
    }

    // Redirect based on library type
    switch (library.LibraryType) {
      case 'MOVIES':
        throw redirect({
          to: '/libraries/$libraryId/movies',
          params: { libraryId: params.libraryId },
        })
      case 'MUSIC':
        throw redirect({
          to: '/libraries/$libraryId/albums',
          params: { libraryId: params.libraryId },
        })
      case 'AUDIOBOOKS':
        throw redirect({
          to: '/libraries/$libraryId/books',
          params: { libraryId: params.libraryId },
        })
      case 'TV':
      default:
        throw redirect({
          to: '/libraries/$libraryId/shows',
          params: { libraryId: params.libraryId },
        })
    }
  },
})
