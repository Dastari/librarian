import { createFileRoute } from "@tanstack/react-router";

import { LibrariesSettings } from "@/features/settings/LibrariesSettings";

export const Route = createFileRoute("/_app/settings/libraries")({
  staticData: { crumb: "Libraries" },
  component: LibrariesSettings,
});
