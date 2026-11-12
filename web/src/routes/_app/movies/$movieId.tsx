import { useLiveRoute } from "@/lib/graphql/live/hooks";
import { libraryTrail } from "@/components/shell/Breadcrumbs";
import { createFileRoute } from "@tanstack/react-router";

import { MoviePage } from "@/features/media/MoviePage";
import { MovieDetailDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/movies/$movieId")({
    loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: MovieDetailDocument, variables: { id: params.movieId }, fetchPolicy: "cache-first" });
    return { crumb: data?.movie?.title ?? "Movie", trail: libraryTrail(data?.movie?.library, "movies") };
  },
  component: () => {
    const { movieId } = Route.useParams();
    useLiveRoute(MovieDetailDocument, { id: movieId });
    return <MoviePage movieId={movieId} />;
  },
});
