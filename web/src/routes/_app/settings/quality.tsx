import { createFileRoute } from "@tanstack/react-router";

import { QualitySettings } from "@/features/settings/QualitySettings";

export const Route = createFileRoute("/_app/settings/quality")({
  staticData: { crumb: "Quality" },
  component: QualitySettings,
});
