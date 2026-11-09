import { useNavigate } from "@tanstack/react-router";
import { BarChart, ColumnChart } from "glass-ux/charts";
import { IconShare, IconWarning } from "glass-ux/icons";
import { ListRow } from "glass-ux/list-row";
import { Metric } from "glass-ux/metric";
import { Segmented } from "glass-ux/segmented";
import { Select } from "glass-ux/select";
import { useMemo, useState, type ReactNode } from "react";

import { Button, EmptyState } from "@/components/ui";
import { PlayStatisticsDocument, StatisticsPeopleDocument } from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatRelative } from "@/lib/format";
import { errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { ShareStatisticsDialog } from "./ShareStatisticsDialog";
import { PERIODS, formatPlayed, hasPlays, periodRange, ranked, timeline, toHours, type PeriodKey, type PlayStatistics, type TopItem } from "./statistics";

/** Everyone, for an administrator. */
export const EVERYONE = "everyone";

function Group({ title, children, className }: { title: string; children: ReactNode; className?: string }) {
  return (
    <section aria-label={title} data-nav-section className={className}>
      <h2 className="mb-2 text-overline text-muted">{title}</h2>
      {children}
    </section>
  );
}

/** A ranked list ("Top shows"), as horizontal bars by hours, or by plays when no time is known. */
function TopList({ title, items, empty }: { title: string; items: readonly TopItem[]; empty: string }) {
  const chart = ranked(items);
  return (
    <Group title={title}>
      <div className="h-64 min-w-0">
        <BarChart
          orientation="horizontal"
          label={`${title}, by ${chart.byPlays ? "plays" : "hours"}`}
          categories={chart.categories}
          categoryLabel={title}
          series={chart.series}
          unit={chart.byPlays ? { one: "play", other: "plays" } : "h"}
          state={items.length === 0 ? "empty" : "ready"}
          emptyText={empty}
        />
      </div>
    </Group>
  );
}

function Totals({ stats }: { stats: PlayStatistics }) {
  const days = stats.days.length > 0 && stats.days.length <= 92 ? stats.days : stats.weeks;
  const range = stats.days.length > 0 && stats.days.length <= 92 ? "Per day" : "Per week";
  const { totals, streak } = stats;
  return (
    <div className="grid min-w-0 gap-x-6 gap-y-3 [grid-template-columns:repeat(auto-fit,minmax(min(100%,12rem),1fr))]" data-nav-group>
      <div className="h-20 min-w-0"><Metric label="Watched" value={toHours(totals.secondsWatched)} unit="h" format={(value) => formatPlayed(value * 3600)} data={days.map((bucket) => toHours(bucket.secondsWatched))} range={range} better="none" /></div>
      <div className="h-20 min-w-0"><Metric label="Listened" value={toHours(totals.secondsListened)} unit="h" format={(value) => formatPlayed(value * 3600)} data={days.map((bucket) => toHours(bucket.secondsListened))} range={range} better="none" /></div>
      <div className="h-20 min-w-0"><Metric label="Plays" value={totals.plays} data={days.map((bucket) => bucket.plays)} range={range} better="none" /></div>
      <div className="h-20 min-w-0"><Metric label="Streak" value={streak.current} unit={streak.current === 1 ? "day" : "days"} better="none" /></div>
      <p className="col-span-full text-body-sm text-muted">
        {[
          `${totals.items.toLocaleString()} different ${totals.items === 1 ? "title" : "titles"}`,
          `${totals.completedPlays.toLocaleString()} finished`,
          `active on ${totals.activeDays} ${totals.activeDays === 1 ? "day" : "days"}`,
          streak.longest > 1 ? `longest streak ${streak.longest} days` : "",
          totals.importedPlays > 0 ? `${totals.importedPlays} imported from watch progress, with no time` : "",
        ].filter(Boolean).join(" · ")}
      </p>
    </div>
  );
}

/** Episodes their number, films their year, music its artist, books their author. */
function recentLine(play: PlayStatistics["recent"][number]): string | null | undefined {
  return play.contentType === "episode" || play.contentType === "movie" ? play.subtitle : play.parentTitle;
}

function Recent({ plays }: { plays: PlayStatistics["recent"] }) {
  if (plays.length === 0) return null;
  return (
    <Group title="Recently played">
      <ul className="flex min-w-0 flex-col" data-nav-group>
        {plays.map((play) => (
          <li key={play.id}>
            <ListRow
              title={play.contentType === "episode" && play.parentTitle ? `${play.parentTitle}: ${play.title}` : play.title}
              description={[recentLine(play), play.imported ? "Imported" : formatPlayed(play.secondsPlayed), play.deviceName, formatRelative(play.startedAt)].filter(Boolean).join(" · ")}
            />
          </li>
        ))}
      </ul>
    </Group>
  );
}

function StatisticsView({ stats, everyone }: { stats: PlayStatistics; everyone: boolean }) {
  const time = timeline(stats);
  const hours = useMemo(() => Array.from({ length: 24 }, (_, hour) => String(hour).padStart(2, "0")), []);
  return (
    <div className="flex min-w-0 flex-col gap-8 pb-4">
      <Totals stats={stats} />
      <Group title={time.weekly ? "Time per week" : "Time per day"}>
        <div className="h-72 min-w-0">
          <ColumnChart
            label={`Hours watched and listened ${time.weekly ? "per week" : "per day"}`}
            categories={time.categories}
            categoryLabel={time.weekly ? "Week of" : "Day"}
            series={time.series}
            layout="stacked"
            unit="h"
            valueLabel="Hours"
          />
        </div>
      </Group>
      <div className="grid min-w-0 gap-8 [grid-template-columns:repeat(auto-fit,minmax(min(100%,24rem),1fr))]">
        <TopList title="Top shows" items={stats.topShows} empty="No episodes played in this period" />
        <TopList title="Top movies" items={stats.topMovies} empty="No movies played in this period" />
        <TopList title="Top artists" items={stats.topArtists} empty="No music played in this period" />
        <TopList title="Top albums" items={stats.topAlbums} empty="No music played in this period" />
        <TopList title="Top authors" items={stats.topAuthors} empty="No audiobooks played in this period" />
        <TopList title="Top genres" items={stats.topGenres} empty="Nothing with a genre played in this period" />
        <Group title="Devices">
          <div className="h-64 min-w-0">
            <BarChart
              orientation="horizontal"
              label="Hours played per device"
              categories={stats.devices.map((device) => device.name)}
              categoryLabel="Device"
              series={[{ key: "hours", label: "Hours", data: stats.devices.map((device) => toHours(device.seconds)) }]}
              unit="h"
              state={stats.devices.length === 0 ? "empty" : "ready"}
              emptyText="No devices played anything in this period"
            />
          </div>
        </Group>
        {everyone ? (
          <Group title="People">
            <div className="h-64 min-w-0">
              <BarChart
                orientation="horizontal"
                label="Hours played per person"
                categories={stats.users.map((user) => user.name)}
                categoryLabel="Person"
                series={[{ key: "hours", label: "Hours", data: stats.users.map((user) => toHours(user.seconds)) }]}
                unit="h"
              />
            </div>
          </Group>
        ) : null}
        <Group title="Time of day">
          <div className="h-64 min-w-0">
            <ColumnChart label="Hours played by hour of the day" categories={hours} categoryLabel="Hour" series={[{ key: "hours", label: "Hours", data: stats.hours.map(toHours) }]} unit="h" />
          </div>
        </Group>
      </div>
      <Recent plays={stats.recent} />
      {!stats.complete ? <p className="text-body-sm text-warning">This period has more plays than one view reads; the oldest are left out. Choose a shorter period.</p> : null}
    </div>
  );
}

/**
 * Watch and listening statistics: yours, someone's who shares theirs with you, or (administrators)
 * anyone's and everyone's together. The period and person are in the URL.
 */
export function StatisticsPage({ period, person }: { period: PeriodKey; person?: string }) {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const [sharing, setSharing] = useState(false);
  const people = useQuery(StatisticsPeopleDocument);
  const everyone = isAdmin && person === EVERYONE;
  const target = everyone ? undefined : person && person !== user?.id ? person : undefined;
  // One range per period choice, so the query and its live refreshes ask for the same window.
  const range = useMemo(() => periodRange(period), [period]);
  const stats = useQuery(PlayStatisticsDocument, {
    variables: { ...range, userId: target ?? null, allUsers: everyone, top: 10 },
  });
  const data = (stats.data ?? stats.previousData)?.playStatistics;
  const options = useMemo(() => {
    const list = (people.data?.statisticsPeople ?? []).map((entry) => ({ value: entry.userId, label: entry.isSelf ? `${entry.name} (you)` : entry.name }));
    return isAdmin ? [...list, { value: EVERYONE, label: "Everyone" }] : list;
  }, [people.data, isAdmin]);
  const selected = everyone ? EVERYONE : target ?? user?.id ?? null;
  const own = !everyone && !target;

  const setSearch = (next: { period?: PeriodKey; user?: string | undefined }) =>
    void navigate({ to: "/dashboard/statistics", search: (previous: Record<string, unknown>) => ({ ...previous, ...next }) as never, replace: true });

  return (
    <div className="flex min-w-0 flex-col gap-6">
      <div className="flex min-w-0 flex-wrap items-center gap-3" data-nav-section>
        {options.length > 1 ? (
          <Select aria-label="Whose statistics" value={selected} onValueChange={(value) => setSearch({ user: !value || value === user?.id ? undefined : value })} options={options} className="w-60 max-w-full" />
        ) : null}
        <Segmented aria-label="Period" size="sm" value={period} onValueChange={(value) => setSearch({ period: value as PeriodKey })} options={PERIODS.map((entry) => ({ value: entry.value, label: entry.label }))} />
        {own ? (
          <Button variant="secondary" size="sm" className="ms-auto" onPress={() => setSharing(true)} data-tv-autofocus>
            <IconShare /> Share
          </Button>
        ) : data?.userName && !everyone ? (
          <p className="ms-auto text-body-sm text-muted">{data.userName} shares these with you. They are read-only.</p>
        ) : null}
      </div>
      {stats.error && !data ? (
        <EmptyState icon={IconWarning} title="Could not load statistics" description={errorMessage(stats.error)} action={<Button onPress={() => void stats.refetch()}>Retry</Button>} />
      ) : !data ? (
        <div className="h-96 min-w-0 animate-pulse rounded-[var(--radius-surface)] bg-surface-secondary" aria-busy="true" aria-label="Loading statistics" />
      ) : !hasPlays(data) ? (
        <EmptyState icon={IconShare} title="Nothing played in this period" description={own ? "What you watch and listen to is counted from here on, on every device." : "Nothing was played in this period."} />
      ) : (
        <StatisticsView stats={data} everyone={everyone} />
      )}
      {user && sharing ? <ShareStatisticsDialog userId={user.id} open={sharing} onOpenChange={setSharing} /> : null}
    </div>
  );
}
