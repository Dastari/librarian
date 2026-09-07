import { useQuery } from "@apollo/client/react";
import { Link } from "@tanstack/react-router";

import { ErrorState, MetaLine, PosterCard, Section, SkeletonHero } from "@/components/ui";
import { CollectionDetailDocument, MovieCollectionDetailsDocument } from "@/graphql/generated/graphql";
import { collectionBackdrop, collectionPoster, moviePoster } from "@/lib/artwork";
import { formatRuntime } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { DetailShell } from "./DetailShell";

export function CollectionPage({ collectionId }: { collectionId: string }) {
  const { data, previousData, loading, error, refetch } = useQuery(CollectionDetailDocument, { variables: { id: collectionId } });
  const collection = (data ?? previousData)?.collection ?? null;
  const details = useQuery(MovieCollectionDetailsDocument, { variables: { collectionId: collection?.tmdbCollectionId ?? 0, libraryId: collection?.libraryId ?? "" }, skip: !collection });
  if (error && !collection) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!collection) return loading ? <SkeletonHero /> : null;
  const owned = collection.movies.edges.map((edge) => edge.node);
  const all = details.data?.movieCollectionDetails.movies ?? [];
  const missing = all.filter((movie) => !movie.libraryMovieId);

  return (
    <DetailShell
      hero={{
        backdrop: collectionBackdrop(collection),
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
              <PosterCard key={movie.tmdbId} title={movie.title} meta={[movie.year, movie.wanted ? "Wanted" : null].filter(Boolean).join(" · ")} image={movie.posterUrl} tint={LIBRARY_TYPES.movies.tintVar} className="opacity-75" onPress={() => undefined} />
            ))}
          </div>
        </Section>
      ) : null}
    </DetailShell>
  );
}
