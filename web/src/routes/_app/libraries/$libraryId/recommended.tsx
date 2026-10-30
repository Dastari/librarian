import { createFileRoute, getRouteApi } from "@tanstack/react-router";

import { LibraryRecommended } from "@/features/libraries/LibraryRecommended";

const library = getRouteApi("/_app/libraries/$libraryId");

export const Route = createFileRoute("/_app/libraries/$libraryId/recommended")({
  staticData: { crumb: "Recommended", hero: true },
  component: () => <LibraryRecommended library={library.useLoaderData().library} />,
});
