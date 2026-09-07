import { createFileRoute } from "@tanstack/react-router";

import { CollectionPage } from "@/features/media/CollectionPage";
import { CollectionDetailDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/collections/$collectionId")({
  staticData: { hero: true },
  loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: CollectionDetailDocument, variables: { id: params.collectionId }, fetchPolicy: "cache-first" });
    return { crumb: data?.collection?.name ?? "Collection" };
  },
  component: () => <CollectionPage collectionId={Route.useParams().collectionId} />,
});
