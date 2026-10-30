import { createFileRoute } from "@tanstack/react-router";

import { TracksBrowser } from "@/features/libraries/browser/TracksBrowser";

export const Route = createFileRoute("/_app/libraries/$libraryId/tracks")({
  staticData: { crumb: "Tracks", fixedHeight: true },
  component: () => <TracksBrowser libraryId={Route.useParams().libraryId} />,
});
