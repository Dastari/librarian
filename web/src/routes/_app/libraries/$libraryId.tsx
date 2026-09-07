import { createFileRoute, notFound } from "@tanstack/react-router";

import { LibraryLayout } from "@/features/libraries/LibraryLayout";
import { LibraryDetailDocument } from "@/graphql/generated/graphql";

export const Route = createFileRoute("/_app/libraries/$libraryId")({
  staticData: { fixedHeight: true },
  loader: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: LibraryDetailDocument, variables: { id: params.libraryId }, fetchPolicy: "cache-first" });
    if (!data?.library) throw notFound();
    return { crumb: data.library.name, library: data.library };
  },
  component: () => {
    const { library } = Route.useLoaderData();
    return <LibraryLayout library={library} />;
  },
});
