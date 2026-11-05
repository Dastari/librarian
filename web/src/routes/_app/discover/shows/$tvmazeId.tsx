import { createFileRoute, redirect } from "@tanstack/react-router";

import { DiscoverShowPage } from "@/features/discover/DiscoverPages";
import { OwnedShowByTvmazeDocument, ProviderShowDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/discover/shows/$tvmazeId")({
  loader: async ({ context, params }) => {
    const tvmazeId = Number(params.tvmazeId);
    // A title already in a library has a real page.
    const owned = await context.apollo.query({ query: OwnedShowByTvmazeDocument, variables: { tvmazeId }, fetchPolicy: "network-only" });
    const showId = owned.data?.shows.edges[0]?.node.id;
    if (showId) throw redirect({ to: "/shows/$showId", params: { showId }, replace: true });
    const { data } = await context.apollo.query({ query: ProviderShowDocument, variables: { tvmazeId }, fetchPolicy: "cache-first" });
    return { crumb: data?.providerShow.name ?? "Show", show: data!.providerShow };
  },
  component: () => <DiscoverShowPage show={Route.useLoaderData().show} />,
});
