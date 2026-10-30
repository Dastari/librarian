import { useApolloClient } from "@apollo/client/react";
import { useLiveEpoch, useLiveInvalidation } from "@/lib/graphql/live/hooks";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Link, useNavigate } from "@tanstack/react-router";
import { IconPlus } from "glass-ux/icons";
import { ListRow } from "glass-ux/list-row";
import { Segmented } from "glass-ux/segmented";
import { Tabs } from "glass-ux/tabs";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Artwork, SkeletonBlock } from "@/components/ui";
import { artworkUrl } from "@/lib/api/urls";
import { NextLibraryEpisodesDocument, ScheduleCountriesDocument, ScheduleWeekDocument, UpcomingLibraryEpisodesDocument, type ScheduleWeekQuery } from "@/graphql/generated/graphql";
import { showPoster } from "@/lib/artwork";
import { formatDate, parseTimestamp } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { cn } from "@/lib/utils";

const DAYS = 7;
type Scope = "mine" | "all";

interface Airing {
  key: string;
  date: string;
  time: string | null;
  sortTime: number;
  showName: string;
  code: string;
  title: string | null;
  network: string | null;
  poster: string | undefined;
  inLibrary: boolean;
  showId?: string;
  hasFile?: boolean;
}

const isoDate = (date: Date) => date.toISOString().slice(0, 10);
const code = (season: number, episode: number) => (episode === 0 ? `S${String(season).padStart(2, "0")} Special` : `S${String(season).padStart(2, "0")}E${String(episode).padStart(2, "0")}`);
const clock = (stamp: string | null | undefined): { time: string | null; sort: number } => {
  const date = stamp ? parseTimestamp(stamp) : null;
  return date ? { time: date.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" }), sort: date.getTime() } : { time: null, sort: Number.MAX_SAFE_INTEGER };
};

/** The schedule cache is partitioned by country; prefer the browser's region when it has been synced. */
function localeCountry(): string | undefined {
  try {
    return new Intl.Locale(navigator.language).maximize().region ?? undefined;
  } catch {
    return undefined;
  }
}

/** Walks every page of the week's schedule for one country (a week is usually a few hundred rows). */
function useScheduleWeek(range: { from: string; to: string }, country: string | null, skip: boolean) {
  const client = useApolloClient();
  const epoch = useLiveEpoch();
  const key = JSON.stringify([epoch, range, country, skip]);
  const generation = useRef(0);
  const [snapshot, setSnapshot] = useState<{ key: string; data?: ScheduleWeekQuery; loading: boolean; error?: Error }>();
  const refetch = useCallback(async () => {
    if (skip || !country) return;
    const version = ++generation.current;
    setSnapshot((previous) => ({ key, data: previous?.key === key ? previous.data : undefined, loading: true }));
    try {
      let offset = 0;
      let data: ScheduleWeekQuery | undefined;
      const rows = new Map<string, ScheduleWeekQuery["scheduleCaches"]["edges"][number]>();
      do {
        const result = await client.query({ query: ScheduleWeekDocument, variables: { ...range, country, offset }, fetchPolicy: "no-cache", context: { queryDeduplication: false } });
        if (version !== generation.current) return;
        if (!result.data) throw new Error("Schedule unavailable");
        data = result.data;
        const edges = data.scheduleCaches.edges;
        if (!edges.length && offset < (data.scheduleCaches.pageInfo.totalCount ?? 0)) throw new Error("Schedule paging did not advance");
        edges.forEach((edge) => rows.set(edge.node.id, edge));
        offset += edges.length;
      } while (offset < (data.scheduleCaches.pageInfo.totalCount ?? 0) && offset < 1000);
      if (version === generation.current) setSnapshot({ key, loading: false, data: { ...data, scheduleCaches: { ...data.scheduleCaches, edges: [...rows.values()] } } });
    } catch (cause) {
      if (version === generation.current) setSnapshot((previous) => ({ key, data: previous?.key === key ? previous.data : undefined, loading: false, error: cause instanceof Error ? cause : new Error(String(cause)) }));
      throw cause;
    }
  }, [client, key]);
  useEffect(() => { void refetch().catch(() => {}); return () => { generation.current++; }; }, [refetch]);
  useLiveInvalidation(ScheduleWeekDocument, { ...range, country }, refetch, { enabled: !skip && Boolean(country), key: `schedule:${key}`, shared: false });
  const current = snapshot?.key === key ? snapshot : undefined;
  return { data: current?.data, loading: !skip && (!current || current.loading), error: current?.error, refetch };
}

function dayLabel(date: Date, today: Date): { name: string; number: string } {
  const diff = Math.round((date.getTime() - today.getTime()) / 86400_000);
  const name = diff === 0 ? "Today" : diff === 1 ? "Tomorrow" : date.toLocaleDateString(undefined, { weekday: "long" });
  return { name, number: date.toLocaleDateString(undefined, { day: "numeric", month: "short" }) };
}

/**
 * TV guide: a day strip across the top and the day's programmes below, in air-time order.
 * "My shows" comes from library episodes; "All shows" adds the TVmaze schedule cache so new
 * shows can be discovered and added from here.
 */
export function TvGuide() {
  const navigate = useNavigate();
  const [scope, setScope] = useState<Scope>("mine");
  const today = useMemo(() => {
    const date = new Date();
    date.setHours(0, 0, 0, 0);
    return date;
  }, []);
  const days = useMemo(() => Array.from({ length: DAYS }, (_, index) => new Date(today.getTime() + index * 86400_000)), [today]);
  const [selected, setSelected] = useState(() => isoDate(today));
  const range = { from: isoDate(days[0]!), to: isoDate(days[DAYS - 1]!) };

  const library = useQuery(UpcomingLibraryEpisodesDocument, { variables: range });
  const countries = useQuery(ScheduleCountriesDocument, { skip: scope !== "all", errorPolicy: "ignore" });
  const synced = useMemo(() => (countries.data?.scheduleSyncStates.edges ?? []).map((edge) => edge.node.countryCode).filter((item): item is string => Boolean(item)), [countries.data]);
  const [country, setCountry] = useState<string | null>(null);
  useEffect(() => {
    if (countries.loading) return;
    if (country && synced.includes(country)) return;
    const preferred = localeCountry();
    setCountry(synced.length === 0 ? (preferred ?? "US") : preferred && synced.includes(preferred) ? preferred : synced[0]!);
  }, [countries.loading, synced, country]);
  const schedule = useScheduleWeek(range, country, scope !== "all");
  const later = useQuery(NextLibraryEpisodesDocument, { variables: { from: range.to } });
  const laterEpisodes = useMemo(() => later.data?.episodes.edges.map((edge) => edge.node) ?? [], [later.data]);

  const airings = useMemo<Airing[]>(() => {
    const mine: Airing[] = (library.data?.episodes.edges ?? []).map(({ node }) => ({
      key: `lib-${node.id}`,
      date: (node.airDate ?? "").slice(0, 10),
      time: clock(node.airStamp).time,
      sortTime: clock(node.airStamp).sort,
      showName: node.show?.name ?? "Show",
      code: code(node.season, node.episode),
      title: node.title ?? null,
      network: node.show?.network ?? null,
      poster: node.show ? showPoster(node.show) : undefined,
      inLibrary: true,
      showId: node.showId,
      hasFile: Boolean(node.mediaFileId),
    }));
    if (scope === "mine") return mine;
    const mineShows = new Set(mine.map((item) => item.showName.toLowerCase()));
    const seen = new Set<number>();
    const global: Airing[] = (schedule.data?.scheduleCaches.edges ?? [])
      .filter(({ node }) => {
        if (mineShows.has(node.showName.toLowerCase()) || seen.has(node.tvmazeEpisodeId)) return false;
        seen.add(node.tvmazeEpisodeId);
        return true;
      })
      .map(({ node }) => {
        const { time, sort } = clock(node.airStamp);
        return {
          key: `sched-${node.tvmazeEpisodeId}`,
          date: node.airDate.slice(0, 10),
          time: time ?? node.airTime ?? null,
          sortTime: sort,
          showName: node.showName,
          code: code(node.season, node.episodeNumber),
          title: node.episodeName,
          network: node.showNetwork ?? null,
          // Served from the local artwork cache, which downloads the guide poster once.
          poster: node.showPosterUrl ? artworkUrl("guide", node.tvmazeShowId, "poster") : undefined,
          inLibrary: false,
        };
      });
    return [...mine, ...global].sort((a, b) => a.sortTime - b.sortTime || a.showName.localeCompare(b.showName));
  }, [library.data, schedule.data, scope]);

  const counts = useMemo(() => {
    const map = new Map<string, number>();
    for (const airing of airings) map.set(airing.date, (map.get(airing.date) ?? 0) + 1);
    return map;
  }, [airings]);
  const programmes = useMemo(() => airings.filter((airing) => airing.date === selected), [airings, selected]);
  const loading = (library.loading && !library.data) || (scope === "all" && ((schedule.loading && !schedule.data) || (countries.loading && !countries.data)));
  const weekEmpty = !loading && airings.length === 0;

  const open = (airing: Airing) => {
    if (airing.showId) void navigate({ to: "/shows/$showId", params: { showId: airing.showId } });
    else void navigate({ to: "/search", search: { q: airing.showName, in: "show" } as never });
  };

  return (
    <section aria-label="TV guide" className="glass-panel  rounded-card">
      <header className="flex flex-wrap items-center gap-3 px-5 pt-5">
        <div className="min-w-0 flex-1">
          <h2 className="text-title-lg text-foreground">TV guide</h2>
          <p className="text-label-sm text-muted">{scope === "mine" ? "Episodes of shows in your library" : `Everything airing this week${country ? ` · ${country}` : ""}`}</p>
        </div>
        {scope === "all" && synced.length > 1 ? <Segmented aria-label="Country" size="sm" value={country ?? synced[0]!} onValueChange={setCountry} options={synced.map((item) => ({ value: item, label: item }))} className="min-w-0 max-w-full" /> : null}
        <Segmented<Scope> aria-label="Guide scope" size="sm" value={scope} onValueChange={setScope} options={[{ value: "mine", label: "My shows" }, { value: "all", label: "All shows" }]} className="min-w-0 max-w-full" />
      </header>

      <Tabs.Root value={selected} onValueChange={setSelected} className="mt-4 min-w-0">
        <Tabs.List aria-label="Day" className="px-5" items={days.map((day) => {
          const iso = isoDate(day);
          const label = dayLabel(day, today);
          const count = counts.get(iso) ?? 0;
          return { value: iso, label: `${label.name} ${label.number}${count ? ` · ${count}` : ""}` };
        })} />
        <Tabs.Panel value={selected} className="px-5 pb-5 pt-4">
          {loading ? (
            <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
              {Array.from({ length: 3 }, (_, index) => (
                <SkeletonBlock key={index} className="h-20 rounded-xl" />
              ))}
            </div>
          ) : weekEmpty && laterEpisodes.length > 0 ? (
            <p className="py-4 text-center text-body-sm text-muted">{scope === "mine" ? "Nothing from your shows airs this week." : "No listings for this week yet."}</p>
          ) : weekEmpty ? (
            <p className="py-4 text-body-sm text-muted">{scope === "mine" ? "Nothing from your shows airs this week" : "No listings for this week"}</p>
          ) : programmes.length === 0 ? (
            <p className="py-6 text-center text-body-sm text-muted">Nothing on {dayLabel(new Date(`${selected}T00:00:00`), today).name.toLowerCase()}.</p>
          ) : (
            <ol className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
              {programmes.map((airing) => (
                <li key={airing.key}>
                  <ListRow
                    onClick={() => open(airing)}
                    icon={<Artwork src={airing.poster} alt="" aspect="poster" displayWidth={48} tint={LIBRARY_TYPES.tv.tintVar} className="w-12 shrink-0 rounded-md" />}
                    title={<span className="flex items-center gap-1.5">
                      {airing.inLibrary ? <span className={cn("size-1.5 shrink-0 rounded-full", airing.hasFile ? "bg-success" : "bg-warning")} title={airing.hasFile ? "Downloaded" : "In your library"} /> : null}
                      <span>{airing.showName}</span>
                    </span>}
                    meta={<span className="flex min-w-0 flex-col">
                      <span>{airing.code}{airing.title ? ` · ${airing.title}` : ""}</span>
                      <span>{[airing.time ?? "Time to be announced", airing.network].filter(Boolean).join(" · ")}</span>
                    </span>}
                    trailing={!airing.inLibrary ? <IconPlus /> : undefined}
                    className="w-full"
                    data-focusable
                  />
                </li>
              ))}
            </ol>
          )}

          {laterEpisodes.length > 0 ? (
            <div className="mt-5">
              <p className="text-overline mb-2 text-muted">Further ahead</p>
              <ol className="flex flex-wrap gap-2">
                {laterEpisodes.map((episode) => (
                  <li key={episode.id}>
                    <Link to="/shows/$showId" params={{ showId: episode.showId }} data-focusable className="nav-focus glass-control flex items-center gap-2.5 rounded-pill py-1.5 pl-1.5 pr-3.5 text-left hover-capable:hover:brightness-110">
                      <Artwork src={episode.show ? showPoster(episode.show) : undefined} alt="" aspect="square" displayWidth={32} tint={LIBRARY_TYPES.tv.tintVar} className="size-8 rounded-full [&_img]:object-top" />
                      <span className="min-w-0">
                        <span className="block truncate text-label text-foreground">{episode.show?.name ?? "Show"}</span>
                        <span className="block truncate text-label-sm text-muted">
                          {code(episode.season, episode.episode)} · {formatDate(episode.airDate)}
                        </span>
                      </span>
                    </Link>
                  </li>
                ))}
              </ol>
            </div>
          ) : null}
        </Tabs.Panel>
      </Tabs.Root>
    </section>
  );
}
