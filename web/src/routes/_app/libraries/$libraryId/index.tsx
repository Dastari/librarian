import { createFileRoute, redirect } from "@tanstack/react-router";

import { libraryViews } from "@/features/libraries/LibraryLayout";
import { LibraryDetailDocument } from "@/graphql/generated/graphql";

/** The bare library URL opens the first view for its type, keeping any settings section asked for. */
export const Route = createFileRoute("/_app/libraries/$libraryId/")({
  beforeLoad: async ({ context, params, location }) => {
    const { data } = await context.apollo.query({ query: LibraryDetailDocument, variables: { id: params.libraryId }, fetchPolicy: "cache-first" });
    const first = data?.library ? libraryViews(data.library)[0] : undefined;
    if (first?.href) throw redirect({ href: `${first.href}${location.searchStr}`, replace: true });
  },
  component: () => null,
});
