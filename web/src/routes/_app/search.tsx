import { Outlet, createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_app/search")({
  staticData: { crumb: "Search", fixedHeight: true },
  component: () => <Outlet />,
});
