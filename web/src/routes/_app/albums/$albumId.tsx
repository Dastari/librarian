import { useLiveRoute } from "@/lib/graphql/live/hooks";
import { libraryTrail } from "@/components/shell/Breadcrumbs";
import { createFileRoute } from "@tanstack/react-router";

import { AlbumPage } from "@/features/media/AlbumPage";
import { AlbumDetailDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/albums/$albumId")({
    loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: AlbumDetailDocument, variables: { id: params.albumId }, fetchPolicy: "cache-first" });
    return { crumb: data?.album?.name ?? "Album", trail: libraryTrail(data?.album?.library, "albums") };
  },
  component: () => {
    const { albumId } = Route.useParams();
    useLiveRoute(AlbumDetailDocument, { id: albumId });
    return <AlbumPage albumId={albumId} />;
  },
});
