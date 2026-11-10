import { createFileRoute } from "@tanstack/react-router";

import { DevicesPage } from "@/features/statistics/DevicesPage";

export const Route = createFileRoute("/_app/settings/statistics/devices")({
  staticData: { crumb: "Devices" },
  component: () => <DevicesPage scope="server" />,
});
