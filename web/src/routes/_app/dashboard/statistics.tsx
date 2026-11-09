import { createFileRoute } from "@tanstack/react-router";

import { StatisticsPage } from "@/features/dashboard/StatisticsPage";
import { isPeriod, type PeriodKey } from "@/features/dashboard/statistics";

interface StatisticsSearch {
  period?: PeriodKey;
  user?: string;
}

export const Route = createFileRoute("/_app/dashboard/statistics")({
  staticData: { crumb: "Statistics" },
  validateSearch: (search: Record<string, unknown>): StatisticsSearch => ({
    period: isPeriod(search.period) ? search.period : undefined,
    user: typeof search.user === "string" && search.user ? search.user : undefined,
  }),
  component: () => {
    const { period, user } = Route.useSearch();
    return <StatisticsPage period={period ?? "30d"} person={user} />;
  },
});
