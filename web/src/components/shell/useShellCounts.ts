import { ActiveDownloadCountDocument, UnreadNotificationCountDocument } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { useLiveQuery } from "@/lib/graphql/live/hooks";

/** Identical watches share one registry task and one listener per topic, even
 * when all three responsive navigation bars remain mounted. */
export function useShellCounts() {
  const { status } = useSession();
  const skip = status !== "authenticated";
  const downloads = useLiveQuery(ActiveDownloadCountDocument, { skip });
  const notifications = useLiveQuery(UnreadNotificationCountDocument, { skip });
  return {
    downloads: downloads.data?.activeDownloadCount ?? 0,
    notifications: notifications.data?.notifications.pageInfo.totalCount ?? 0,
  };
}
