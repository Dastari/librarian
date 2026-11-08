import { defineActions, type ActionDefinition, type ActionSet } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import { createDataTableColumns, DataTable, TextCell } from "glass-ux/data-table";
import { IconArrowDown, IconArrowUp, IconPlay, IconSave, IconSkipForward, IconTrash } from "glass-ux/icons";
import { createContext, useContext, useMemo, useState, type RefObject } from "react";

import { Artwork } from "@/components/ui";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { PlayingBars } from "@/features/media/RowPlayAction";
import { playItemTargets } from "@/features/personal/playlistPlayback";
import { playlistTitle, useAddToPlaylist } from "@/features/personal/useAddToPlaylist";
import { useSession } from "@/lib/auth/useSession";
import { formatClock } from "@/lib/format";
import { useSelectableTableState } from "@/hooks/useTableSelection";

import { SaveQueueDialog } from "./SaveQueueDialog";
import { playerStore, type PlayerState, type PlayItem } from "./store";

/** One entry of the queue as a table row: its own id, so the same media queued twice is two rows. */
interface QueueRow {
  id: string;
  item: PlayItem;
  index: number;
}

const QueueContext = createContext<{ currentId: string | undefined; playing: boolean } | null>(null);

function useQueueContext() {
  const context = useContext(QueueContext);
  if (!context) throw new Error("Queue cells require QueueContext");
  return context;
}

function EntryCell({ row: { original: row } }: { row: { original: QueueRow } }) {
  const { currentId, playing } = useQueueContext();
  const current = row.id === currentId;
  return (
    <div className="flex min-w-0 items-center gap-3">
      <Artwork src={row.item.artwork} alt="" aspect="square" className="size-9 shrink-0 rounded-md" />
      <div className="min-w-0">
        <p className={`truncate text-body-sm ${current ? "text-brand" : "text-foreground"}`}>{row.item.title}</p>
        {row.item.subtitle ? <p className="truncate text-label-sm text-muted">{row.item.subtitle}</p> : null}
      </div>
      {current && playing ? <PlayingBars /> : null}
    </div>
  );
}

function LengthCell({ row: { original: row } }: { row: { original: QueueRow } }) {
  return row.item.duration ? <TextCell identifier>{formatClock(row.item.duration)}</TextCell> : <TextCell muted>—</TextCell>;
}

// flexRender treats cell functions as component types; context keeps the live values reactive.
const column = createDataTableColumns<QueueRow>();
const columns = column.columns([
  column.display({ id: "entry", header: "Title", cardRole: "title", priority: 1, width: 24, minWidth: 10, hideable: false, cell: EntryCell }),
  column.display({ id: "length", header: "Length", priority: 2, width: 6, minWidth: 5, align: "end", grow: false, cell: LengthCell }),
]);

const getRowId = (row: QueueRow) => row.id;
const getRowLabel = (row: QueueRow) => row.item.title;
const noop = () => undefined;

/** The queue's own actions: what can be done to its entries from a row's menu, right-click or the bulk bar. */
function queueActions(state: PlayerState, addToPlaylist: ActionDefinition<QueueRow>): ActionSet<QueueRow> {
  const atCurrent = (rows: readonly QueueRow[]) => rows.length === 1 && rows[0]!.index === state.index;
  return defineActions<QueueRow>("queue-entry", [
    { id: "play", label: "Play", icon: IconPlay, group: "open", priority: 1, bulk: true, visible: (rows) => rows.length === 1 && !atCurrent(rows), run: ([row]) => playerStore.jumpTo(row!.index) },
    { id: "play-next", label: "Play next", icon: IconSkipForward, group: "organise", bulk: true, visible: (rows) => rows.length === 1 && !atCurrent(rows) && rows[0]!.index !== state.index + 1, run: ([row]) => playerStore.move(row!.index, row!.index > state.index ? state.index + 1 : state.index) },
    { id: "up", label: "Move up", icon: IconArrowUp, group: "organise", bulk: true, visible: (rows) => rows.length === 1 && rows[0]!.index > 0, run: ([row]) => playerStore.move(row!.index, row!.index - 1) },
    { id: "down", label: "Move down", icon: IconArrowDown, group: "organise", bulk: true, visible: (rows) => rows.length === 1 && rows[0]!.index < state.queue.length - 1, run: ([row]) => playerStore.move(row!.index, row!.index + 1) },
    addToPlaylist,
    {
      id: "remove", label: (rows) => (rows.length === 1 ? "Remove from queue" : `Remove ${rows.length} from queue`), icon: IconTrash, group: "danger", bulk: true,
      // Highest index first, so the earlier removals do not move the later ones.
      run: (rows) => { for (const row of [...rows].sort((a, b) => b.index - a.index)) playerStore.removeAt(row.index); },
    },
  ]);
}

/**
 * The queue as an editable list: drag a row by its grip (or Space to lift it, the arrow keys to
 * move it and Space to drop it) to put it where it should play, and a row's menu has Play, Play
 * next, Move up and down and Remove. Save as playlist and Clear are for the whole queue.
 */
export function QueuePanel({ state, listRef, fill }: { state: PlayerState; listRef: RefObject<HTMLDivElement | null>; fill: boolean }) {
  const { user } = useSession();
  const [saving, setSaving] = useState(false);
  const rows = useMemo<QueueRow[]>(() => state.queue.map((item, index) => ({ id: item.queueId ?? String(index), item, index })), [state.queue]);
  const table = useSelectableTableState({ scope: [user?.id], rows, getRowId });
  const playlist = useAddToPlaylist<QueueRow>({
    targetsOf: (selected) => playItemTargets(selected.map((row) => row.item)),
    titleOf: (selected) => playlistTitle(selected, selected[0]!.item.title, { one: "item", other: "items" }),
    bulk: true,
    priority: 5,
  });
  const actions = useMemo(() => queueActions(state, playlist.action), [state, playlist.action]);
  const context = useMemo(() => ({ currentId: state.queue[state.index]?.queueId, playing: state.transport.playing }), [state.queue, state.index, state.transport.playing]);
  const upcoming = state.queue.length - state.index - 1;

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      <div className="flex shrink-0 items-center justify-between gap-2">
        <p className="text-title-md text-foreground">Up next</p>
        <div className="flex items-center gap-1">
          <Button size="sm" variant="ghost" icon={<IconSave />} onClick={() => setSaving(true)} data-focusable>Save as playlist</Button>
          <Button size="sm" variant="ghost" disabled={upcoming <= 0} onClick={() => playerStore.clearUpcoming()} data-focusable>Clear up next</Button>
        </div>
      </div>
      <div ref={listRef} className="min-h-0 flex-1">
        <QueueContext.Provider value={context}>
          <DataTable<QueueRow>
            id="librarian.queue"
            label="Play queue"
            queryKey="queue"
            columns={columns}
            rows={table.rows}
            getRowId={getRowId}
            getRowLabel={getRowLabel}
            height={fill ? "fill" : "auto"}
            density="compact"
            view="table"
            selectable
            state={table.state}
            onStateChange={table.onStateChange}
            actions={actions}
            renderBulkActions={(selected) => <TableBulkActions set={actions} rows={selected} noun={{ one: "item", other: "items" }} />}
            onRowOpen={(row) => playerStore.jumpTo(row.index)}
            rowOpen="click"
            infinite={{ hasMore: false, onLoadMore: noop }}
            rowCount={rows.length}
            noun={{ one: "item", other: "items" }}
            reorder={{ onReorder: (move) => playerStore.move(move.from, move.to) }}
            emptyMessage="The queue is empty"
          />
        </QueueContext.Provider>
      </div>
      <SaveQueueDialog open={saving} onOpenChange={setSaving} />
      {playlist.dialog}
    </div>
  );
}
