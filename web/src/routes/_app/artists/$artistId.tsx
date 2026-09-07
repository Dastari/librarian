import { createFileRoute } from "@tanstack/react-router";

import { ArtistPage } from "@/features/media/ArtistPage";
import { ArtistDetailDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/artists/$artistId")({
  staticData: { hero: true },
  loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: ArtistDetailDocument, variables: { id: params.artistId }, fetchPolicy: "cache-first" });
    return { crumb: data?.artist?.name ?? "Artist" };
  },
  component: () => <ArtistPage artistId={Route.useParams().artistId} />,
});
