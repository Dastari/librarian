import { createFileRoute } from "@tanstack/react-router";

import { DownloadsSettings } from "@/features/settings/DownloadsSettings";

export const Route = createFileRoute("/_app/settings/downloads")({
  staticData: { crumb: "Downloads" },
  component: DownloadsSettings,
});
