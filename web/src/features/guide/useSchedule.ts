import { useApolloClient } from "@apollo/client/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { ScheduleCountriesDocument, ScheduleWeekDocument, type ScheduleWeekQuery } from "@/graphql/generated/graphql";
import { useLiveEpoch, useLiveInvalidation, useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import type { ScheduleRow } from "./guide-model";

/** The schedule cache is partitioned by country; prefer the browser's region when it has been synced. */
function localeCountry(): string | undefined {
  try {
    return new Intl.Locale(navigator.language).maximize().region ?? undefined;
  } catch {
    return undefined;
  }
}

/** Synced schedule countries and the one to show (the browser's region when synced). */
export function useScheduleCountry(skip = false) {
  const countries = useQuery(ScheduleCountriesDocument, { skip, errorPolicy: "ignore" });
  const synced = useMemo(() => (countries.data?.scheduleSyncStates.edges ?? []).map((edge) => edge.node.countryCode).filter((item): item is string => Boolean(item)), [countries.data]);
  const [country, setCountry] = useState<string | null>(null);
  useEffect(() => {
    if (skip || countries.loading) return;
    if (country && synced.includes(country)) return;
    const preferred = localeCountry();
    setCountry(synced.length === 0 ? (preferred ?? "US") : preferred && synced.includes(preferred) ? preferred : synced[0]!);
  }, [skip, countries.loading, synced, country]);
  return { country, setCountry, synced, loading: !skip && countries.loading && !countries.data };
}

/**
 * Every schedule row for one country in a date range. A week is usually a few hundred rows,
 * so this walks `offset` until `totalCount` (capped at 1,000).
 */
export function useScheduleRange(range: { from: string; to: string }, country: string | null, skip = false) {
  const client = useApolloClient();
  const epoch = useLiveEpoch();
  const key = JSON.stringify([epoch, range.from, range.to, country, skip]);
  const generation = useRef(0);
  const [snapshot, setSnapshot] = useState<{ key: string; rows?: ScheduleRow[]; loading: boolean; error?: Error }>();
  const refetch = useCallback(async () => {
    if (skip || !country) return;
    const version = ++generation.current;
    setSnapshot((previous) => ({ key, rows: previous?.key === key ? previous.rows : undefined, loading: true }));
    try {
      let offset = 0;
      let total = 0;
      const rows = new Map<string, ScheduleRow>();
      do {
        const result = await client.query({ query: ScheduleWeekDocument, variables: { from: range.from, to: range.to, country, offset }, fetchPolicy: "no-cache", context: { queryDeduplication: false } });
        if (version !== generation.current) return;
        const data: ScheduleWeekQuery | undefined = result.data;
        if (!data) throw new Error("Schedule unavailable");
        const edges = data.scheduleCaches.edges;
        total = data.scheduleCaches.pageInfo.totalCount ?? 0;
        if (!edges.length && offset < total) throw new Error("Schedule paging did not advance");
        edges.forEach((edge) => rows.set(edge.node.id, edge.node));
        offset += edges.length;
      } while (offset < total && offset < 1000);
      if (version === generation.current) setSnapshot({ key, loading: false, rows: [...rows.values()] });
    } catch (cause) {
      if (version === generation.current) setSnapshot((previous) => ({ key, rows: previous?.key === key ? previous.rows : undefined, loading: false, error: cause instanceof Error ? cause : new Error(String(cause)) }));
    }
  }, [client, key, range.from, range.to, country, skip]);
  useEffect(() => {
    void refetch();
    return () => { generation.current++; };
  }, [refetch]);
  useLiveInvalidation(ScheduleWeekDocument, { ...range, country }, refetch, { enabled: !skip && Boolean(country), key: `schedule:${key}`, shared: false });
  const current = snapshot?.key === key ? snapshot : undefined;
  return { rows: current?.rows, loading: !skip && Boolean(country) && (!current || current.loading), error: current?.error, refetch };
}
