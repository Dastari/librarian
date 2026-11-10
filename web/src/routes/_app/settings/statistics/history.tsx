import { createFileRoute } from "@tanstack/react-router";

import { HistoryPage } from "@/features/statistics/HistoryPage";

export const Route = createFileRoute("/_app/settings/statistics/history")({
  staticData: { crumb: "History" },
  component: () => <HistoryPage scope="server" />,
});
