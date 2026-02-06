import { createFileRoute, redirect } from '@tanstack/react-router'
import { apolloClient, gql } from '../../../lib/graphql/client'

const LIBRARY_TYPE_QUERY = gql`
  query LibraryTypeForRedirect($Id: String!) {
    Library(Id: $Id) {
      Id
      LibraryType
    }
  }
`

export const Route = createFileRoute('/libraries/$libraryId/')({
  loader: async ({ params }) => {
    const result = await apolloClient.query<{
      Library: { Id: string; LibraryType: string } | null
    }>({
      query: LIBRARY_TYPE_QUERY,
      variables: { Id: params.libraryId },
      fetchPolicy: 'network-only',
    })

    const library = result.data?.Library ?? null
    if (!library) {
      throw redirect({
        to: '/libraries/$libraryId/shows',
        params: { libraryId: params.libraryId },
      })
    }

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
