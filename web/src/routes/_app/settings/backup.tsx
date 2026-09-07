import { createFileRoute } from "@tanstack/react-router";

import { BackupSettings } from "@/features/settings/BackupSettings";

export const Route = createFileRoute("/_app/settings/backup")({
  staticData: { crumb: "Backup" },
  component: BackupSettings,
});
