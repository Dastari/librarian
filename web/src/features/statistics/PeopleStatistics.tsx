import { useNavigate } from "@tanstack/react-router";
import { defineActions } from "glass-ux/actions";
import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconChartLine } from "glass-ux/icons";
import { Segmented } from "glass-ux/segmented";
import { useMemo } from "react";

import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { SettingsLoadError } from "@/features/settings/SettingsLoadError";
import { UserIdentity } from "@/features/users/UserIdentity";
import { EntityUserListDocument, PlayStatisticsDocument, type UserFieldsFragment } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { parseTimestamp } from "@/lib/format";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { PERIODS, formatPlayed, periodRange, type PeriodKey, type PlayStatistics } from "./statistics";

type Usage = PlayStatistics["users"][number];
export interface PersonRow extends Usage {
  account: UserFieldsFragment | null;
}

const column = createDataTableColumns<PersonRow>();
function columnsFor(self: string | undefined) {
  return column.columns([
    column.accessor((row) => row.name, {
      id: "person", header: "Person", width: 26, priority: 1, cardRole: "title",
      cell: ({ row }) => (row.original.account
        ? <UserIdentity user={row.original.account} self={row.original.userId === self} />
        : <TextCell muted>{row.original.name}</TextCell>),
    }),
    column.accessor((row) => row.secondsWatched, { id: "watched", header: "Watched", width: 9, priority: 2, cell: ({ row }) => <TextCell>{formatPlayed(row.original.secondsWatched)}</TextCell> }),
    column.accessor((row) => row.secondsListened, { id: "listened", header: "Listened", width: 9, priority: 2, cell: ({ row }) => <TextCell>{formatPlayed(row.original.secondsListened)}</TextCell> }),
    column.accessor((row) => row.plays, { id: "plays", header: "Plays", width: 7, priority: 3, cell: ({ row }) => <TextCell>{row.original.plays.toLocaleString()}</TextCell> }),
    column.accessor((row) => row.completedPlays, { id: "finished", header: "Finished", width: 7, priority: 4, cell: ({ row }) => <TextCell muted>{row.original.completedPlays.toLocaleString()}</TextCell> }),
    column.accessor((row) => row.lastPlayedAt, { id: "last", header: "Last played", width: 9, priority: 2, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.lastPlayedAt)} fallback="—" /> }),
  ]);
}

/** Join the period's usage with the accounts, so each person shows as their identity. */
export function peopleRows(users: readonly Usage[], accounts: readonly UserFieldsFragment[]): PersonRow[] {
  const byId = new Map(accounts.map((account) => [account.id, account]));
  return users.map((usage) => ({ ...usage, account: byId.get(usage.userId) ?? null }));
}

const noun = { one: "person", other: "people" };

/**
 * Everyone's time for the period, one row per person who played something: watched, listened,
 * plays, finished and when they last played. A row opens that person's statistics.
 */
export function PeopleStatistics({ period }: { period: PeriodKey }) {
  const navigate = useNavigate();
  const { user } = useSession();
  const range = useMemo(() => periodRange(period), [period]);
  const stats = useQuery(PlayStatisticsDocument, { variables: { ...range, userId: null, allUsers: true, top: 1 } });
  const accounts = useQuery(EntityUserListDocument, { variables: { page: { limit: 100, offset: 0 } } });
  const data = (stats.data ?? stats.previousData)?.playStatistics;
  const rows = useMemo(() => peopleRows(data?.users ?? [], (accounts.data?.users.edges ?? []).map((edge) => edge.node)), [data, accounts.data]);
  const columns = useMemo(() => columnsFor(user?.id), [user?.id]);
  const open = (row: PersonRow) => void navigate({ to: "/settings/statistics/overview", search: { user: row.userId, period } });
  const actions = useMemo(() => defineActions<PersonRow>("statistics-person", [
    { id: "open", label: "Open statistics", icon: IconChartLine, group: "open", priority: 1, visible: (selected) => selected.length === 1, run: (selected) => open(selected[0]!) },
  ]), [period]); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <div className="flex shrink-0 flex-wrap items-center gap-3" data-nav-section>
        <Segmented
          aria-label="Period"
          size="sm"
          value={period}
          onValueChange={(value) => void navigate({ to: "/settings/statistics/people", search: { period: value as PeriodKey }, replace: true })}
          options={PERIODS.map((entry) => ({ value: entry.value, label: entry.label }))}
        />
      </div>
      <SettingsLoadError error={stats.error} retry={stats.refetch} hasRows={rows.length > 0} />
      <LoadedDataTable<PersonRow>
        complete={Boolean(data) && !stats.error}
        height="fill"
        queryKey={JSON.stringify([user?.id, period])}
        id={`librarian.${user?.id ?? "session"}.server-statistics.people`}
        label="Time per person"
        columns={columns}
        rows={rows}
        getRowId={(row) => row.userId}
        getRowLabel={(row) => row.name}
        loading={!data && stats.loading}
        actions={actions}
        onRowOpen={open}
        rowOpen="click"
        noun={noun}
        emptyMessage="Nobody played anything in this period."
      />
    </div>
  );
}
