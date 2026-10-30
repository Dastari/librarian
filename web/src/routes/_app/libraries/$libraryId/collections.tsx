import { createFileRoute } from "@tanstack/react-router";

import { CollectionsBrowser } from "@/features/libraries/browser/CollectionsBrowser";

export const Route = createFileRoute("/_app/libraries/$libraryId/collections")({
  staticData: { crumb: "Collections", fixedHeight: true },
  component: () => <CollectionsBrowser libraryId={Route.useParams().libraryId} />,
});
