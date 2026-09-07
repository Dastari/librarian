import { createFileRoute } from "@tanstack/react-router";

import { AboutSettings } from "@/features/settings/AboutSettings";

export const Route = createFileRoute("/_app/settings/about")({
  staticData: { crumb: "About" },
  component: AboutSettings,
});
