import { createFileRoute, redirect } from "@tanstack/react-router";

import { DiscoverAudiobookPage } from "@/features/discover/DiscoverPages";
import { OwnedAudiobookByOpenlibraryDocument, ProviderAudiobookDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/discover/audiobooks/$providerId")({
  loader: async ({ context, params }) => {
    const { providerId } = params;
    // A title already in a library has a real page.
    const owned = await context.apollo.query({ query: OwnedAudiobookByOpenlibraryDocument, variables: { providerId }, fetchPolicy: "network-only" });
    const audiobookId = owned.data?.audiobooks.edges[0]?.node.id;
    if (audiobookId) throw redirect({ to: "/audiobooks/$audiobookId", params: { audiobookId }, replace: true });
    const { data } = await context.apollo.query({ query: ProviderAudiobookDocument, variables: { providerId }, fetchPolicy: "cache-first" });
    return { crumb: data?.providerAudiobook.title ?? "Audiobook", book: data!.providerAudiobook };
  },
  component: () => <DiscoverAudiobookPage book={Route.useLoaderData().book} />,
});
