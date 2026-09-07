import { useQuery, useSubscription } from "@apollo/client/react";

import {
  ActiveDownloadCountDocument,
  EntityNotificationChangedDocument,
  TorrentAddedDocument,
  TorrentCompletedDocument,
  TorrentRemovedDocument,
  UnreadNotificationCountDocument,
} from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";

/** Live badge counts for the navigation. Refetches on the relevant change events. */
export function useShellCounts() {
  const { status } = useSession();
  const enabled = status === "authenticated";

  const downloads = useQuery(ActiveDownloadCountDocument, { skip: !enabled, pollInterval: 30_000 });
  const notifications = useQuery(UnreadNotificationCountDocument, { skip: !enabled });

  const refetchDownloads = () => void downloads.refetch();
  useSubscription(TorrentAddedDocument, { skip: !enabled, onData: refetchDownloads });
  useSubscription(TorrentCompletedDocument, { skip: !enabled, onData: refetchDownloads });
  useSubscription(TorrentRemovedDocument, { skip: !enabled, onData: refetchDownloads });
  useSubscription(EntityNotificationChangedDocument, { skip: !enabled, onData: () => void notifications.refetch() });

  return {
    downloads: downloads.data?.activeDownloadCount ?? 0,
    notifications: notifications.data?.notifications.pageInfo.totalCount ?? 0,
  };
}
