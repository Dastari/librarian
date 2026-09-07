import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { IconBan, IconRotate } from "@tabler/icons-react";
import { useMemo } from "react";

import { DataTable, type DataTableColumn, type DataTableRowAction, EmptyState, Panel, StatusChip } from "@/components/ui";
import {
  EntityReleaseBlocklistDeleteDocument,
  EntityReleaseBlocklistListDocument,
  type ReleaseBlocklistFieldsFragment,
} from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatDate, formatRelative } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

/** Human wording for the reasons the backend records. */
const REASON: Record<string, string> = {
  import_failed: "Import failed",
  stalled: "Stalled",
  rejected: "Rejected",
  manual: "Blocked by hand",
};

/**
 * Releases the importer parked so auto-download stops grabbing them again. Removing an entry
 * makes the release eligible on the next search.
 */
export function BlockedReleases() {
  const isAdmin = useIsAdmin();
  const { data, previousData, loading, refetch } = useQuery(EntityReleaseBlocklistListDocument, {
    variables: { orderBy: [{ createdAt: "DESC" }], page: { limit: 100, offset: 0 } },
  });
  const rows = (data ?? previousData)?.releaseBlocklists.edges.map((edge) => edge.node) ?? [];
  const [remove] = useMutation(EntityReleaseBlocklistDeleteDocument);

  const unblock = async (row: ReleaseBlocklistFieldsFragment) => {
    try {
      assertSuccess((await remove({ variables: { id: row.id } })).data?.deleteReleaseBlocklist, "Could not unblock");
      toast.success("Release unblocked");
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const columns = useMemo<Array<DataTableColumn<ReleaseBlocklistFieldsFragment>>>(
    () => [
      {
        id: "title",
        header: "Release",
        cell: (row) => (
          <span className="min-w-0">
            <span className="block truncate text-body-sm text-foreground">{row.title}</span>
            <span className="block truncate font-mono text-label-sm text-muted">{row.infoHash ?? row.guid ?? "—"}</span>
          </span>
        ),
      },
      { id: "reason", header: "Reason", size: 160, cell: (row) => <StatusChip status={{ label: REASON[row.reason] ?? row.reason, tone: "warning", dot: "bg-warning" }} /> },
      { id: "createdAt", header: "Blocked", size: 140, hideBelow: "md", cell: (row) => <span className="text-muted">{formatRelative(row.createdAt)}</span> },
      { id: "expiresAt", header: "Until", size: 140, hideBelow: "lg", cell: (row) => <span className="text-muted">{row.expiresAt ? formatDate(row.expiresAt) : "Permanent"}</span> },
    ],
    [],
  );

  const actions: Array<DataTableRowAction<ReleaseBlocklistFieldsFragment>> = isAdmin
    ? [{ key: "unblock", label: "Unblock", icon: <IconRotate size={16} />, onAction: (row) => void unblock(row) }]
    : [];

  return (
    <Panel title="Blocked releases" description="Releases that failed to import or stalled. They are skipped until you unblock them." flush>
      <DataTable<ReleaseBlocklistFieldsFragment>
        className="px-4 pb-4"
        frame={false}
        columns={columns}
        rows={rows}
        getRowId={(row) => row.id}
        isLoading={loading && rows.length === 0}
        rowActions={actions}
        density="compact"
        noun="releases"
        emptyState={<EmptyState compact icon={IconBan} title="Nothing is blocked" />}
      />
    </Panel>
  );
}
