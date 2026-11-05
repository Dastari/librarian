import { createFileRoute, redirect } from "@tanstack/react-router";

import { DiscoverMoviePage } from "@/features/discover/DiscoverPages";
import { OwnedMovieByTmdbDocument, ProviderMovieDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/discover/movies/$tmdbId")({
  loader: async ({ context, params }) => {
    const tmdbId = Number(params.tmdbId);
    // A title already in a library has a real page.
    const owned = await context.apollo.query({ query: OwnedMovieByTmdbDocument, variables: { tmdbId }, fetchPolicy: "network-only" });
    const movieId = owned.data?.movies.edges[0]?.node.id;
    if (movieId) throw redirect({ to: "/movies/$movieId", params: { movieId }, replace: true });
    const { data } = await context.apollo.query({ query: ProviderMovieDocument, variables: { tmdbId }, fetchPolicy: "cache-first" });
    return { crumb: data?.providerMovie.title ?? "Movie", movie: data!.providerMovie };
  },
  component: () => <DiscoverMoviePage movie={Route.useLoaderData().movie} />,
});
