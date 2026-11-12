import { createFileRoute } from "@tanstack/react-router";

/** Matches any old `/dashboard/…` address so the parent's redirect sees it. */
export const Route = createFileRoute("/_app/dashboard/$")({
  component: () => null,
});
