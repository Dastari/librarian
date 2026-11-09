import { Dialog as GlassDialog } from "glass-ux/dialog";
import { Badge } from "glass-ux/badge";
import { defineActions } from "glass-ux/actions";
import { copyText } from "glass-ux/copy-button";
import { createDataTableColumns, DataTable, TextCell, type DataTableState } from "glass-ux/data-table";
import { IconCopy } from "glass-ux/icons";
import { Segmented } from "glass-ux/segmented";
import { Tooltip } from "glass-ux/tooltip";
import { useMemo, useState } from "react";

import { useDialogReturnFocus } from "@/components/ui/Dialog";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { useTableSelection } from "@/hooks/useTableSelection";
import { EntityRssFeedItemListDocument, type EntityRssFeedItemListQuery } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { formatBytes, formatRelative } from "@/lib/format";

import { ReleaseIdentity } from "./ReleaseIdentity";
import { SettingsLoadError } from "./SettingsLoadError";
import { useSettingsConnection } from "./useSettingsConnection";

type Item = EntityRssFeedItemListQuery["rssFeedItems"]["edges"][number]["node"];
type Show = "all" | "matched" | "unmatched";

/** What the title says it is, short: `1080p · bluray · hevc · GRP`. */
export function itemQuality(item: Pick<Item, "parsedResolution" | "parsedSource" | "parsedCodec" | "parsedHdr" | "parsedGroup" | "parsedFormat" | "parsedBitrate">): string {
  return [item.parsedResolution, item.parsedSource, item.parsedCodec, item.parsedHdr, item.parsedFormat, item.parsedBitrate, item.parsedGroup].filter(Boolean).join(" · ");
}

const column = createDataTableColumns<Item>();
const columns = column.columns([
  column.accessor("title", {
    header: "Release", width: 34, priority: 1, cardRole: "title",
    cell: ({ row }) => <ReleaseIdentity title={row.original.title} facts={[row.original.category, row.original.parsedScope?.replace(/_/g, " ")].filter(Boolean).join(" · ")} />,
  }),
  column.display({ id: "quality", header: "Quality", width: 16, priority: 3, cell: ({ row }) => <TextCell muted>{itemQuality(row.original) || "—"}</TextCell> }),
  column.accessor((item) => item.size ?? 0, { id: "size", header: "Size", width: 7, priority: 2, align: "end", grow: false, cell: ({ row }) => <TextCell muted>{row.original.size ? formatBytes(row.original.size) : "—"}</TextCell> }),
  column.accessor((item) => item.matchConfidence ?? -1, {
    id: "match", header: "In your library", width: 26, priority: 1, cardRole: "status",
    cell: ({ row }) => {
      const item = row.original;
      if (!item.matchKind) {
        return <Tooltip content={item.matchReason ?? "Nothing in the library matches"}><span tabIndex={0}><TextCell muted>Not in your library</TextCell></span></Tooltip>;
      }
      return (
        <Tooltip content={item.matchReason ?? ""}>
          <span tabIndex={0} className="flex min-w-0 items-center gap-2">
            <span className="min-w-0 truncate text-body-sm text-foreground">{item.matchLabel}</span>
            <Badge size="sm" tone={(item.matchConfidence ?? 0) >= 90 ? "success" : "warning"}>{item.matchConfidence}%</Badge>
          </span>
        </Tooltip>
      );
    },
  }),
  column.accessor("seenAt", { header: "Seen", width: 11, priority: 2, grow: false, cell: ({ row }) => <TextCell muted>{formatRelative(row.original.pubDate ?? row.original.seenAt)}</TextCell> }),
]);

/** What can be done with a listed release: take its title elsewhere. */
export const feedItemActions = defineActions<Item>("feed-item", [
  { id: "copy-titles", label: (targets) => (targets.length === 1 ? "Copy release title" : "Copy release titles"), icon: IconCopy, group: "open", bulk: true,
    run: (targets) => copyText(targets.map((target) => target.title).join("\n")), successMessage: () => "Release titles copied" },
]);

const initialView = (): DataTableState => ({ pagination: { pageIndex: 0, pageSize: 50 }, sorting: [], filters: [], search: "", selection: {} });
const NOUN = { one: "release", other: "releases" };

interface FeedItemsDialogProps {
  feedId: string | null;
  feedName: string;
  onOpenChange: (open: boolean) => void;
}

/**
 * A feed's catalogue: the releases it has listed, newest first, with the title in the library each
 * one is. Served in batches by the server; filters narrow it there.
 */
export function FeedItemsDialog({ feedId, feedName, onOpenChange }: FeedItemsDialogProps) {
  const isOpen = feedId !== null;
  const finalFocus = useDialogReturnFocus(isOpen);
  const { user } = useSession();
  const [show, setShow] = useState<Show>("all");
  const where = useMemo(
    () => ({ feedId: { eq: feedId ?? "" }, ...(show === "matched" ? { matchKind: { isNull: false } } : show === "unmatched" ? { matchKind: { isNull: true } } : {}) }),
    [feedId, show],
  );
  const [view, setView] = useState<DataTableState>(initialView);
  const list = useSettingsConnection(EntityRssFeedItemListDocument, { where, orderBy: [{ seenAt: "DESC" }] }, (data) => data.rssFeedItems, { infinite: true, batchSize: 100, skip: !isOpen });
  const selection = useTableSelection({ scope: [user?.id, feedId, show], rows: list.rows, ready: !list.loading });
  const state = useMemo(() => ({ ...view, selection: selection.selection }), [view, selection.selection]);
  return (
    <GlassDialog.Root open={isOpen} onOpenChange={onOpenChange}>
      <GlassDialog.Popup size="full" finalFocus={finalFocus} className="min-h-0 overflow-hidden">
        <GlassDialog.Header title={`${feedName}: what it has listed`} />
        <GlassDialog.Body className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden">
          <SettingsLoadError error={list.error} retry={list.refetch} hasRows={list.rows.length > 0} />
          <DataTable<Item>
            id={`librarian.${user?.id ?? "session"}.settings.feed-items`}
            label={`${feedName} releases`}
            columns={columns}
            rows={list.rows}
            queryKey={[user?.id, feedId, show]}
            getRowId={(item) => item.id}
            getRowLabel={(item) => item.title}
            loading={list.loading}
            state={state}
            onStateChange={(next, change) => (change === "selection" ? selection.onSelectionChange(next.selection) : setView(next))}
            selectable
            actions={feedItemActions}
            renderBulkActions={(rows) => <TableBulkActions set={feedItemActions} rows={rows} noun={NOUN} />}
            toolbar={
              <div className="flex min-w-0 flex-wrap items-center justify-end gap-2">
                <Segmented<Show> aria-label="Show" size="sm" value={show} onValueChange={setShow} options={[{ value: "all", label: "All" }, { value: "matched", label: "In my library" }, { value: "unmatched", label: "Not in my library" }]} />
                {list.totalCount !== undefined ? <span className="text-numeric whitespace-nowrap text-label-sm text-muted">{list.totalCount} listed</span> : null}
              </div>
            }
            infinite={{ revision: list.revision, hasMore: list.hasMore, onLoadMore: list.loadMore, loadingMore: list.loadingMore, error: list.moreError?.message }}
            noun={NOUN}
            emptyMessage="Nothing listed yet. Poll the feed to read it now."
          />
        </GlassDialog.Body>
      </GlassDialog.Popup>
    </GlassDialog.Root>
  );
}
