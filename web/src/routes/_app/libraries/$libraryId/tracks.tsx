import { createFileRoute } from "@tanstack/react-router";

import { TracksBrowser } from "@/features/libraries/browser/TracksBrowser";

export const Route = createFileRoute("/_app/libraries/$libraryId/tracks")({
  staticData: { crumb: "Tracks" },
  component: () => <TracksBrowser libraryId={Route.useParams().libraryId} />,
});
