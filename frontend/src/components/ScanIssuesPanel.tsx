import { useState } from "react";
import { Button } from "@heroui/button";
import { Chip } from "@heroui/chip";
import { useQuery, useSubscription } from "../lib/graphql/client";
import {
  UnresolvedLibraryScanIssuesDocument,
  LibraryScanIssueNotificationChangedDocument,
} from "../lib/graphql/generated/graphql";
import {
  useNotificationOwner,
  useNotificationRefresh,
} from "../hooks/useNotificationFeed";
import {
  ScanIssueDetailModal,
  type ScanIssueNotification,
} from "./ScanIssueDetailModal";
import { parseTimestamp } from "../lib/format";

export function ScanIssuesPanel() {
  const owner = useNotificationOwner();
  const [page, setPage] = useState(0);
  const [selected, setSelected] = useState<ScanIssueNotification | null>(null);
  const query = useQuery(UnresolvedLibraryScanIssuesDocument, {
    variables: {
      where: { ...owner, resolvedAt: { isNull: true } },
      page: { limit: 20, offset: page * 20 },
    },
    fetchPolicy: "cache-and-network",
  });
  const refresh = useNotificationRefresh(() => query.refetch());
  useSubscription(LibraryScanIssueNotificationChangedDocument, {
    fetchPolicy: "no-cache",
    ignoreResults: true,
    onData: refresh,
  });
  const connection = (query.data ?? query.previousData)?.libraryScanIssues;
  const total = connection?.pageInfo.totalCount ?? 0;
  return (
    <section aria-label="Unresolved scan issues" className="space-y-4">
      <p className="text-sm text-default-500">
        Read scan issues remain here until resolved. Marking them as read does
        not change your files.
      </p>
      {query.error && (
        <p role="alert">
          Could not load scan issues.{" "}
          <Button size="sm" onPress={refresh}>
            Retry
          </Button>
        </p>
      )}
      {query.loading && !connection ? (
        <p>Loading scan issues…</p>
      ) : !total ? (
        <p>No unresolved scan issues</p>
      ) : (
        <div className="divide-y divide-divider">
          {connection?.edges.map(({ node: issue }) => (
            <div key={issue.id} className="flex items-start gap-4 py-4">
              <div className="min-w-0 flex-1 space-y-1">
                <div className="flex flex-wrap items-center gap-2">
                  <h2 className="font-semibold">
                    {issue.issueCode.toLowerCase().replaceAll("_", " ")}
                  </h2>
                  <Chip
                    size="sm"
                    variant="flat"
                    color={issue.readAt ? "default" : "warning"}
                  >
                    {issue.readAt ? "Read" : "Unread"}
                  </Chip>
                </div>
                <p className="break-words text-sm text-default-500">
                  {issue.message}
                </p>
                <p className="text-xs text-default-400">
                  {parseTimestamp(issue.createdAt)?.toLocaleString() ??
                    "Unknown date"}
                </p>
              </div>
              <Button
                size="sm"
                variant="flat"
                onPress={() => setSelected(issue)}
              >
                Review scan issue
              </Button>
            </div>
          ))}
        </div>
      )}
      <div className="flex items-center justify-between gap-3">
        <span className="text-sm text-default-500">
          {total ? page * 20 + 1 : 0}–{Math.min((page + 1) * 20, total)} of{" "}
          {total}
        </span>
        <div className="flex gap-2">
          <Button
            size="sm"
            isDisabled={page === 0 || query.loading}
            onPress={() => setPage(page - 1)}
          >
            Previous scan issues
          </Button>
          <Button
            size="sm"
            isDisabled={!connection?.pageInfo.hasNextPage || query.loading}
            onPress={() => setPage(page + 1)}
          >
            Next scan issues
          </Button>
        </div>
      </div>
      <ScanIssueDetailModal
        issue={selected}
        isOpen={Boolean(selected)}
        onClose={() => setSelected(null)}
        onChanged={refresh}
      />
    </section>
  );
}
