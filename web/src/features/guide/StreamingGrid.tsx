import { Link, useNavigate } from "@tanstack/react-router";
import { Button } from "glass-ux/button";
import { IconChevronLeft, IconChevronRight } from "glass-ux/icons";
import { MultiSelect } from "glass-ux/multi-select";
import { Segmented } from "glass-ux/segmented";
import { useMemo, useState, type ReactNode } from "react";

import { Artwork, ErrorState, SkeletonBlock } from "@/components/ui";
import { GuideEpisodesDocument } from "@/graphql/generated/graphql";
import { artworkUrl } from "@/lib/api/urls";
import { showPoster } from "@/lib/artwork";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { cn } from "@/lib/utils";

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
const ROWS_COLLAPSED = 10;
const TILES_PER_CELL = 3;

/**
 * The week's listings as a services × days grid. Streaming releases have days rather than
 * time slots, so each cell holds that service's shows for the day. Your shows stay pinned
 * at the top; filters hide the news and daily listings that crowd the full schedule.
 */
export function StreamingGrid() {
  const today = useMemo(() => startOfToday(), []);
  const [weekOffset, setWeekOffset] = useState(0);
  const days = useMemo(() => Array.from({ length: DAYS }, (_, index) => isoDate(addDays(today, weekOffset * DAYS + index))), [today, weekOffset]);
  const range = { from: days[0]!, to: days[DAYS - 1]! };
  const { country, setCountry, synced } = useScheduleCountry();
  const schedule = useScheduleRange(range, country);
  const library = useQuery(GuideEpisodesDocument, { variables: range });
  const [filters, setFilters] = useState<GridFilters>({ hideNoise: true, genres: [], newOnly: false });
  const [chosenServices, setChosenServices] = useChosenServices();
  const [expanded, setExpanded] = useState(false);

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
  const rows = expanded ? grid : grid.slice(0, ROWS_COLLAPSED);
  const todayIso = isoDate(today);
  const toggleGenre = (genre: string) => setFilters((current) => ({ ...current, genres: current.genres.includes(genre) ? current.genres.filter((item) => item !== genre) : [...current.genres, genre] }));

  return (
    <section aria-label="Streaming grid" data-nav-section className="glass-panel min-w-0 rounded-card p-4 sm:p-5">
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="ghost" shape="circle" size="sm" icon={<IconChevronLeft className="rtl:rotate-180" />} aria-label="Previous week" onClick={() => setWeekOffset((value) => value - 1)} disabled={weekOffset <= -2} data-focusable />
        <Button variant="ghost" shape="circle" size="sm" icon={<IconChevronRight className="rtl:rotate-180" />} aria-label="Next week" onClick={() => setWeekOffset((value) => value + 1)} disabled={weekOffset >= 2} data-focusable />
        <h2 className="text-title-md text-foreground me-2" aria-live="polite">{weekOffset === 0 ? "This week" : `${parseDay(days[0]!).toLocaleDateString(undefined, { day: "numeric", month: "short" })} – ${parseDay(days[DAYS - 1]!).toLocaleDateString(undefined, { day: "numeric", month: "short" })}`}</h2>
        {synced.length > 1 ? <Segmented aria-label="Country" size="sm" value={country ?? synced[0]!} onValueChange={setCountry} options={synced.map((item) => ({ value: item, label: item }))} /> : country ? <span className="text-label-sm text-muted">{country} listings</span> : null}
      </div>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <MultiSelect aria-label="Services" size="sm" width="full" className="max-w-xl" placeholder="All services" options={serviceOptions} value={services} onValueChange={setChosenServices} />
        {chosenServices ? <Button variant="ghost" size="sm" onClick={() => { setChosenServices(networks.map((item) => item.network).filter(isStreamingService)); }} data-focusable>Streaming services</Button> : null}
      </div>
      <div role="group" aria-label="Filters" data-nav-group className="mt-2 flex flex-wrap gap-1.5">
        <FilterChip pressed={filters.hideNoise} onPress={() => setFilters((current) => ({ ...current, hideNoise: !current.hideNoise }))}>Hide news &amp; daily</FilterChip>
        <FilterChip pressed={filters.newOnly} onPress={() => setFilters((current) => ({ ...current, newOnly: !current.newOnly }))}>Premieres only</FilterChip>
        {genres.map((genre) => <FilterChip key={genre} pressed={filters.genres.includes(genre)} onPress={() => toggleGenre(genre)}>{genre}</FilterChip>)}
      </div>

      {schedule.error && !schedule.rows ? <ErrorState error={schedule.error} onRetry={() => void schedule.refetch()} className="mt-4" /> : (
        <div className="mt-4 overflow-x-auto">
          <div role="table" aria-label="Listings by service and day" className="grid min-w-[56rem] grid-cols-[9.5rem_repeat(7,minmax(0,1fr))] overflow-hidden rounded-2xl border border-white/10">
            <div role="row" className="contents">
              <div role="columnheader" className="bg-black/25" />
              {days.map((day) => (
                <div key={day} role="columnheader" className={cn("border-s border-white/5 bg-black/25 px-3 py-2 text-label-sm text-muted", day === todayIso && "bg-brand-soft")}>
                  <span className={cn("block text-label font-semibold text-foreground", day === todayIso && "text-brand")}>{day === todayIso ? "Today" : parseDay(day).toLocaleDateString(undefined, { weekday: "short" })}</span>
                  {parseDay(day).toLocaleDateString(undefined, { day: "numeric", month: "short" })}
                </div>
              ))}
            </div>

            <YourShowsRow days={days} todayIso={todayIso} episodes={libraryEpisodes} loading={library.loading && !library.data} />

            {schedule.loading && !schedule.rows ? (
              <div role="row" className="contents">
                <div role="cell" className="col-span-8 p-3"><SkeletonBlock className="h-48 rounded-xl" /></div>
              </div>
            ) : rows.length === 0 ? (
              <div role="row" className="contents">
                <div role="cell" className="col-span-8 px-4 py-8 text-center text-body-sm text-muted">No listings match these filters.</div>
              </div>
            ) : rows.map((row) => (
              <div key={row.network} role="row" data-nav-group className="contents">
                <div role="rowheader" className="flex flex-col justify-center gap-0.5 border-t border-white/5 bg-black/15 px-3 py-2">
                  <span className="truncate text-label font-semibold text-foreground">{row.network}</span>
                  <span className="text-label-sm text-muted">{row.total} show{row.total === 1 ? "" : "s"}</span>
                </div>
                {days.map((day) => <ScheduleCell key={day} shows={row.days.get(day) ?? []} today={day === todayIso} />)}
              </div>
            ))}
          </div>
          {grid.length > ROWS_COLLAPSED ? (
            <div className="mt-3 flex justify-center">
              <Button variant="ghost" size="sm" onClick={() => setExpanded((value) => !value)} data-focusable>{expanded ? "Show fewer services" : `Show all ${grid.length} services`}</Button>
            </div>
          ) : null}
        </div>
      )}
    </section>
  );
}

function FilterChip({ pressed, onPress, children }: { pressed: boolean; onPress: () => void; children: ReactNode }) {
  return <Button variant={pressed ? "secondary" : "ghost"} size="sm" aria-pressed={pressed} onClick={onPress} className={cn("rounded-pill", pressed && "text-brand")} data-focusable>{children}</Button>;
}

function YourShowsRow({ days, todayIso, episodes, loading }: { days: string[]; todayIso: string; episodes: GuideEpisode[]; loading: boolean }) {
  const byDay = new Map(days.map((day) => [day, groupDayByShow(episodes.filter((episode) => episode.airDate?.slice(0, 10) === day))]));
  const any = [...byDay.values()].some((groups) => groups.length > 0);
  return (
    <div role="row" data-nav-group className="contents">
      <div role="rowheader" className="flex flex-col justify-center gap-0.5 border-t border-white/5 bg-brand-soft px-3 py-2">
        <span className="text-label font-semibold text-foreground">Your shows</span>
        <Link to="/guide" search={{ view: "calendar" }} className="nav-focus text-label-sm text-muted hover:text-foreground" data-tv-skip>Open calendar</Link>
      </div>
      {loading ? <div role="cell" className="col-span-7 border-t border-white/5 p-3"><SkeletonBlock className="h-20 rounded-xl" /></div>
        : !any ? <div role="cell" className="col-span-7 flex items-center border-t border-white/5 px-4 py-3 text-label-sm text-muted">Nothing from your shows this week.</div>
        : days.map((day) => (
          <div key={day} role="cell" className={cn("flex min-h-28 flex-wrap content-start gap-1.5 border-s border-t border-white/5 p-2", day === todayIso && "bg-brand-soft/40")}>
            {byDay.get(day)!.map((group) => (
              <Link key={group.show.id} to="/shows/$showId" params={{ showId: group.show.id }} data-focusable aria-label={`${group.show.name} ${episodeRange(group.episodes)}`} className="nav-focus tv-lift relative w-14 rounded-md">
                <Artwork src={showPoster(group.show)} alt="" aspect="poster" displayWidth={56} tint={LIBRARY_TYPES.tv.tintVar} className="tv-lift-target rounded-md" />
                <span className="absolute -end-1 -top-1 rounded-pill border border-white/10 bg-background px-1 text-[0.625rem] text-muted">{episodeRange(group.episodes)}</span>
              </Link>
            ))}
          </div>
        ))}
    </div>
  );
}

function ScheduleCell({ shows, today }: { shows: ScheduledShow[]; today: boolean }) {
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const visible = open ? shows : shows.slice(0, TILES_PER_CELL);
  return (
    <div role="cell" className={cn("flex min-h-28 flex-wrap content-start gap-1.5 border-s border-t border-white/5 p-2", today && "bg-brand-soft/40")}>
      {visible.map((show) => (
        <button
          key={show.key}
          type="button"
          onClick={() => void navigate({ to: "/search", search: { q: show.name, in: "show" } as never })}
          aria-label={`${show.name}, ${scheduledLabel(show)}${show.network ? `, ${show.network}` : ""}. Find and add`}
          title={`${show.name} · ${scheduledLabel(show)}`}
          data-focusable
          className="nav-focus tv-lift relative w-14 rounded-md text-start"
        >
          <Artwork src={show.posterUrl ? artworkUrl("guide", show.tvmazeShowId, "poster") : undefined} alt="" aspect="poster" displayWidth={56} tint={LIBRARY_TYPES.tv.tintVar} className="tv-lift-target rounded-md" fallback={<span className="px-1 text-center text-[0.625rem] leading-tight text-foreground/80">{show.name}</span>} />
          {show.newShow || show.premiere ? <span className="absolute bottom-1 start-1 rounded bg-brand px-1 text-[0.5625rem] font-bold uppercase text-brand-foreground">{show.newShow ? "New" : `S${show.season}`}</span> : null}
        </button>
      ))}
      {shows.length > TILES_PER_CELL && !open ? (
        <button type="button" onClick={() => setOpen(true)} data-focusable className="nav-focus grid aspect-[2/3] w-14 place-items-center rounded-md border border-dashed border-white/15 text-label-sm text-muted">+{shows.length - TILES_PER_CELL}</button>
      ) : null}
    </div>
  );
}
