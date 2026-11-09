import { Dialog as GlassDialog } from "glass-ux/dialog";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell, TextCell } from "glass-ux/data-table";
import { Segmented } from "glass-ux/segmented";
import { Tooltip } from "glass-ux/tooltip";
import { useMemo, useState } from "react";

import { useDialogReturnFocus } from "@/components/ui/Dialog";
import { EntityRssFeedItemListDocument, type EntityRssFeedItemListQuery } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { formatBytes, formatRelative } from "@/lib/format";

import { SettingsLoadError } from "./SettingsLoadError";
import { useSettingsConnection } from "./useSettingsConnection";

type Item = EntityRssFeedItemListQuery["rssFeedItems"]["edges"][number]["node"];
type Show = "all" | "matched" | "unmatched";

const ItemGlyph = ENTITY_KINDS.torrent.icon;

/** What the title says it is, short: `1080p · bluray · hevc · GRP`. */
export function itemQuality(item: Pick<Item, "parsedResolution" | "parsedSource" | "parsedCodec" | "parsedHdr" | "parsedGroup" | "parsedFormat" | "parsedBitrate">): string {
  return [item.parsedResolution, item.parsedSource, item.parsedCodec, item.parsedHdr, item.parsedFormat, item.parsedBitrate, item.parsedGroup].filter(Boolean).join(" · ");
}

const column = createDataTableColumns<Item>();
const columns = column.columns([
  column.accessor("title", {
    header: "Release", width: 34, priority: 1, cardRole: "title",
    cell: ({ row }) => <IdentityCell icon={<ItemGlyph />} title={row.original.title} subtitle={[row.original.category, row.original.parsedScope?.replace(/_/g, " ")].filter(Boolean).join(" · ")} />,
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
  column.accessor("seenAt", { header: "Seen", width: 8, priority: 2, grow: false, cell: ({ row }) => <TextCell muted>{formatRelative(row.original.pubDate ?? row.original.seenAt)}</TextCell> }),
]);

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
  const list = useSettingsConnection(EntityRssFeedItemListDocument, { where, orderBy: [{ seenAt: "DESC" }] }, (data) => data.rssFeedItems, { infinite: true, batchSize: 100, skip: !isOpen });
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
            toolbar={
              <div className="flex min-w-0 flex-wrap items-center justify-end gap-2">
                <Segmented<Show> aria-label="Show" size="sm" value={show} onValueChange={setShow} options={[{ value: "all", label: "All" }, { value: "matched", label: "In my library" }, { value: "unmatched", label: "Not in my library" }]} />
                {list.totalCount !== undefined ? <span className="text-numeric whitespace-nowrap text-label-sm text-muted">{list.totalCount} listed</span> : null}
              </div>
            }
            infinite={{ revision: list.revision, hasMore: list.hasMore, onLoadMore: list.loadMore, loadingMore: list.loadingMore, error: list.moreError?.message }}
            noun={{ one: "release", other: "releases" }}
            emptyMessage="Nothing listed yet. Poll the feed to read it now."
          />
        </GlassDialog.Body>
      </GlassDialog.Popup>
    </GlassDialog.Root>
  );
}
