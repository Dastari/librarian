import { createFileRoute } from "@tanstack/react-router";

import { CastingSettings } from "@/features/settings/CastingSettings";

export const Route = createFileRoute("/_app/settings/casting")({
  staticData: { crumb: "Casting" },
  component: CastingSettings,
});
