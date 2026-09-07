import { useCallback, useEffect, useRef } from "react";
import { Button } from "@heroui/button";
import { Badge } from "@heroui/badge";
import { Tooltip } from "@heroui/tooltip";
import { Link } from "@tanstack/react-router";
import { IconDownload } from "@tabler/icons-react";
import {
  ActiveDownloadCountDocument,
  TorrentAddedDocument,
  TorrentCompletedDocument,
  TorrentProgressDocument,
  TorrentRemovedDocument,
} from "../lib/graphql/generated/graphql";
import { useQuery, useSubscription } from "../lib/graphql/client";
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

  useSubscription(TorrentProgressDocument, {
    onData: () => {
      scheduleRefetch();
    },
  });
  useSubscription(TorrentAddedDocument, {
    onData: () => {
      void refetch();
    },
  });
  useSubscription(TorrentRemovedDocument, {
    onData: () => {
      void refetch();
    },
  });
  useSubscription(TorrentCompletedDocument, {
    onData: () => {
      void refetch();
    },
  });

  return (
    data?.activeDownloadCount ??
    previousData?.activeDownloadCount ??
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
