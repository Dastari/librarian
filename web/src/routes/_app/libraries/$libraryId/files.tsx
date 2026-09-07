import { createFileRoute } from "@tanstack/react-router";

import { FilesBrowser } from "@/features/libraries/browser/FilesBrowser";

export const Route = createFileRoute("/_app/libraries/$libraryId/files")({
  staticData: { crumb: "Files" },
  component: () => <FilesBrowser libraryId={Route.useParams().libraryId} />,
});
