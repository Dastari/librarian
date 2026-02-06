import { createFileRoute } from '@tanstack/react-router'
import { useState, useRef, useCallback } from "react";
import { useDisclosure } from '@heroui/modal'
import { useSubscription, gql } from "../../../lib/graphql/client";
import {
  LibraryMoviesTab,
  AddMovieModal,
  DeleteMovieModal,
} from "../../../components/library";
import { useLibraryContext } from '../$libraryId'
import { ChangeAction } from "../../../lib/graphql/generated/graphql";
import { MOVIE_CHANGED_SUBSCRIPTION } from "../../../lib/graphql";

export const Route = createFileRoute('/libraries/$libraryId/movies')({
  component: MoviesPage,
})

const MOVIE_CHANGED = gql`
  ${MOVIE_CHANGED_SUBSCRIPTION}
`;

function MoviesPage() {
  const { library, loading } = useLibraryContext();
  const {
    isOpen: isDeleteOpen,
    onOpen: onDeleteOpen,
    onClose: onDeleteClose,
  } = useDisclosure();
  const {
    isOpen: isAddOpen,
    onOpen: onAddOpen,
    onClose: onAddClose,
  } = useDisclosure();
  const [movieToDelete, setMovieToDelete] = useState<{
    id: string;
    title: string;
  } | null>(null);

  // Use ref to store refresh function to avoid re-render loop
  const refreshMoviesRef = useRef<(() => void) | null>(null);

  // Subscribe to movie changes for this library
  useSubscription<{
    MovieChanged: {
      Id: string;
      Action: ChangeAction;
      Movie?: { LibraryId: string } | null;
    };
  }>(
    MOVIE_CHANGED,
    {
      variables: {
        Filter: { Actions: ["Created", "Updated", "Deleted"] },
      },
      onData: ({ data }) => {
        const event = data.data?.MovieChanged;
        if (!event) return;

        // SubscriptionFilterInput only supports Id/Actions, so filter by library in client.
        if (event.Movie?.LibraryId && event.Movie.LibraryId !== library.Id) return;

        // Refresh the movies list on any change
        if (refreshMoviesRef.current) {
          refreshMoviesRef.current();
        }
      },
    }
  );

  const handleDeleteMovieClick = (movieId: string, movieTitle: string) => {
    setMovieToDelete({ id: movieId, title: movieTitle });
    onDeleteOpen();
  };

  const handleMoviesRefresh = useCallback((refreshFn: () => void) => {
    refreshMoviesRef.current = refreshFn;
  }, []);

  const handleMoviesUpdated = useCallback(() => {
    refreshMoviesRef.current?.();
  }, []);

  return (
    <>
      <LibraryMoviesTab
        libraryId={library.Id}
        loading={loading}
        onDeleteMovie={handleDeleteMovieClick}
        onAddMovie={onAddOpen}
        onRefreshReady={handleMoviesRefresh}
      />

      {/* Add Movie Modal */}
      <AddMovieModal
        isOpen={isAddOpen}
        onClose={onAddClose}
        libraryId={library.Id}
        onAdded={handleMoviesUpdated}
      />

      {/* Delete Movie Modal */}
      <DeleteMovieModal
        isOpen={isDeleteOpen}
        onClose={onDeleteClose}
        movie={movieToDelete}
        onDeleted={handleMoviesUpdated}
      />
    </>
  );
}
