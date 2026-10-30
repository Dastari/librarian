import { IconInfo } from "glass-ux/icons";
import { useCallback } from "react";
import { useApolloClient } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Link } from "@tanstack/react-router";

import { Button, EmptyState, ErrorState, MetaLine, PosterCard, Section, SkeletonHero } from "@/components/ui";
import { CollectionDetailDocument, MovieCollectionDetailsDocument } from "@/graphql/generated/graphql";
import { collectionBackdrop, collectionPoster, moviePoster } from "@/lib/artwork";
import { formatRuntime } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { DetailShell } from "./DetailShell";
import { useAudioPages } from "./useAudioPages";

export function CollectionPage({ collectionId }: { collectionId: string }) {
  const client = useApolloClient();
  const { data, previousData, loading, error, refetch } = useQuery(CollectionDetailDocument, { variables: { id: collectionId }, context: { liveManaged: true } });
  const candidate = (data ?? previousData)?.collection;
  const collection = candidate?.id === collectionId ? candidate : null;
  const loadPage = useCallback(async (offset: number) => {
    const result = await client.query({ query: CollectionDetailDocument, variables: { id: collectionId, page: { limit: 100, offset } }, fetchPolicy: "no-cache" });
    if (!result.data?.collection) throw new Error("Could not load the remaining movies");
    return result.data.collection.movies;
  }, [client, collectionId]);
  const pages = useAudioPages(collectionId, collection?.movies, loadPage, { document: CollectionDetailDocument, variables: { id: collectionId }, refreshHead: refetch });
  const details = useQuery(MovieCollectionDetailsDocument, { variables: { collectionId: collection?.tmdbCollectionId ?? 0, libraryId: collection?.libraryId ?? "" }, skip: !collection });
  if (error && !collection) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!collection) return loading ? <SkeletonHero /> : <EmptyState icon={IconInfo} title="Collection not found" description="This item may have been removed from the library." className="page-gutter my-8" />;
  const owned = pages.rows;
  const all = details.data?.movieCollectionDetails.movies ?? [];
  const missing = all.filter((movie) => !movie.libraryMovieId);

  return (
    <DetailShell
      backdrop={collectionBackdrop(collection)}
      hero={{
        poster: collectionPoster(collection),
        tint: LIBRARY_TYPES.movies.tintVar,
        eyebrow: collection.library ? (
          <Link to="/libraries/$libraryId/collections" params={{ libraryId: collection.library.id }} className="nav-focus rounded hover:underline">
            {collection.library.name}
          </Link>
        ) : null,
        title: collection.name,
        meta: <MetaLine items={[`${owned.length} of ${collection.movieCount} movies`, formatRuntime(owned.reduce((sum, movie) => sum + (movie.runtime ?? 0), 0), "minutes")]} />,
        description: collection.overview,
      }}
    >
      {pages.error ? <p role="status">Could not load all movies. <Button size="sm" onPress={pages.retry}>Retry</Button></p> : pages.loading ? <p role="status">Loading remaining movies…</p> : null}
      <Section title="In your library">
        <div className="poster-grid">
          {owned.map((movie) => (
            <PosterCard key={movie.id} title={movie.title} meta={movie.year ?? undefined} image={moviePoster(movie.id)} tint={LIBRARY_TYPES.movies.tintVar} to="/movies/$movieId" params={{ movieId: movie.id }} />
          ))}
        </div>
      </Section>
      {missing.length > 0 ? (
        <Section title="Missing from the collection">
          <div className="poster-grid">
            {missing.map((movie) => (
              <PosterCard key={movie.tmdbId} title={movie.title} meta={[movie.year, movie.wanted ? "Wanted" : null].filter(Boolean).join(" · ")} image={movie.posterUrl} tint={LIBRARY_TYPES.movies.tintVar} className="opacity-75" />
            ))}
          </div>
        </Section>
      ) : null}
    </DetailShell>
  );
}
