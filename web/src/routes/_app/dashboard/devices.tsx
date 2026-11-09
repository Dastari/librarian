import { createFileRoute } from "@tanstack/react-router";

import { DevicesPage } from "@/features/dashboard/DevicesPage";

export const Route = createFileRoute("/_app/dashboard/devices")({
  staticData: { crumb: "Devices" },
  component: DevicesPage,
});
