import { createFileRoute } from "@tanstack/react-router";

import { LibrariesPage } from "@/features/libraries/LibrariesPage";

export const Route = createFileRoute("/_app/libraries/")({
  component: LibrariesPage,
});
