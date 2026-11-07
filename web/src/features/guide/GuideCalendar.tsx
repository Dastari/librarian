import { Link } from "@tanstack/react-router";
import { Tooltip } from "glass-ux/tooltip";
import { useMemo, useState, type ReactNode } from "react";

import { PageScroller } from "@/components/shell/PageScroller";
import { SkeletonBlock } from "@/components/ui";
import { GuideEpisodesDocument } from "@/graphql/generated/graphql";
import { showPoster } from "@/lib/artwork";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { cn } from "@/lib/utils";

import { GuidePoster, GuideToolbar, useLibraryShowActions } from "./GuidePoster";
import { addDays, episodeRange, episodesByDay, groupDayByShow, isoDate, parseDay, relativeDay, startOfToday, upcomingByShow, type GuideEpisode } from "./guide-model";

const WEEKDAYS = Array.from({ length: 7 }, (_, index) => new Date(2024, 0, 1 + index).toLocaleDateString(undefined, { weekday: "short" }));

/** Monday-first weeks covering the whole month. */
function monthGrid(month: Date): Date[] {
  const first = new Date(month.getFullYear(), month.getMonth(), 1);
  const start = addDays(first, -((first.getDay() + 6) % 7));
  const last = new Date(month.getFullYear(), month.getMonth() + 1, 0);
  const end = addDays(last, 6 - ((last.getDay() + 6) % 7));
  const days: Date[] = [];
  for (let day = start; day <= end; day = addDays(day, 1)) days.push(day);
  return days;
}

/**
 * Your shows on a month calendar. Each air day shows its shows as posters; choosing a day
 * lists its episodes beside the grid. Starts on the month of the next airing.
 */
export function GuideCalendar({ switcher }: { switcher: ReactNode }) {
  const today = useMemo(() => startOfToday(), []);
  const todayIso = isoDate(today);
  const horizon = useQuery(GuideEpisodesDocument, { variables: { from: todayIso, to: isoDate(addDays(today, 400)) } });
  const upcoming = useMemo(() => upcomingByShow(horizon.data?.episodes.edges.map((edge) => edge.node) ?? [], today), [horizon.data, today]);
  const firstAiring = upcoming[0]?.next.airDate?.slice(0, 10);

  const [chosenMonth, setChosenMonth] = useState<Date | null>(null);
  const [chosenDay, setChosenDay] = useState<string | null>(null);
  // Until someone navigates, open on the month (and day) of the next airing.
  const month = chosenMonth ?? (firstAiring && daysAhead(firstAiring, today) <= 62 ? monthOf(parseDay(firstAiring)) : monthOf(today));
  const cells = useMemo(() => monthGrid(month), [month]);
  const range = { from: isoDate(cells[0]!), to: isoDate(cells[cells.length - 1]!) };
  const monthEpisodes = useQuery(GuideEpisodesDocument, { variables: range });
  const byDay = useMemo(() => episodesByDay(monthEpisodes.data?.episodes.edges.map((edge) => edge.node) ?? []), [monthEpisodes.data]);

  const inMonth = (iso: string) => parseDay(iso).getMonth() === month.getMonth();
  const defaultDay = firstAiring && byDay.has(firstAiring) ? firstAiring : byDay.has(todayIso) ? todayIso : [...byDay.keys()].find((day) => day >= todayIso && inMonth(day)) ?? todayIso;
  const selected = chosenDay ?? defaultDay;
  const step = (delta: number) => {
    setChosenMonth(new Date(month.getFullYear(), month.getMonth() + delta, 1));
    setChosenDay(null);
  };
  const { actions, dialog } = useLibraryShowActions();
  const total = [...byDay.entries()].filter(([day]) => inMonth(day)).reduce((sum, [, list]) => sum + list.length, 0);

  return (
    <>
      <GuideToolbar
        noun="month"
        title={month.toLocaleDateString(undefined, { month: "long", year: "numeric" })}
        onPrevious={() => step(-1)}
        onToday={() => { setChosenMonth(monthOf(today)); setChosenDay(todayIso); }}
        onNext={() => step(1)}
        meta={<span className="text-label-sm text-muted">{total ? `${total} episode${total === 1 ? "" : "s"} this month` : "Nothing from your shows this month"}</span>}
        switcher={switcher}
      />
      <PageScroller className="page-gutter flex-1">
    <div className="grid min-w-0 gap-5 lg:grid-cols-[minmax(0,1fr)_22rem]">
      <section aria-label="Calendar" data-nav-section className="flex min-w-0 flex-col lg:min-h-[calc(var(--view-h,100svh)-1.5rem)] lg:self-start">
        <div role="grid" aria-label="Days" data-nav-group className="grid min-h-0 flex-1 grid-cols-7 gap-1.5" style={{ gridTemplateRows: `auto repeat(${cells.length / 7}, minmax(4.25rem, 1fr))` }}>
          {WEEKDAYS.map((name) => <div key={name} role="columnheader" className="px-2 pb-1 text-overline text-muted">{name}</div>)}
          {cells.map((day) => {
            const iso = isoDate(day);
            const groups = groupDayByShow(byDay.get(iso) ?? []);
            return (
              <Tooltip key={iso} enabled={groups.length > 0} content={groups.map((group) => `${group.show.name} · S${group.episodes[0]!.season} ${episodeRange(group.episodes)}`).join(", ")}>
                <button
                  type="button"
                  role="gridcell"
                  aria-selected={iso === selected}
                  aria-label={`${day.toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long" })}${groups.length ? `, ${groups.map((group) => group.show.name).join(", ")}` : ""}`}
                  onClick={() => setChosenDay(iso)}
                  data-focusable
                  className={cn(
                    "nav-focus guide-day relative flex min-w-0 flex-col items-stretch gap-1 rounded-xl border p-1.5 text-start transition-colors sm:p-2",
                    "border-white/5 bg-white/[0.03] hover-capable:hover:bg-white/[0.06]",
                    !inMonth(iso) && "opacity-40",
                    iso === todayIso && "border-brand/60 bg-brand-soft",
                    iso === selected && "border-foreground/40 bg-white/[0.08]",
                  )}
                >
                  <span className={cn("guide-day-date text-label-sm text-muted", iso === todayIso && "font-semibold text-brand")}>{day.getDate()}{iso === todayIso ? <span className="guide-day-today"> · Today</span> : null}</span>
                  {groups.slice(0, 1).map((group) => (
                    <span key={group.show.id} className="mt-auto flex min-h-0 min-w-0 items-end gap-1.5">
                      <GuidePoster size="day" name={group.show.name} image={showPoster(group.show)} actions={actions} target={group.show} />
                      <span className="guide-day-text min-w-0 text-label-sm leading-tight">
                        <span className="guide-day-title text-foreground">{group.show.name}</span>
                        <span className="block truncate text-muted">S{group.episodes[0]!.season} {episodeRange(group.episodes)}</span>
                      </span>
                    </span>
                  ))}
                  {groups.length > 1 ? <span className="absolute end-1.5 top-1.5 rounded-pill bg-white/10 px-1.5 text-label-sm text-muted">+{groups.length - 1}</span> : null}
                </button>
              </Tooltip>
            );
          })}
        </div>
      </section>

      <DayPanel day={selected} episodes={byDay.get(selected) ?? []} loading={monthEpisodes.loading && !monthEpisodes.data} upcoming={upcoming} today={today} actions={actions} />
    </div>
      </PageScroller>
      {dialog}
    </>
  );
}

const monthOf = (date: Date) => new Date(date.getFullYear(), date.getMonth(), 1);
const daysAhead = (iso: string, today: Date) => Math.round((parseDay(iso).getTime() - today.getTime()) / 86_400_000);

function DayPanel({ day, episodes, loading, upcoming, today, actions }: { day: string; episodes: GuideEpisode[]; loading: boolean; upcoming: ReturnType<typeof upcomingByShow>; today: Date; actions: ReturnType<typeof useLibraryShowActions>["actions"] }) {
  const groups = groupDayByShow(episodes);
  const later = upcoming.filter((item) => item.next.airDate!.slice(0, 10) > day).slice(0, 4);
  return (
    <aside aria-label="Selected day" data-nav-section className="glass-panel flex min-w-0 flex-col gap-4 rounded-card p-5">
      <p className="text-overline text-muted">{parseDay(day).toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long" })} · {relativeDay(day, today)}</p>
      {loading ? <SkeletonBlock className="h-40 rounded-xl" /> : groups.length === 0 ? <p className="text-body-sm text-muted">Nothing from your shows on this day.</p> : groups.map((group) => (
        <div key={group.show.id} className="flex flex-col gap-2">
          <div className="grid grid-cols-[4.5rem_minmax(0,1fr)] items-center gap-3">
            <GuidePoster size="panel" name={group.show.name} image={showPoster(group.show)} to="/shows/$showId" params={{ showId: group.show.id }} actions={actions} target={group.show} />
            <Link to="/shows/$showId" params={{ showId: group.show.id }} data-tv-skip className="nav-focus min-w-0 rounded-xl">
              <span className="block text-title-md text-foreground">{group.show.name}</span>
              <span className="block text-label-sm text-muted">{[group.episodes[0]!.episode === 1 ? `Season ${group.episodes[0]!.season} premiere` : `Season ${group.episodes[0]!.season}`, group.show.network].filter(Boolean).join(" · ")}</span>
              {group.show.autoDownload ? <span className="mt-1.5 inline-block rounded-pill bg-brand-soft px-2 py-0.5 text-label-sm text-brand">Will auto-download</span> : null}
            </Link>
          </div>
          <ol className="flex flex-col gap-1.5">
            {group.episodes.map((episode) => (
              <li key={episode.id}>
                <Link to="/shows/$showId" params={{ showId: group.show.id }} search={{ season: episode.season, episode: episode.id }} data-focusable className="nav-focus grid grid-cols-[2.75rem_minmax(0,1fr)] gap-2 rounded-xl border border-white/5 bg-white/[0.04] px-3 py-2 hover-capable:hover:bg-white/[0.08]">
                  <span className="text-label-sm font-semibold text-brand">E{String(episode.episode).padStart(2, "0")}</span>
                  <span className="min-w-0 text-label">
                    <span className="block truncate text-foreground">{episode.title && episode.title !== "TBA" ? episode.title : "Title to be announced"}</span>
                    {episode.mediaFileId ? <span className="text-label-sm text-success">In your library</span> : null}
                  </span>
                </Link>
              </li>
            ))}
          </ol>
        </div>
      ))}
      {later.length > 0 ? (
        <div className="flex flex-col gap-2">
          <p className="text-overline text-muted">Later</p>
          {later.map((item) => (
            <div key={item.show.id} className="flex items-center gap-3 rounded-xl border border-white/5 bg-white/[0.04] p-2">
              <GuidePoster size="row" name={item.show.name} image={showPoster(item.show)} to="/shows/$showId" params={{ showId: item.show.id }} actions={actions} target={item.show} />
              <Link to="/shows/$showId" params={{ showId: item.show.id }} data-tv-skip className="nav-focus min-w-0 rounded-md text-label">
                <span className="block truncate text-foreground">{item.show.name}{item.premiere ? ` · Season ${item.season}` : ""}</span>
                <span className="block truncate text-label-sm text-muted">{parseDay(item.next.airDate!).toLocaleDateString(undefined, { day: "numeric", month: "short", year: parseDay(item.next.airDate!).getFullYear() !== today.getFullYear() ? "numeric" : undefined })} · {relativeDay(item.next.airDate!, today)}</span>
              </Link>
            </div>
          ))}
        </div>
      ) : null}
    </aside>
  );
}
