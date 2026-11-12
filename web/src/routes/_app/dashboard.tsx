import { createFileRoute, redirect } from "@tanstack/react-router";

import { dashboardRedirect } from "@/features/statistics/dashboardRedirect";
import { session } from "@/lib/auth/session";

/**
 * The dashboard is gone: its personal views are tabs of your profile and its everyone views are
 * Settings › Statistics. Old addresses (bookmarks, the TV app's history) are sent there.
 */
export const Route = createFileRoute("/_app/dashboard")({
  beforeLoad: ({ location }) => {
    throw redirect({ href: dashboardRedirect(location.pathname, location.search as Record<string, unknown>, session.getSnapshot().user?.role === "admin"), replace: true });
  },
  component: () => null,
});
