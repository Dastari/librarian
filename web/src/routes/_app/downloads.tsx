import { Outlet, createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_app/downloads")({
  staticData: { crumb: "Downloads", fixedHeight: true },
  component: () => <Outlet />,
});
