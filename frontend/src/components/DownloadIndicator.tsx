import { useCallback, useEffect, useRef } from "react";
import { Button } from "@heroui/button";
import { Badge } from "@heroui/badge";
import { Tooltip } from "@heroui/tooltip";
import { Link } from "@tanstack/react-router";
import { IconDownload } from "@tabler/icons-react";
import { ActiveDownloadCountDocument } from "../lib/graphql/generated/graphql";
import { gql, useQuery, useSubscription } from "../lib/graphql/client";
import {
  TORRENT_ADDED_SUBSCRIPTION,
  TORRENT_COMPLETED_SUBSCRIPTION,
  TORRENT_PROGRESS_SUBSCRIPTION,
  TORRENT_REMOVED_SUBSCRIPTION,
} from "../lib/graphql/subscriptions";
import { ErrorBoundary } from "./ErrorBoundary";

const PROGRESS_REFETCH_DEBOUNCE_MS = 500;

function useActiveDownloadCount() {
  const { data, previousData, refetch } = useQuery(ActiveDownloadCountDocument, {
    fetchPolicy: "cache-and-network",
  });

  const refetchDebounceTimerRef = useRef<number | null>(null);

  const scheduleRefetch = useCallback(() => {
    if (refetchDebounceTimerRef.current !== null) {
      window.clearTimeout(refetchDebounceTimerRef.current);
    }
    refetchDebounceTimerRef.current = window.setTimeout(() => {
      refetchDebounceTimerRef.current = null;
      void refetch();
    }, PROGRESS_REFETCH_DEBOUNCE_MS);
  }, [refetch]);

  useEffect(
    () => () => {
      if (refetchDebounceTimerRef.current !== null) {
        window.clearTimeout(refetchDebounceTimerRef.current);
      }
    },
    [],
  );

  useSubscription(gql(TORRENT_PROGRESS_SUBSCRIPTION), {
    onData: () => {
      scheduleRefetch();
    },
  });
  useSubscription(gql(TORRENT_ADDED_SUBSCRIPTION), {
    onData: () => {
      void refetch();
    },
  });
  useSubscription(gql(TORRENT_REMOVED_SUBSCRIPTION), {
    onData: () => {
      void refetch();
    },
  });
  useSubscription(gql(TORRENT_COMPLETED_SUBSCRIPTION), {
    onData: () => {
      void refetch();
    },
  });

  return (
    data?.ActiveDownloadCount ??
    previousData?.ActiveDownloadCount ??
    0
  );
}

function DownloadIndicatorInner() {
  const activeDownloadCount = useActiveDownloadCount();

  return (
    <Tooltip
      content={
        activeDownloadCount > 0
          ? `${activeDownloadCount} active download${activeDownloadCount !== 1 ? "s" : ""}`
          : "No active downloads"
      }
    >
      <Button
        isIconOnly
        variant="light"
        size="sm"
        as={Link}
        to="/downloads"
        aria-label={`${activeDownloadCount} active downloads`}
      >
        <Badge
          content={activeDownloadCount}
          color="primary"
          size="sm"
          isInvisible={activeDownloadCount === 0}
          showOutline={false}
        >
          <IconDownload size={20} className="text-blue-400" />
        </Badge>
      </Button>
    </Tooltip>
  );
}

/** Fallback when DownloadIndicator fails (e.g. GraphQL not ready) */
function DownloadIndicatorFallback() {
  return (
    <Button
      isIconOnly
      variant="light"
      size="sm"
      as={Link}
      to="/downloads"
      aria-label="Downloads"
    >
      <IconDownload size={20} className="text-blue-400" />
    </Button>
  );
}

/**
 * Download icon with active-download badge, links to /downloads.
 * Uses ActiveDownloadCount query + torrent event subscriptions.
 * Only render when the user is authenticated.
 * Wrapped in ErrorBoundary so a failure here does not take down the Navbar.
 */
export function DownloadIndicator() {
  return (
    <ErrorBoundary fallback={<DownloadIndicatorFallback />}>
      <DownloadIndicatorInner />
    </ErrorBoundary>
  );
}
