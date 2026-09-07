import { createFileRoute } from "@tanstack/react-router";

import { ShowPage } from "@/features/media/ShowPage";
import { ShowDetailDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/shows/$showId")({
  staticData: { hero: true },
  loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: ShowDetailDocument, variables: { id: params.showId }, fetchPolicy: "cache-first" });
    return { crumb: data?.show?.name ?? "Show" };
  },
  component: () => <ShowPage showId={Route.useParams().showId} />,
});
