import { createFileRoute } from "@tanstack/react-router";

import { LogsSettings } from "@/features/settings/LogsSettings";

export const Route = createFileRoute("/_app/settings/logs")({
  staticData: { crumb: "Logs" },
  component: LogsSettings,
});
