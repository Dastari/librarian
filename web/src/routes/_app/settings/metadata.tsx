import { createFileRoute } from "@tanstack/react-router";

import { MetadataSettings } from "@/features/settings/MetadataSettings";

export const Route = createFileRoute("/_app/settings/metadata")({
  staticData: { crumb: "Metadata" },
  component: MetadataSettings,
});
