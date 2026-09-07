import { createFileRoute, redirect } from "@tanstack/react-router";

import { AppShell } from "@/components/shell/AppShell";
import { session } from "@/lib/auth/session";

/**
 * Layout route for everything that needs a signed-in user. `beforeLoad` waits for the session
 * to boot so a hard refresh never flashes the login page for a valid cookie session.
 */
export const Route = createFileRoute("/_app")({
  beforeLoad: async ({ location }) => {
    await session.ready();
    if (session.getSnapshot().status !== "authenticated") {
      throw redirect({ to: "/login", search: { redirect: location.href } });
    }
  },
  component: AppShell,
});
