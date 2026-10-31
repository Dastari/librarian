import { useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { useRef, useState } from "react";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell } from "glass-ux/data-table";
import { IconArrowDown, IconArrowUp, IconBookmark, IconExternalLink, IconPlay, IconTrash } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { Form, FormActions, FormStack, formBehaviour, useAppForm } from "glass-ux/form";
import { z } from "zod";
import { Artwork, Button, ConfirmDialog, Dialog, EmptyState, ErrorState, PageHeader } from "@/components/ui";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { usePlayer } from "@/features/player/usePlayer";
import { EntityListEntryDeleteDocument, EntityListEntryUpdateDocument, EntityPersonalListDeleteDocument, EntityPersonalListGetDocument, EntityPersonalListUpdateDocument, EntitySavedViewGetDocument, PersonalEntriesDocument, type PersonalEntryFragment } from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery } from "@/lib/graphql/live/hooks";
import { ListEditor } from "./ListEditor";
import { SmartResults } from "./SmartResults";
import { entryPresentation } from "./types";
import { useHomeShelves } from "./usePersonalLibrary";

type Entry = PersonalEntryFragment;
const entryKind = (entry: Entry) => (entry.movie ? "Movie" : entry.show ? "Show" : entry.album ? "Album" : entry.audiobook ? "Audiobook" : entry.track ? "Track" : "Unavailable");
const column = createDataTableColumns<Entry>();
const columns = column.columns([
  column.accessor((entry) => entryPresentation(entry)?.title ?? "Unavailable item", {
    id: "title", header: "Title", width: 28, priority: 1, cardRole: "title",
    cell: ({ row }) => {
      const item = entryPresentation(row.original);
      return <IdentityCell icon={item?.image ? <Artwork src={item.image} alt="" aspect={item.square ? "square" : "poster"} className={item.square ? "w-8 rounded-sm" : "w-6 rounded-sm"} /> : undefined} title={item?.title ?? "Unavailable item"} subtitle={item?.subtitle} />;
    },
  }),
  column.accessor(entryKind, { id: "kind", header: "Type", width: 8, priority: 2, cell: ({ row }) => <Badge size="sm" tone="neutral">{entryKind(row.original)}</Badge> }),
]);

export function PersonalListPage({ listId }: { listId: string }) {
  const navigate = useNavigate();
  const player = usePlayer();
  const list = useLiveQuery(EntityPersonalListGetDocument, { variables: { id: listId } });
  const candidate = list.data?.personalList;
  const current = candidate?.id === listId ? candidate : null;
  const saved = useLiveQuery(EntitySavedViewGetDocument, { variables: { id: current?.savedViewId ?? "" }, skip: !current?.savedViewId });
  const entries = useCompleteConnection(PersonalEntriesDocument, { listId }, (data) => data.listEntries, { skip: current?.kind === "SMART" });
  const ready = useRef({ listId, complete: false });
  if (ready.current.listId !== listId) ready.current = { listId, complete: false };
  if (entries.complete) ready.current.complete = true;
  const home = useHomeShelves();
  const [deleteList, { loading: deleting }] = useMutation(EntityPersonalListDeleteDocument);
  const [remove] = useMutation(EntityListEntryDeleteDocument);
  const [move] = useMutation(EntityListEntryUpdateDocument);
  const [confirm, setConfirm] = useState(false);
  const [editing, setEditing] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [pending, setPending] = useState(false);
  const run = async (action: () => Promise<unknown>) => { setPending(true); try { await action(); } catch (error) { toast.danger(errorMessage(error)); } finally { setPending(false); } };
  const reorder = async (entry: PersonalEntryFragment, delta: number) => {
    const index = entries.rows.findIndex((row) => row.id === entry.id);
    const target = index + delta;
    if (target < 0 || target >= entries.rows.length) return;
    const ordered = [...entries.rows]; ordered.splice(index, 1); ordered.splice(target, 0, entry);
    // Sparse positions keep the common move to one generated write. Rebalance only if needed.
    const before = ordered[target - 1]?.position;
    const after = ordered[target + 1]?.position;
    const position = before === undefined ? (after ?? 0) - 1024 : after === undefined ? before + 1024 : Math.floor((before + after) / 2);
    const updates = position < 0 || (before !== undefined && after !== undefined && after - before < 2) ? ordered.map((row, i) => ({ id: row.id, position: i * 1024 })) : [{ id: entry.id, position }];
    for (const update of updates) { const { data } = await move({ variables: { id: update.id, input: { position: update.position } } }); assertSuccess(data?.updateListEntry, "Could not reorder list"); }
    await entries.refetch();
  };
  const reorderTo = async (entry: Entry, target: number) => {
    const index = entries.rows.findIndex((row) => row.id === entry.id);
    if (index < 0 || target === index) return;
    await reorder(entry, target - index);
  };
  const actions = defineActions<Entry>("list-entry", [
    { id: "open", label: (rows) => `Open ${entryKind(rows[0]!).toLowerCase()}`, icon: IconExternalLink, group: "open", priority: 2, visible: (rows) => rows.length === 1 && Boolean(entryPresentation(rows[0]!)?.href), run: (rows, context) => { const href = entryPresentation(rows[0]!)?.href; if (href) context.navigate(href); } },
    {
      id: "play", label: "Play", icon: IconPlay, group: "open", priority: 1, visible: (rows) => rows.length === 1 && Boolean(rows[0]!.track?.mediaFileId),
      run: (rows) => { const track = rows[0]!.track!; player.playAudio([{ mediaFileId: track.mediaFileId!, title: track.title, subtitle: track.artistName ?? undefined, artwork: entryPresentation(rows[0]!)?.image, href: `/albums/${track.albumId}`, entity: { kind: "track", id: track.id }, duration: track.durationSecs ?? undefined }]); },
    },
    { id: "up", label: "Move up", icon: IconArrowUp, group: "organise", visible: (rows) => rows.length === 1 && entries.rows[0]?.id !== rows[0]!.id, disabledReason: () => (pending ? "Saving list" : !entries.complete ? "Load the complete list to reorder" : false), run: (rows) => run(() => reorder(rows[0]!, -1)) },
    { id: "down", label: "Move down", icon: IconArrowDown, group: "organise", visible: (rows) => rows.length === 1 && entries.rows.at(-1)?.id !== rows[0]!.id, disabledReason: () => (pending ? "Saving list" : !entries.complete ? "Load the complete list to reorder" : false), run: (rows) => run(() => reorder(rows[0]!, 1)) },
    {
      id: "remove", label: (rows) => (rows.length === 1 ? "Remove from list" : `Remove ${rows.length} from list`), icon: IconTrash, group: "danger", bulk: true,
      disabledReason: () => (pending ? "Saving list" : false),
      run: async (rows) => { for (const entry of rows) { const { data } = await remove({ variables: { id: entry.id } }); assertSuccess(data?.deleteListEntry, "Could not remove item"); } await entries.refetch(); },
      successMessage: (rows) => (rows.length === 1 ? "Removed from the list" : `Removed ${rows.length} items`),
    },
  ]);
  if (list.error && !current) return <ErrorState error={list.error} onRetry={() => void list.refetch()} className="m-6" />;
  if (!current) return <div className="page-gutter py-6">{list.loading ? <p role="status">Loading list…</p> : <EmptyState icon={IconBookmark} title="List not found" />}</div>;
  const pinned = home.rows.find((shelf) => shelf.listId === listId && shelf.visible);
  const audio = entries.rows.flatMap((entry) => entry.track?.mediaFileId ? [{ mediaFileId: entry.track.mediaFileId, title: entry.track.title, subtitle: entry.track.artistName ?? undefined, artwork: entryPresentation(entry)?.image, href: `/albums/${entry.track.albumId}`, entity: { kind: "track" as const, id: entry.track.id }, duration: entry.track.durationSecs ?? undefined }] : []);
  return <div data-print-flow className="page-gutter flex min-h-0 min-w-0 flex-1 flex-col gap-5 overflow-hidden py-6">
    <PageHeader title={current.name} className="shrink-0" actions={<div className="flex flex-wrap gap-2">
      {audio.length > 0 ? <Button variant="primary" isDisabled={!entries.complete} onPress={() => player.playAudio(audio)}><IconPlay />Play playlist</Button> : null}
      <Button isPending={pending} isDisabled={!home.complete} onPress={() => void run(() => home.save(`list:${listId}`, { position: pinned?.position ?? 1000 + home.rows.length * 10, visible: !pinned, listId }))}>{pinned ? "Unpin from Home" : "Pin to Home"}</Button>
      {saved.data?.savedView ? <Button onPress={() => setEditing(true)}>Edit rules</Button> : null}
      {current.kind === "MANUAL" ? <Button onPress={() => setRenaming(true)}>Rename</Button> : null}
      <Button variant="danger-soft" onPress={() => setConfirm(true)}>Delete list</Button>
    </div>} />
    {home.error ? <ErrorState error={home.error} onRetry={() => void home.refetch()} /> : null}
    {current.kind === "SMART" ? saved.data?.savedView ? <SmartResults view={saved.data.savedView} /> : saved.error ? <ErrorState error={saved.error} onRetry={() => void saved.refetch()} /> : <p role="status">Loading rules…</p> : <>
      {entries.error ? <ErrorState error={entries.error} onRetry={() => void entries.refetch()} /> : null}
      <DataTable<Entry> id={`personal:${listId}`} label={current.name} height="fill" columns={columns} rows={entries.rows} getRowId={(entry) => entry.id} getRowLabel={(entry) => entryPresentation(entry)?.title ?? "Unavailable item"}
        loading={entries.loading} refetching={entries.fetching && !entries.loading}
        actions={actions} selectable onRowOpen={(entry) => { const href = entryPresentation(entry)?.href; if (href) void navigate({ href }); }} rowOpen="click"
        reorder={{ disabled: pending ? "Saving list" : !entries.complete ? "Load the complete list to reorder" : false, onReorder: (change) => run(() => reorderTo(change.row, change.to)) }}
        noun={{ one: "item", other: "items" }} emptyMessage="Nothing saved yet. Use Save on a title, or the track menu, to add items to this list." />
    </>}
    <ConfirmDialog isOpen={confirm} onOpenChange={setConfirm} title={`Delete ${current.name}?`} description="The list is removed. Media and download settings stay as they are." confirmLabel="Delete list" destructive isPending={deleting} onConfirm={() => run(async () => { const { data } = await deleteList({ variables: { id: listId } }); assertSuccess(data?.deletePersonalList, "Could not delete list"); await navigate({ to: "/lists" }); })} />
    {editing && saved.data?.savedView ? <ListEditor open={editing} onOpenChange={setEditing} initial={saved.data.savedView} personalListId={listId} /> : null}
    {renaming ? <RenameList id={listId} name={current.name} onOpenChange={setRenaming} /> : null}
  </div>;
}

function RenameList({ id, name, onOpenChange }: { id: string; name: string; onOpenChange: (open: boolean) => void }) {
  const [update] = useMutation(EntityPersonalListUpdateDocument);
  const form = useAppForm({ ...formBehaviour, defaultValues: { name }, validators: { onDynamic: z.object({ name: z.string().trim().min(1, "Enter a name").max(120) }) }, onSubmit: async ({ value }) => {
    const { data } = await update({ variables: { id, input: { name: value.name.trim() } } }); assertSuccess(data?.updatePersonalList, "Could not rename list"); onOpenChange(false);
  } });
  return <Dialog isOpen onOpenChange={onOpenChange} title="Rename list"><Form form={form} onError={(error) => toast.danger(errorMessage(error))}><FormStack>
    <form.AppField name="name">{(field) => <field.TextField label="Name" required />}</form.AppField>
    <FormActions><form.SubmitButton>Save</form.SubmitButton></FormActions>
  </FormStack></Form></Dialog>;
}
