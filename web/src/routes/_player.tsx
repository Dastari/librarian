import { Outlet, createFileRoute, redirect } from "@tanstack/react-router";

import { session } from "@/lib/auth/session";

/** Immersive routes (video playback) share the auth guard but not the application shell. */
export const Route = createFileRoute("/_player")({
  beforeLoad: async ({ location }) => {
    await session.ready();
    if (session.getSnapshot().status !== "authenticated") {
      throw redirect({ to: "/login", search: { redirect: location.href } });
    }
  },
  component: () => <Outlet />,
});
