import { createFileRoute } from "@tanstack/react-router";

import { DashboardLayout } from "@/features/dashboard/DashboardLayout";

export const Route = createFileRoute("/_app/dashboard")({
  staticData: { crumb: "Dashboard", fixedHeight: true },
  component: DashboardLayout,
});
