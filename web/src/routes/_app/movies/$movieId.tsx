import { createFileRoute } from "@tanstack/react-router";

import { MoviePage } from "@/features/media/MoviePage";
import { MovieDetailDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/movies/$movieId")({
  staticData: { hero: true },
  loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: MovieDetailDocument, variables: { id: params.movieId }, fetchPolicy: "cache-first" });
    return { crumb: data?.movie?.title ?? "Movie" };
  },
  component: () => <MoviePage movieId={Route.useParams().movieId} />,
});
