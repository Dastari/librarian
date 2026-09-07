/**
 * Dashboard data cache with stale-while-revalidate pattern.
 * Uses generated types only; all data is PascalCase.
 */

import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { useRouterState } from "@tanstack/react-router";
import {
  LibrariesDocument,
  LibraryChangedDocument,
  TorrentCompletedDocument,
  DashboardRecentMediaDocument,
  DashboardScheduleCachesDocument,
  type LibrariesQuery,

  type DashboardScheduleCachesQuery,
} from "../lib/graphql/generated/graphql";
import { recentDashboardMedia, type RecentMedia } from "../lib/dashboard";
import { apolloClient, subscriptionStream } from "../lib/graphql/client";

/** Library node type derived from Libraries query */
export type LibraryNode = LibrariesQuery["libraries"]["edges"][0]["node"];

/** ScheduleCache node type derived from DashboardScheduleCaches query */
export type ScheduleCacheNode =
  DashboardScheduleCachesQuery["scheduleCaches"]["edges"][0]["node"];

const CACHE_KEY = "librarian:dashboard_cache:v2";
const CACHE_TTL_MS = 5 * 60 * 1000;
const STALE_TTL_MS = 30 * 60 * 1000;

interface DashboardCache {
  libraries: LibraryNode[];
  recentMedia: RecentMedia[];
  libraryUpcoming: ScheduleCacheNode[];
  globalUpcoming: ScheduleCacheNode[];
  timestamp: number;
  userId: string;
}

interface DashboardData {
  libraries: LibraryNode[];
  recentMedia: RecentMedia[];
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
    recentMedia: [],
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
        librariesResult.data?.libraries.edges.map((e: any) => e.node) ?? [];

      const [libraryUpcomingResult, globalUpcomingResult] = await Promise.all([
        apolloClient.query({
          query: DashboardScheduleCachesDocument,
          variables: {
            where: { airDate: { gte: fromStr, lte: toStr } },
            orderBy: [{ airDate: "ASC" }],
            page: { limit: 50, offset: 0 },
          },
          fetchPolicy: "network-only",
        }),
        apolloClient.query({
          query: DashboardScheduleCachesDocument,
          variables: {
            where: {
              airDate: { gte: fromStr, lte: toStr },
              countryCode: { eq: "US" },
            },
            orderBy: [{ airDate: "ASC" }],
            page: { limit: 50, offset: 0 },
          },
          fetchPolicy: "network-only",
        }),
      ]);

      const libraryUpcoming: ScheduleCacheNode[] =
        libraryUpcomingResult.data?.scheduleCaches.edges.map(
          (e: any) => e.node,
        ) ?? [];
      const globalUpcomingRaw: ScheduleCacheNode[] =
        globalUpcomingResult.data?.scheduleCaches.edges.map(
          (e: any) => e.node,
        ) ?? [];

      const recentResult = await apolloClient.query({
        query: DashboardRecentMediaDocument,
        fetchPolicy: "network-only",
      });

      const seenShows = new Set<number>();
      const filteredGlobalUpcoming = globalUpcomingRaw
        .filter((ep) => {
          if (seenShows.has(ep.tvmazeShowId)) return false;
          seenShows.add(ep.tvmazeShowId);
          return true;
        })
        .slice(0, 12);

      return {
        libraries,
        recentMedia: recentDashboardMedia(recentResult.data),
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
        recentMedia: cached.recentMedia,
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
          if (result.data?.libraryChanged) handleEvent();
        },
      },
    );
    const torrentSub = subscriptionStream<{ torrentCompleted: { id: number } }>(
      TorrentCompletedDocument,
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
