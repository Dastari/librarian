import { Outlet, createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_app/libraries")({
  staticData: { crumb: "Libraries" },
  component: () => <Outlet />,
});
