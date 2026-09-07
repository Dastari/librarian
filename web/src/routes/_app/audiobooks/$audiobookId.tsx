import { createFileRoute } from "@tanstack/react-router";

import { AudiobookPage } from "@/features/media/AudiobookPage";
import { AudiobookDetailDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/audiobooks/$audiobookId")({
  staticData: { hero: true },
  loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: AudiobookDetailDocument, variables: { id: params.audiobookId }, fetchPolicy: "cache-first" });
    return { crumb: data?.audiobook?.title ?? "Audiobook" };
  },
  component: () => <AudiobookPage audiobookId={Route.useParams().audiobookId} />,
});
