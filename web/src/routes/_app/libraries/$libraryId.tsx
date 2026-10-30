import { useLiveRoute } from "@/lib/graphql/live/hooks";
import { createFileRoute, notFound } from "@tanstack/react-router";

import { LibraryLayout } from "@/features/libraries/LibraryLayout";
import { LibraryDetailDocument } from "@/graphql/generated/graphql";

// Each tab sets its own height model: browsers are bounded panes, Recommended scrolls.
export const Route = createFileRoute("/_app/libraries/$libraryId")({
  loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: LibraryDetailDocument, variables: { id: params.libraryId }, fetchPolicy: "cache-first" });
    if (!data?.library) throw notFound();
    return { crumb: data.library.name, library: data.library };
  },
  component: () => {
    const { libraryId } = Route.useParams();
    useLiveRoute(LibraryDetailDocument, { id: libraryId });
    const { library } = Route.useLoaderData();
    return <LibraryLayout library={library} />;
  },
});
