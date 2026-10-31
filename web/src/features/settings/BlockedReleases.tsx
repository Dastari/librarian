import { useMutation } from "@apollo/client/react";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, filters, IdentityCell, RelativeTime } from "glass-ux/data-table";
import { IconExternalLink } from "glass-ux/icons";
import { IconBan, IconRotate } from "@tabler/icons-react";
import { useMemo } from "react";

import {
  EntityReleaseBlocklistDeleteDocument,
  EntityReleaseBlocklistListDocument,
  type ReleaseBlocklistFieldsFragment,
} from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { entityPath, glyph, type EntityKindName } from "@/lib/entities/kinds";
import { parseTimestamp } from "@/lib/format";
import { assertSuccess } from "@/lib/graphql/errors";

import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsSection } from "./SettingsSection";
import { useSettingsConnection } from "./useSettingsConnection";

type Row = ReleaseBlocklistFieldsFragment;

/** Human wording for the reasons the backend records. */
const REASON: Record<string, string> = {
  import_failed: "Import failed",
  stalled: "Stalled",
  rejected: "Rejected",
  manual: "Blocked by hand",
};
const BlockGlyph = glyph(IconBan, "ban");
const UnblockIcon = glyph(IconRotate, "rotate");

/** What the release was for, as a kind and id for its page. */
function blockedFor(row: Row): { kind: EntityKindName; id: string; noun: string } | null {
  if (row.movieId) return { kind: "movie", id: row.movieId, noun: "movie" };
  if (row.showId) return { kind: "show", id: row.showId, noun: "show" };
  if (row.albumId) return { kind: "album", id: row.albumId, noun: "album" };
  if (row.audiobookId) return { kind: "audiobook", id: row.audiobookId, noun: "audiobook" };
  return null;
}

const column = createDataTableColumns<Row>();
const columns = column.columns([
  column.accessor("title", {
    header: "Release", width: 30, priority: 1, cardRole: "title", sortable: true,
    cell: ({ row }) => <IdentityCell icon={<BlockGlyph />} title={row.original.title} subtitle={row.original.infoHash ?? row.original.guid ?? undefined} />,
  }),
  column.accessor("reason", {
    header: "Reason", width: 10, priority: 2, cardRole: "status", sortable: true,
    filter: filters.options({ field: "reason", nullable: false, operators: ["in"], options: Object.entries(REASON).map(([value, label]) => ({ value, label })) }),
    cell: ({ row }) => <Badge size="sm" marker="dot" tone={row.original.reason === "manual" ? "neutral" : "warning"}>{REASON[row.original.reason] ?? row.original.reason}</Badge>,
  }),
  column.accessor("createdAt", { header: "Blocked", width: 9, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.createdAt)} /> }),
  column.accessor((row) => row.expiresAt ?? "", { id: "expiresAt", header: "Until", width: 9, priority: 4, sortable: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.expiresAt)} fallback="Permanent" /> }),
]);

/**
 * Releases the importer parked so auto-download stops grabbing them again. Removing an entry
 * makes the release eligible on the next search.
 */
export function BlockedReleases() {
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const list = useSettingsConnection(EntityReleaseBlocklistListDocument, { orderBy: [{ createdAt: "DESC" }] }, (data) => data.releaseBlocklists);
  const { rows, loading, refetch } = list;
  const [remove] = useMutation(EntityReleaseBlocklistDeleteDocument);

  const actions = useMemo(() => defineActions<Row>("blocked-release", [
    {
      id: "open-item", label: (targets) => `Open ${blockedFor(targets[0]!)?.noun ?? "item"}`, icon: IconExternalLink, group: "open", priority: 2,
      visible: (targets) => targets.length === 1 && Boolean(blockedFor(targets[0]!)),
      run: (targets, context) => { const item = blockedFor(targets[0]!); const href = item ? entityPath(item.kind, item.id) : undefined; if (href) context.navigate(href); },
    },
    ...(isAdmin ? [{
      id: "unblock", label: (targets: readonly Row[]) => (targets.length === 1 ? "Unblock" : `Unblock ${targets.length}`), icon: UnblockIcon, group: "manage" as const, priority: 1, bulk: true,
      run: async (targets: readonly Row[]) => {
        for (const row of targets) assertSuccess((await remove({ variables: { id: row.id } })).data?.deleteReleaseBlocklist, "Could not unblock");
        void refetch();
      },
      successMessage: (targets: readonly Row[]) => (targets.length === 1 ? "Release unblocked" : `Unblocked ${targets.length} releases`),
    }] : []),
  ]), [isAdmin, refetch, remove]);

  return (
    <SettingsSection title="Blocked releases" description="Releases that failed to import or stalled. They are skipped until you unblock them.">
      <SettingsLoadError error={list.error} retry={list.refetch} hasRows={rows.length > 0} />
      <DataTable<Row> height="auto"
        id={`librarian.${user?.id ?? "session"}.settings.blocked-releases`}
        label="Blocked releases"
        columns={columns}
        rows={rows}
        getRowId={(row) => row.id}
        getRowLabel={(row) => row.title}
        loading={loading && rows.length === 0}
        actions={actions}
        selectable={isAdmin}
        density="compact"
        noun={{ one: "release", other: "releases" }}
        emptyMessage="Nothing is blocked."
      />
    </SettingsSection>
  );
}
