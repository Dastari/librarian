import { createFileRoute } from "@tanstack/react-router";

import { ArtistsBrowser } from "@/features/libraries/browser/ArtistsBrowser";

export const Route = createFileRoute("/_app/libraries/$libraryId/artists")({
  staticData: { crumb: "Artists" },
  component: () => <ArtistsBrowser libraryId={Route.useParams().libraryId} />,
});
