import { createFileRoute, redirect } from "@tanstack/react-router";

import { FilesBrowser } from "@/features/libraries/browser/FilesBrowser";
import { libraryViews } from "@/features/libraries/LibraryLayout";
import { LibraryDetailDocument } from "@/graphql/generated/graphql";

/** A library with nothing else to browse shows its files here. Any other's are in its settings. */
export const Route = createFileRoute("/_app/libraries/$libraryId/files")({
  staticData: { crumb: "Files", fixedHeight: true },
  beforeLoad: async ({ context, params, location }) => {
    const { data } = await context.apollo.query({ query: LibraryDetailDocument, variables: { id: params.libraryId }, fetchPolicy: "cache-first" });
    const first = data?.library ? libraryViews(data.library)[0] : undefined;
    if (first?.href && first.key !== "files") {
      const search = new URLSearchParams(location.searchStr);
      if (!search.has("settings")) search.set("settings", "files");
      // Links to a file filter by its path; in the dialog that filter has its own name.
      const path = search.get("q");
      if (path) { search.delete("q"); search.set("fileq", path); }
      throw redirect({ href: `${first.href}?${search.toString()}`, replace: true });
    }
  },
  component: () => <FilesBrowser libraryId={Route.useParams().libraryId} />,
});
