import { createFileRoute } from "@tanstack/react-router";

import { UsersSettings } from "@/features/settings/UsersSettings";

export const Route = createFileRoute("/_app/settings/users")({
  staticData: { crumb: "Users" },
  component: UsersSettings,
});
