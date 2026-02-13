/**
 * Dashboard data cache with stale-while-revalidate pattern.
 * Uses generated types only; all data is PascalCase.
 */

import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { useRouterState } from "@tanstack/react-router";
import {
  LibrariesDocument,
  LibraryChangedDocument,
  DashboardShowsDocument,
  DashboardScheduleCachesDocument,
  type LibrariesQuery,
  type DashboardShowsQuery,
  type DashboardScheduleCachesQuery,
} from "../lib/graphql/generated/graphql";
import { apolloClient, subscriptionStream } from "../lib/graphql/client";
import { TORRENT_COMPLETED_SUBSCRIPTION } from "../lib/graphql/subscriptions";

/** Library node type derived from Libraries query */
export type LibraryNode = LibrariesQuery["Libraries"]["Edges"][0]["Node"];

/** Show node type derived from DashboardShows query */
export type ShowNode = DashboardShowsQuery["Shows"]["Edges"][0]["Node"];

/** ScheduleCache node type derived from DashboardScheduleCaches query */
export type ScheduleCacheNode =
  DashboardScheduleCachesQuery["ScheduleCaches"]["Edges"][0]["Node"];

const CACHE_KEY = "librarian:dashboard_cache";
const CACHE_TTL_MS = 5 * 60 * 1000;
const STALE_TTL_MS = 30 * 60 * 1000;

interface DashboardCache {
  libraries: LibraryNode[];
  recentShows: ShowNode[];
  libraryUpcoming: ScheduleCacheNode[];
  globalUpcoming: ScheduleCacheNode[];
  timestamp: number;
  userId: string;
}

interface DashboardData {
  libraries: LibraryNode[];
  recentShows: ShowNode[];
  libraryUpcoming: ScheduleCacheNode[];
  globalUpcoming: ScheduleCacheNode[];
}

interface UseDashboardCacheResult {
  data: DashboardData;
  isLoading: boolean;
  isStale: boolean;
  isFetching: boolean;
  refetch: () => Promise<void>;
}

function readCache(userId: string): DashboardCache | null {
  try {
    const cached = localStorage.getItem(CACHE_KEY);
    if (!cached) return null;
    const parsed: DashboardCache = JSON.parse(cached);
    if (parsed.userId !== userId) return null;
    if (Date.now() - parsed.timestamp > STALE_TTL_MS) return null;
    return parsed;
  } catch {
    return null;
  }
}

function writeCache(data: DashboardData, userId: string): void {
  try {
    localStorage.setItem(
      CACHE_KEY,
      JSON.stringify({ ...data, timestamp: Date.now(), userId }),
    );
  } catch {
    // ignore
  }
}

function isCacheFresh(cache: DashboardCache | null): boolean {
  return cache != null && Date.now() - cache.timestamp < CACHE_TTL_MS;
}

function formatDateForFilter(d: Date): string {
  return d.toISOString().slice(0, 10);
}

export function useDashboardCache(
  userId: string | null,
): UseDashboardCacheResult {
  const [data, setData] = useState<DashboardData>({
    libraries: [],
    recentShows: [],
    libraryUpcoming: [],
    globalUpcoming: [],
  });
  const [isLoading, setIsLoading] = useState(true);
  const [isFetching, setIsFetching] = useState(false);
  const [isStale, setIsStale] = useState(false);
  const fetchInProgress = useRef(false);
  const initialLoadDone = useRef(false);

  const fetchData = useCallback(async (): Promise<DashboardData | null> => {
    if (!userId) return null;
    try {
      const today = new Date();
      const endDate = new Date(today);
      endDate.setDate(endDate.getDate() + 7);
      const fromStr = formatDateForFilter(today);
      const toStr = formatDateForFilter(endDate);

      const librariesResult = await apolloClient.query({
        query: LibrariesDocument,
        fetchPolicy: "network-only",
      });

      const libraries: LibraryNode[] =
        librariesResult.data?.Libraries.Edges.map((e: any) => e.Node) ?? [];

      const [libraryUpcomingResult, globalUpcomingResult] = await Promise.all([
        apolloClient.query({
          query: DashboardScheduleCachesDocument,
          variables: {
            Where: { AirDate: { Gte: fromStr, Lte: toStr } },
            OrderBy: [{ AirDate: "Asc" }],
            Page: { Limit: 50, Offset: 0 },
          },
          fetchPolicy: "network-only",
        }),
        apolloClient.query({
          query: DashboardScheduleCachesDocument,
          variables: {
            Where: {
              AirDate: { Gte: fromStr, Lte: toStr },
              CountryCode: { Eq: "US" },
            },
            OrderBy: [{ AirDate: "Asc" }],
            Page: { Limit: 50, Offset: 0 },
          },
          fetchPolicy: "network-only",
        }),
      ]);

      const libraryUpcoming: ScheduleCacheNode[] =
        libraryUpcomingResult.data?.ScheduleCaches.Edges.map(
          (e: any) => e.Node,
        ) ?? [];
      const globalUpcomingRaw: ScheduleCacheNode[] =
        globalUpcomingResult.data?.ScheduleCaches.Edges.map(
          (e: any) => e.Node,
        ) ?? [];

      const tvLibraries = libraries.filter((lib) => lib.LibraryType === "TV");
      const allShows: ShowNode[] = [];
      for (const library of tvLibraries.slice(0, 2)) {
        const showsResult = await apolloClient.query({
          query: DashboardShowsDocument,
          variables: {
            Where: { LibraryId: { Eq: library.Id } },
            OrderBy: [{ CreatedAt: "Desc" }],
            Page: { Limit: 6, Offset: 0 },
          },
          fetchPolicy: "network-only",
        });
        const nodes = showsResult.data?.Shows.Edges ?? [];
        allShows.push(...nodes.map((e: any) => e.Node));
      }

      const seenShows = new Set<number>();
      const filteredGlobalUpcoming = globalUpcomingRaw
        .filter((ep) => {
          if (seenShows.has(ep.TvmazeShowId)) return false;
          seenShows.add(ep.TvmazeShowId);
          return true;
        })
        .slice(0, 12);

      return {
        libraries,
        recentShows: allShows.slice(0, 6),
        libraryUpcoming,
        globalUpcoming: filteredGlobalUpcoming,
      };
    } catch (err) {
      console.error("Failed to fetch dashboard data:", err);
      return null;
    }
  }, [userId]);

  const refetch = useCallback(async () => {
    if (!userId || fetchInProgress.current) return;
    fetchInProgress.current = true;
    setIsFetching(true);
    const freshData = await fetchData();
    if (freshData) {
      setData(freshData);
      setIsStale(false);
      writeCache(freshData, userId);
    }
    setIsFetching(false);
    setIsLoading(false);
    fetchInProgress.current = false;
  }, [userId, fetchData]);

  useEffect(() => {
    if (!userId) {
      setIsLoading(false);
      return;
    }
    if (initialLoadDone.current) return;
    initialLoadDone.current = true;
    const cached = readCache(userId);
    if (cached) {
      setData({
        libraries: cached.libraries,
        recentShows: cached.recentShows,
        libraryUpcoming: cached.libraryUpcoming,
        globalUpcoming: cached.globalUpcoming,
      });
      setIsStale(!isCacheFresh(cached));
      setIsLoading(false);
      if (!isCacheFresh(cached)) refetch();
    } else {
      refetch();
    }
  }, [userId, refetch]);

  const routerState = useRouterState();
  const isOnDashboard = routerState.location.pathname === "/";

  useEffect(() => {
    if (!userId || !isOnDashboard) return;
    const handleEvent = () => {
      if (document.visibilityState === "visible") refetch();
      else setIsStale(true);
    };
    const librarySub = subscriptionStream(LibraryChangedDocument, {}).subscribe(
      {
        next: (result: any) => {
          if (result.data?.LibraryChanged) handleEvent();
        },
      },
    );
    const torrentSub = subscriptionStream<{ TorrentCompleted: { Id: number } }>(
      TORRENT_COMPLETED_SUBSCRIPTION,
      {},
    ).subscribe({ next: handleEvent });
    return () => {
      librarySub.unsubscribe();
      torrentSub.unsubscribe();
    };
  }, [userId, refetch, isOnDashboard]);

  return useMemo(
    () => ({ data, isLoading, isStale, isFetching, refetch }),
    [data, isLoading, isStale, isFetching, refetch],
  );
}

export function clearDashboardCache(): void {
  try {
    localStorage.removeItem(CACHE_KEY);
  } catch {
    // ignore
  }
}
