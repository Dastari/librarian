import { useLiveRoute } from "@/lib/graphql/live/hooks";
import { libraryTrail } from "@/components/shell/Breadcrumbs";
import { createFileRoute } from "@tanstack/react-router";

import { AudiobookPage } from "@/features/media/AudiobookPage";
import { AudiobookDetailDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/audiobooks/$audiobookId")({
    loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: AudiobookDetailDocument, variables: { id: params.audiobookId }, fetchPolicy: "cache-first" });
    return { crumb: data?.audiobook?.title ?? "Audiobook", trail: libraryTrail(data?.audiobook?.library, "audiobooks") };
  },
  component: () => {
    const { audiobookId } = Route.useParams();
    useLiveRoute(AudiobookDetailDocument, { id: audiobookId });
    return <AudiobookPage audiobookId={audiobookId} />;
  },
});
