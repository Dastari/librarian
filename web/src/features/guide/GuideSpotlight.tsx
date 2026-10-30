import { Link, useNavigate } from "@tanstack/react-router";
import { Button } from "glass-ux/button";
import { IconCalendar, IconCheck, IconInfo, IconPlus } from "glass-ux/icons";
import { useMemo } from "react";

import { Artwork, MediaRow, PosterCard, SkeletonBlock } from "@/components/ui";
import { GuideEpisodesDocument } from "@/graphql/generated/graphql";
import { artworkUrl } from "@/lib/api/urls";
import { showBackdrop, showPoster } from "@/lib/artwork";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { addDays, daysUntil, episodeRange, isoDate, parseDay, relativeDay, releaseBatches, scheduledLabel, startOfToday, tonightPicks, upcomingByShow, type ShowSchedule } from "./guide-model";
import { useScheduleCountry, useScheduleRange } from "./useSchedule";

/** How far ahead the spotlight looks for the next episode of a library show. */
const HORIZON_DAYS = 400;
const BATCHES_SHOWN = 4;

const dayMonth = (day: string) => parseDay(day).toLocaleDateString(undefined, { day: "numeric", month: "short" });
const longDay = (day: string) => parseDay(day).toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long" });

/**
 * Home's TV guide: the next of your shows to air over its own artwork with a countdown and
 * its release schedule, your other upcoming shows beneath it, and tonight's new listings.
 * The full calendar and streaming grid live on /guide.
 */
export function GuideSpotlight() {
  const today = useMemo(() => startOfToday(), []);
  const todayIso = isoDate(today);
  const range = useMemo(() => ({ from: todayIso, to: isoDate(addDays(today, HORIZON_DAYS)) }), [today, todayIso]);
  const library = useQuery(GuideEpisodesDocument, { variables: range });
  const { country } = useScheduleCountry();
  const schedule = useScheduleRange({ from: todayIso, to: todayIso }, country);

  const shows = useMemo(() => upcomingByShow(library.data?.episodes.edges.map((edge) => edge.node) ?? [], today), [library.data, today]);
  const libraryNames = useMemo(() => new Set((library.data?.episodes.edges ?? []).map((edge) => edge.node.show?.name.toLowerCase()).filter((name): name is string => Boolean(name))), [library.data]);
  const tonight = useMemo(() => tonightPicks(schedule.rows ?? [], todayIso, libraryNames), [schedule.rows, todayIso, libraryNames]);

  const [next, ...later] = shows;
  const loading = library.loading && !library.data;

  return (
    <section aria-label="TV guide" data-nav-section className="glass-panel relative isolate overflow-hidden rounded-card">
      {loading ? <SkeletonBlock className="h-[26rem] rounded-none" /> : next ? <NextUp schedule={next} today={today} /> : <NothingScheduled />}

      {later.length > 0 || next ? (
        <div data-nav-group className="relative z-10 flex flex-wrap items-stretch gap-3 px-5 pb-5 sm:px-7">
          {later.slice(0, 4).map((item) => (
            <Link key={item.show.id} to="/shows/$showId" params={{ showId: item.show.id }} data-focusable className="nav-focus glass-control flex min-w-0 items-center gap-3 rounded-xl py-2 pl-2 pr-4 hover-capable:hover:brightness-110">
              <Artwork src={showPoster(item.show)} alt="" aspect="poster" displayWidth={40} tint={LIBRARY_TYPES.tv.tintVar} className="w-10 shrink-0 rounded-md" />
              <span className="min-w-0">
                <span className="block truncate text-label text-foreground">{item.show.name}{item.premiere ? ` · Season ${item.season}` : ""}</span>
                <span className="block truncate text-label-sm text-muted">{[`${dayMonth(item.next.airDate!)} (${relativeDay(item.next.airDate!, today)})`, item.show.network].filter(Boolean).join(" · ")}</span>
              </span>
            </Link>
          ))}
          <Link to="/guide" data-focusable className="nav-focus glass-control ms-auto flex items-center gap-3 rounded-xl px-4 py-2 hover-capable:hover:brightness-110">
            <IconCalendar aria-hidden />
            <span>
              <span className="block text-label text-foreground">Full guide</span>
              <span className="block text-label-sm text-muted">Calendar and streaming grid</span>
            </span>
          </Link>
        </div>
      ) : null}

      {tonight.length > 0 ? (
        <div className="relative z-10 flex flex-col gap-3 pb-5">
          <div className="flex flex-wrap items-baseline gap-x-3 px-5 sm:px-7">
            <h3 className="text-title-md text-foreground">New tonight</h3>
            <span className="text-label-sm text-muted">From the {country ?? ""} schedule · premieres first</span>
          </div>
          <TonightRow picks={tonight} />
        </div>
      ) : null}
    </section>
  );
}

function NextUp({ schedule, today }: { schedule: ShowSchedule; today: Date }) {
  const navigate = useNavigate();
  const { show, next, season, episodes, premiere } = schedule;
  const batches = releaseBatches(episodes);
  const days = Math.max(0, daysUntil(next.airDate!, today));
  const weeks = batches.length > 1 ? Math.max(1, Math.round(daysUntil(batches[batches.length - 1]!.date, parseDay(batches[0]!.date)) / 7) + 1) : 1;
  const airsToday = days === 0;
  const headline = airsToday
    ? `${episodeCode(next)} airs today`
    : premiere
      ? `Season ${season} premieres ${longDay(next.airDate!)}`
      : `${episodeCode(next)} airs ${longDay(next.airDate!)}`;
  const cadence = batches.length > 1 && batches[0]!.episodes.length > 1 ? `${batches[0]!.episodes.length} episodes weekly` : batches.length > 1 ? "Weekly" : null;

  return (
    <div className="relative min-h-[26rem]">
      {show.backdropUrl ? <Artwork src={showBackdrop(show)} alt="" aspect="backdrop" displayWidth={1280} tint={LIBRARY_TYPES.tv.tintVar} className="guide-spotlight-art absolute inset-0 h-full w-full" imageClassName="object-cover object-[center_25%]" /> : null}
      <div aria-hidden className="guide-spotlight-scrim absolute inset-0" />
      <div className="relative z-10 grid gap-6 p-5 sm:p-7 lg:grid-cols-[minmax(0,1fr)_minmax(0,28rem)] lg:items-center">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-overline text-muted">{airsToday ? "Tonight on your shows" : "Next up on your shows"}</span>
            {show.network ? <span className="glass-control rounded-pill px-2.5 py-0.5 text-label-sm text-muted">{show.network}</span> : null}
          </div>
          <h2 className="mt-2 max-w-[18ch] text-display-md text-foreground text-balance">{show.name}</h2>
          <p className="mt-2 text-body text-foreground/85">{[headline, cadence].filter(Boolean).join(" · ")}</p>
          <dl className="mt-5 flex flex-wrap gap-2.5" aria-label="Countdown">
            <Countdown value={airsToday ? "Today" : String(days)} label={airsToday ? "" : days === 1 ? "day" : "days"} />
            <Countdown value={String(episodes.length)} label={episodes.length === 1 ? "episode" : "episodes"} />
            {batches.length > 1 ? <Countdown value={String(weeks)} label={weeks === 1 ? "week" : "weeks"} /> : null}
          </dl>
          <div className="mt-5 flex flex-wrap items-center gap-3">
            <Button variant="primary" size="lg" icon={<IconInfo />} nativeButton={false} render={<Link to="/shows/$showId" params={{ showId: show.id }} />} data-tv-autofocus data-focusable>Open show</Button>
            {show.autoDownload ? <span className="glass-control flex items-center gap-2 rounded-pill px-4 py-2 text-label text-foreground"><IconCheck aria-hidden /> Auto-download on</span> : <Button variant="secondary" size="lg" onClick={() => void navigate({ to: "/shows/$showId", params: { showId: show.id } })} data-focusable>Set up downloads</Button>}
          </div>
        </div>
        <ol aria-label={`${show.name} season ${season} schedule`} className="glass-panel grid gap-1 rounded-2xl p-3">
          {batches.slice(0, BATCHES_SHOWN).map((batch) => (
            <li key={batch.date} className="grid grid-cols-[3.75rem_minmax(0,1fr)] gap-3 border-b border-separator px-2 py-2 last:border-b-0">
              <span className="text-label-sm leading-tight text-muted">
                <span className="block text-title-md text-foreground">{parseDay(batch.date).getDate()}</span>
                {parseDay(batch.date).toLocaleDateString(undefined, { month: "short", weekday: "short" })}
              </span>
              <span className="min-w-0 text-label leading-relaxed">
                {batch.episodes.map((episode) => (
                  <span key={episode.id} className="block truncate">
                    <span className="me-1.5 font-semibold text-brand">E{episode.episode}</span>
                    {episode.title && episode.title !== "TBA" ? episode.title : <span className="text-muted">Title to be announced</span>}
                  </span>
                ))}
              </span>
            </li>
          ))}
          {batches.length > BATCHES_SHOWN ? (
            <li className="px-2 pt-1 text-label-sm text-muted">+ {batches.slice(BATCHES_SHOWN).reduce((sum, batch) => sum + batch.episodes.length, 0)} more through {dayMonth(batches[batches.length - 1]!.date)}</li>
          ) : null}
        </ol>
      </div>
    </div>
  );
}

const episodeCode = (episode: { season: number; episode: number }) => `S${episode.season} ${episodeRange([episode])}`;

function Countdown({ value, label }: { value: string; label: string }) {
  return (
    <div className="glass-control min-w-[4.75rem] rounded-xl px-3.5 py-2 text-center">
      <dt className="sr-only">{label}</dt>
      <dd>
        <span className="block text-title-lg text-foreground">{value}</span>
        {label ? <span className="text-overline text-muted">{label}</span> : null}
      </dd>
    </div>
  );
}

function NothingScheduled() {
  return (
    <div className="flex flex-wrap items-center gap-4 p-5 sm:p-7">
      <div className="min-w-0 flex-1">
        <h2 className="text-title-lg text-foreground">TV guide</h2>
        <p className="text-body-sm text-muted">No upcoming episodes are listed for your shows yet.</p>
      </div>
    </div>
  );
}

function TonightRow({ picks }: { picks: ReturnType<typeof tonightPicks> }) {
  const navigate = useNavigate();
  return (
    <MediaRow ariaLabel="New tonight">
      {picks.map((pick) => (
        <PosterCard
          key={pick.key}
          width="compact"
          title={pick.name}
          meta={[scheduledLabel(pick), pick.network].filter(Boolean).join(" · ")}
          image={pick.posterUrl ? artworkUrl("guide", pick.tvmazeShowId, "poster") : undefined}
          tint={LIBRARY_TYPES.tv.tintVar}
          badge={pick.newShow || pick.premiere ? <span className="rounded bg-brand px-1.5 py-0.5 text-[0.625rem] font-bold uppercase tracking-wide text-brand-foreground">{pick.newShow ? "New" : `S${pick.season}`}</span> : undefined}
          actions={<IconPlus aria-label="Find and add" />}
          onPress={() => void navigate({ to: "/search", search: { q: pick.name, in: "show" } as never })}
        />
      ))}
    </MediaRow>
  );
}
