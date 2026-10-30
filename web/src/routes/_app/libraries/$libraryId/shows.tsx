import { createFileRoute } from "@tanstack/react-router";

import { ShowsBrowser } from "@/features/libraries/browser/ShowsBrowser";

export const Route = createFileRoute("/_app/libraries/$libraryId/shows")({
  staticData: { crumb: "Browse", fixedHeight: true },
  component: () => <ShowsBrowser libraryId={Route.useParams().libraryId} />,
});
