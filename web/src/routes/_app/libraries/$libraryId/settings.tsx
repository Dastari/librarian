import { createFileRoute } from "@tanstack/react-router";
import { getRouteApi } from "@tanstack/react-router";

import { LibrarySettingsForm } from "@/features/libraries/LibrarySettingsForm";

const parent = getRouteApi("/_app/libraries/$libraryId");

export const Route = createFileRoute("/_app/libraries/$libraryId/settings")({
  staticData: { crumb: "Settings" },
  component: () => {
    const { library } = parent.useLoaderData();
    return <LibrarySettingsForm library={library} />;
  },
});
