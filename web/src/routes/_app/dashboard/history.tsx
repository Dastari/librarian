import { createFileRoute } from "@tanstack/react-router";

import { HistoryPage } from "@/features/dashboard/HistoryPage";

export const Route = createFileRoute("/_app/dashboard/history")({
  staticData: { crumb: "History" },
  component: HistoryPage,
});
