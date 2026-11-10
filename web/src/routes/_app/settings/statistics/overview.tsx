import { createFileRoute, useNavigate } from "@tanstack/react-router";

import { StatisticsPage, type StatisticsSearch } from "@/features/statistics/StatisticsPage";
import { isPeriod } from "@/features/statistics/statistics";

export const Route = createFileRoute("/_app/settings/statistics/overview")({
  staticData: { crumb: "Overview" },
  validateSearch: (search: Record<string, unknown>): StatisticsSearch => ({
    period: isPeriod(search.period) ? search.period : undefined,
    user: typeof search.user === "string" && search.user ? search.user : undefined,
    origin: typeof search.origin === "string" && search.origin ? search.origin : undefined,
  }),
  component: ServerStatistics,
});

function ServerStatistics() {
  const { period, user, origin } = Route.useSearch();
  const navigate = useNavigate();
  return (
    <StatisticsPage
      scope="server"
      period={period ?? "30d"}
      person={user}
      origin={origin}
      onSearchChange={(next) => void navigate({ to: "/settings/statistics/overview", search: (previous: StatisticsSearch) => ({ ...previous, ...next }), replace: true })}
    />
  );
}
