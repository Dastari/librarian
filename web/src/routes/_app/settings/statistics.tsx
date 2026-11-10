import { createFileRoute, redirect } from "@tanstack/react-router";

import { ServerStatisticsLayout } from "@/features/settings/ServerStatisticsLayout";
import { session } from "@/lib/auth/session";

/** Everyone's playback, for administrators (design.md Q150). Others see their own in their profile. */
export const Route = createFileRoute("/_app/settings/statistics")({
  staticData: { crumb: "Statistics" },
  beforeLoad: () => {
    if (session.getSnapshot().user?.role !== "admin") throw redirect({ href: "/?profile=statistics", replace: true });
  },
  component: ServerStatisticsLayout,
});
