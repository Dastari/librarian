import { createFileRoute } from "@tanstack/react-router";

import { ScansPanel } from "@/features/libraries/ScansPanel";

export const Route = createFileRoute("/_app/libraries/$libraryId/scans")({
  staticData: { crumb: "Scans", fixedHeight: true },
  component: () => <ScansPanel libraryId={Route.useParams().libraryId} />,
});
