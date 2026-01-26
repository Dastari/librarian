import { useState, useEffect, useCallback } from "react";
import { Button } from "@heroui/button";
import { Badge } from "@heroui/badge";
import { Tooltip } from "@heroui/tooltip";
import { Link } from "@tanstack/react-router";
import { IconDownload } from "@tabler/icons-react";
import { graphqlClient } from "../lib/graphql";
import {
  ActiveDownloadCountDocument,
  TorrentChangedDocument,
} from "../lib/graphql/generated/graphql";
import { ErrorBoundary } from "./ErrorBoundary";

const DOWNLOADING_WHERE = { State: { Eq: "downloading" } } as const;
const PAGE_ONE = { Limit: 1, Offset: 0 } as const;

function useActiveDownloadCount() {
  const [count, setCount] = useState(0);

  const fetchCount = useCallback(async () => {
    try {
      const { data } = await graphqlClient
        .query(ActiveDownloadCountDocument, {
          Where: DOWNLOADING_WHERE,
          Page: PAGE_ONE,
        })
        .toPromise();
      const total = data?.Torrents?.PageInfo?.TotalCount;
      setCount(total ?? 0);
    } catch {
      setCount(0);
    }
  }, []);

  useEffect(() => {
    fetchCount();
  }, [fetchCount]);

  useEffect(() => {
    let sub: { unsubscribe: () => void } | null = null;
    try {
      sub = graphqlClient
        .subscription(TorrentChangedDocument, {})
        .subscribe({
          next: () => fetchCount(),
          error: () => {},
        });
    } catch {
      // subscription setup failed (e.g. client not ready)
    }
    return () => sub?.unsubscribe?.();
  }, [fetchCount]);

  return count;
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
 * Uses ActiveDownloadCount query + TorrentChanged subscription.
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
