import { createFileRoute } from "@tanstack/react-router";

import { MoviesBrowser } from "@/features/libraries/browser/MoviesBrowser";

export const Route = createFileRoute("/_app/libraries/$libraryId/movies")({
  staticData: { crumb: "Movies" },
  component: () => <MoviesBrowser libraryId={Route.useParams().libraryId} />,
});
