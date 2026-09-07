import { createFileRoute } from "@tanstack/react-router";

import { AudiobooksBrowser } from "@/features/libraries/browser/AudiobooksBrowser";

export const Route = createFileRoute("/_app/libraries/$libraryId/audiobooks")({
  staticData: { crumb: "Audiobooks" },
  component: () => <AudiobooksBrowser libraryId={Route.useParams().libraryId} />,
});
