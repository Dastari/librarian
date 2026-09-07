import { createFileRoute } from "@tanstack/react-router";

import { SourcesSettings } from "@/features/settings/SourcesSettings";

export const Route = createFileRoute("/_app/settings/sources")({
  staticData: { crumb: "Sources" },
  component: SourcesSettings,
});
