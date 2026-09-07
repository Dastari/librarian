import { createFileRoute } from "@tanstack/react-router";

import { AlbumsBrowser } from "@/features/libraries/browser/AlbumsBrowser";

export const Route = createFileRoute("/_app/libraries/$libraryId/albums")({
  staticData: { crumb: "Albums" },
  component: () => <AlbumsBrowser libraryId={Route.useParams().libraryId} />,
});
