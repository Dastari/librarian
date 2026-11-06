import { Link, useNavigate } from "@tanstack/react-router";
import { Button } from "glass-ux/button";
import { MultiSelect } from "glass-ux/multi-select";
import { Segmented } from "glass-ux/segmented";
import { Tooltip } from "glass-ux/tooltip";
import type { ActionSet } from "glass-ux/actions";
import { useMemo, useState, type ReactNode } from "react";

import { ErrorState, SkeletonBlock } from "@/components/ui";
import { GuideEpisodesDocument } from "@/graphql/generated/graphql";
import { artworkUrl } from "@/lib/api/urls";
import { showPoster } from "@/lib/artwork";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { cn } from "@/lib/utils";

import { GuidePoster, GuideToolbar, scheduledShowActions, useLibraryShowActions, type LibraryShowTarget } from "./GuidePoster";
import { addDays, episodeRange, groupDayByShow, isStreamingService, isoDate, parseDay, scheduleGenres, scheduleNetworks, scheduledLabel, startOfToday, streamingGrid, type GridFilters, type GuideEpisode, type ScheduledShow } from "./guide-model";
import { useScheduleCountry, useScheduleRange } from "./useSchedule";

const DAYS = 7;
const SERVICES_KEY = "librarian.guide.services";

/** The services someone picked, remembered per browser; `null` until they choose. */
function useChosenServices() {
  const [chosen, setChosen] = useState<string[] | null>(() => {
    try {
      const saved = localStorage.getItem(SERVICES_KEY);
      return saved ? (JSON.parse(saved) as string[]) : null;
    } catch {
      return null;
    }
  });
  const update = (next: string[]) => {
    setChosen(next);
    try { localStorage.setItem(SERVICES_KEY, JSON.stringify(next)); } catch { /* private mode */ }
  };
  return [chosen, update] as const;
}
const TILES_PER_CELL = 3;

/**
 * The week's listings as a services × days grid. Streaming releases have days rather than
 * time slots, so each cell holds that service's shows for the day. Your shows stay pinned
 * at the top; filters hide the news and daily listings that crowd the full schedule.
 */
export function StreamingGrid({ switcher }: { switcher: ReactNode }) {
  const today = useMemo(() => startOfToday(), []);
  const [weekOffset, setWeekOffset] = useState(0);
  const days = useMemo(() => Array.from({ length: DAYS }, (_, index) => isoDate(addDays(today, weekOffset * DAYS + index))), [today, weekOffset]);
  const range = { from: days[0]!, to: days[DAYS - 1]! };
  const { country, setCountry, synced } = useScheduleCountry();
  const schedule = useScheduleRange(range, country);
  const library = useQuery(GuideEpisodesDocument, { variables: range });
  const [filters, setFilters] = useState<GridFilters>({ hideNoise: true, genres: [], newOnly: false });
  const [chosenServices, setChosenServices] = useChosenServices();
  const { actions: showActions, dialog } = useLibraryShowActions();

  const libraryEpisodes = useMemo(() => library.data?.episodes.edges.map((edge) => edge.node) ?? [], [library.data]);
  const libraryNames = useMemo(() => new Set(libraryEpisodes.map((episode) => episode.show?.name.toLowerCase()).filter((name): name is string => Boolean(name))), [libraryEpisodes]);
  const genres = useMemo(() => scheduleGenres(schedule.rows ?? []), [schedule.rows]);
  const networks = useMemo(() => scheduleNetworks(schedule.rows ?? []), [schedule.rows]);
  // Until someone picks, show the subscription streamers that have listings this week.
  const services = useMemo(() => chosenServices ?? networks.map((item) => item.network).filter(isStreamingService), [chosenServices, networks]);
  const serviceOptions = useMemo(() => {
    const known = new Set(networks.map((item) => item.network));
    return [...networks.map((item) => ({ value: item.network, label: `${item.network} (${item.count})` })), ...services.filter((name) => !known.has(name)).map((name) => ({ value: name, label: name }))];
  }, [networks, services]);
  const grid = useMemo(() => streamingGrid(schedule.rows ?? [], days, { ...filters, services }, libraryNames), [schedule.rows, days, filters, services, libraryNames]);
  const service = useMemo(() => ({
    only: (name: string) => setChosenServices([name]),
    hide: (name: string) => setChosenServices(services.filter((item) => item !== name)),
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }), [services]);
  const todayIso = isoDate(today);
  const weekLabel = weekOffset === 0 ? "This week" : `${parseDay(days[0]!).toLocaleDateString(undefined, { day: "numeric", month: "short" })} – ${parseDay(days[DAYS - 1]!).toLocaleDateString(undefined, { day: "numeric", month: "short" })}`;

  return (
    <>
      <GuideToolbar
        noun="week"
        title={weekLabel}
        onPrevious={() => setWeekOffset((value) => value - 1)}
        onNext={() => setWeekOffset((value) => value + 1)}
        previousDisabled={weekOffset <= -2}
        nextDisabled={weekOffset >= 2}
        switcher={switcher}
      >
        {synced.length > 1 ? <Segmented aria-label="Country" size="sm" value={country ?? synced[0]!} onValueChange={setCountry} options={synced.map((item) => ({ value: item, label: item }))} /> : null}
        <MultiSelect aria-label="Services" size="sm" width="full" className="w-60" placeholder="All services" options={serviceOptions} value={services} onValueChange={setChosenServices} />
        <MultiSelect aria-label="Genres" size="sm" width="full" className="w-44" placeholder="All genres" options={genres.map((genre) => ({ value: genre, label: genre }))} value={filters.genres} onValueChange={(value) => setFilters((current) => ({ ...current, genres: value }))} />
        <div role="group" aria-label="Filters" data-nav-group className="flex items-center gap-1">
          <FilterChip pressed={filters.hideNoise} onPress={() => setFilters((current) => ({ ...current, hideNoise: !current.hideNoise }))}>Hide news &amp; daily</FilterChip>
          <FilterChip pressed={filters.newOnly} onPress={() => setFilters((current) => ({ ...current, newOnly: !current.newOnly }))}>Premieres only</FilterChip>
        </div>
      </GuideToolbar>
      <section aria-label="Streaming grid" data-nav-section className="page-gutter flex min-h-0 min-w-0 flex-1 flex-col">
        {schedule.error && !schedule.rows ? <ErrorState error={schedule.error} onRetry={() => void schedule.refetch()} /> : (
          // The rows scroll here; the day headings and service names stay put.
          <div data-page-scroller className="guide-grid-scroll scrollbar-thin min-h-0 flex-1 overflow-auto rounded-2xl">
            <div role="table" aria-label="Listings by service and day" className="guide-grid grid min-w-[56rem] grid-cols-[9.5rem_repeat(7,minmax(0,1fr))]">
              <div role="row" className="contents">
                <div role="columnheader" aria-label="Service" className="guide-grid-corner" />
                {days.map((day) => (
                  <div key={day} role="columnheader" className={cn("guide-grid-day border-s border-(--line) px-3 py-2 text-label-sm text-muted", day === todayIso && "guide-grid-today")}>
                    <span className={cn("block text-label font-semibold text-foreground", day === todayIso && "text-brand")}>{day === todayIso ? "Today" : parseDay(day).toLocaleDateString(undefined, { weekday: "short" })}</span>
                    {parseDay(day).toLocaleDateString(undefined, { day: "numeric", month: "short" })}
                  </div>
                ))}
              </div>

              <YourShowsRow days={days} todayIso={todayIso} episodes={libraryEpisodes} loading={library.loading && !library.data} actions={showActions} />

              {schedule.loading && !schedule.rows ? (
                <div role="row" className="contents">
                  <div role="cell" className="col-span-8 p-3"><SkeletonBlock className="h-48 rounded-xl" /></div>
                </div>
              ) : grid.length === 0 ? (
                <div role="row" className="contents">
                  <div role="cell" className="col-span-8 px-4 py-8 text-center text-body-sm text-muted">No listings match these filters.</div>
                </div>
              ) : grid.map((row) => (
                <div key={row.network} role="row" data-nav-group className="contents">
                  <div role="rowheader" className="guide-grid-service flex flex-col justify-center gap-0.5 border-t border-(--line) px-3 py-2">
                    <span className="truncate text-label font-semibold text-foreground">{row.network}</span>
                    <span className="text-label-sm text-muted">{row.total} show{row.total === 1 ? "" : "s"}</span>
                  </div>
                  {days.map((day) => <ScheduleCell key={day} shows={row.days.get(day) ?? []} today={day === todayIso} network={row.network} service={service} />)}
                </div>
              ))}
            </div>
          </div>
        )}
      </section>
      {dialog}
    </>
  );
}

function FilterChip({ pressed, onPress, children }: { pressed: boolean; onPress: () => void; children: ReactNode }) {
  return <Button variant={pressed ? "secondary" : "ghost"} size="sm" aria-pressed={pressed} onClick={onPress} className={cn("rounded-pill", pressed && "text-brand")} data-focusable>{children}</Button>;
}

function YourShowsRow({ days, todayIso, episodes, loading, actions }: { days: string[]; todayIso: string; episodes: GuideEpisode[]; loading: boolean; actions: ActionSet<LibraryShowTarget> }) {
  const byDay = new Map(days.map((day) => [day, groupDayByShow(episodes.filter((episode) => episode.airDate?.slice(0, 10) === day))]));
  const any = [...byDay.values()].some((groups) => groups.length > 0);
  return (
    <div role="row" data-nav-group className="contents">
      <div role="rowheader" className="guide-grid-service guide-grid-yours flex flex-col justify-center gap-0.5 border-t border-(--line) px-3 py-2">
        <span className="text-label font-semibold text-foreground">Your shows</span>
        <Link to="/guide" search={{ view: "calendar" }} className="nav-focus text-label-sm text-muted hover:text-foreground" data-tv-skip>Open calendar</Link>
      </div>
      {loading ? <div role="cell" className="col-span-7 border-t border-(--line) p-3"><SkeletonBlock className="h-20 rounded-xl" /></div>
        : !any ? <div role="cell" className="col-span-7 flex items-center border-t border-(--line) px-4 py-3 text-label-sm text-muted">Nothing from your shows this week.</div>
        : days.map((day) => (
          <div key={day} role="cell" className={cn("flex min-h-28 flex-wrap content-start gap-1.5 border-s border-t border-(--line) p-2", day === todayIso && "bg-brand-soft/40")}>
            {byDay.get(day)!.map((group) => (
              <GuidePoster key={group.show.id} size="tile" name={group.show.name} image={showPoster(group.show)} badge={episodeRange(group.episodes)} tooltip={`${group.show.name} · ${episodeRange(group.episodes)}`} ariaLabel={`${group.show.name} ${episodeRange(group.episodes)}`} to="/shows/$showId" params={{ showId: group.show.id }} actions={actions} target={group.show} />
            ))}
          </div>
        ))}
    </div>
  );
}

function ScheduleCell({ shows, today, network, service }: { shows: ScheduledShow[]; today: boolean; network: string; service: { only: (name: string) => void; hide: (name: string) => void } }) {
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const visible = open ? shows : shows.slice(0, TILES_PER_CELL);
  const actions = useMemo(() => scheduledShowActions(network, (show) => void navigate({ to: "/discover/shows/$tvmazeId", params: { tvmazeId: String(show.tvmazeShowId) } }), service), [network, navigate, service]);
  return (
    <div role="cell" className={cn("flex min-h-28 flex-wrap content-start gap-1.5 border-s border-t border-(--line) p-2", today && "bg-brand-soft/40")}>
      {visible.map((show) => (
        <GuidePoster
          key={show.key}
          size="tile"
          name={show.name}
          image={show.posterUrl ? artworkUrl("guide", show.tvmazeShowId, "poster") : undefined}
          badge={show.newShow ? "New" : show.premiere ? `S${show.season}` : undefined}
          tooltip={`${show.name} · ${scheduledLabel(show)}`}
          ariaLabel={`${show.name}, ${scheduledLabel(show)}${show.network ? `, ${show.network}` : ""}`}
          onPress={() => void navigate({ to: "/discover/shows/$tvmazeId", params: { tvmazeId: String(show.tvmazeShowId) } })}
          actions={actions}
          target={show}
        />
      ))}
      {shows.length > TILES_PER_CELL && !open ? (
        <Tooltip content={`Show ${shows.length - TILES_PER_CELL} more`}><button type="button" onClick={() => setOpen(true)} data-focusable className="guide-tile guide-tile-more nav-focus grid aspect-[2/3] w-14 place-items-center rounded-md border border-dashed border-(--line-strong) text-label-sm text-muted">+{shows.length - TILES_PER_CELL}</button></Tooltip>
      ) : null}
    </div>
  );
}
