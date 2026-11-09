import { createFileRoute } from "@tanstack/react-router";

import { NetworkServersSettings } from "@/features/servers/NetworkServersSettings";

export const Route = createFileRoute("/_app/settings/network-shares")({
  staticData: { crumb: "Network shares" },
  component: NetworkServersSettings,
});
