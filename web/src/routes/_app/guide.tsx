import { Outlet, createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_app/guide")({
  staticData: { crumb: "TV guide", fixedHeight: true },
  component: () => <Outlet />,
});
