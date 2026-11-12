import { createFileRoute } from "@tanstack/react-router";

import { PeopleStatistics } from "@/features/statistics/PeopleStatistics";
import { isPeriod, type PeriodKey } from "@/features/statistics/statistics";

export const Route = createFileRoute("/_app/settings/statistics/people")({
  staticData: { crumb: "People" },
  validateSearch: (search: Record<string, unknown>): { period?: PeriodKey } => ({ period: isPeriod(search.period) ? search.period : undefined }),
  component: () => <PeopleStatistics period={Route.useSearch().period ?? "30d"} />,
});
