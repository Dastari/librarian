import { createFileRoute } from "@tanstack/react-router";

import { OrganizationSettings } from "@/features/settings/OrganizationSettings";

export const Route = createFileRoute("/_app/settings/organization")({
  staticData: { crumb: "Organization" },
  component: OrganizationSettings,
});
