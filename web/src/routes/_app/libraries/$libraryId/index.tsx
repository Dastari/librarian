import { createFileRoute, redirect } from "@tanstack/react-router";

import { libraryTabs } from "@/features/libraries/LibraryLayout";
import { LibraryDetailDocument } from "@/graphql/generated/graphql";

/** The bare library URL opens the first tab for its type. */
export const Route = createFileRoute("/_app/libraries/$libraryId/")({
  beforeLoad: async ({ context, params }) => {
    const { data } = await context.apollo.query({ query: LibraryDetailDocument, variables: { id: params.libraryId }, fetchPolicy: "cache-first" });
    const first = data?.library ? libraryTabs(data.library)[0] : undefined;
    if (first?.href) throw redirect({ href: first.href, replace: true });
  },
  component: () => null,
});
