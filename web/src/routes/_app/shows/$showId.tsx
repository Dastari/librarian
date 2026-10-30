import { useLiveRoute } from "@/lib/graphql/live/hooks";
import { createFileRoute } from "@tanstack/react-router";

import { ShowPage } from "@/features/media/ShowPage";
import { ShowDetailDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/shows/$showId")({
  staticData: { hero: true },
  loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: ShowDetailDocument, variables: { id: params.showId }, fetchPolicy: "cache-first" });
    return { crumb: data?.show?.name ?? "Show" };
  },
  component: () => {
    const { showId } = Route.useParams();
    useLiveRoute(ShowDetailDocument, { id: showId });
    return <ShowPage showId={showId} />;
  },
});
