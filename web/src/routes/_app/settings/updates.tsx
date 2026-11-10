import { createFileRoute } from "@tanstack/react-router";

import { UpdatesSettings } from "@/features/settings/UpdatesSettings";

export const Route = createFileRoute("/_app/settings/updates")({
  staticData: { crumb: "Updates" },
  component: UpdatesSettings,
});
