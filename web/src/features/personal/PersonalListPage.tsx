import { useMutation } from "@apollo/client/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { useRef, useState } from "react";
import { IconBookmark, IconPlay } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { Form, FormActions, FormStack, formBehaviour, useAppForm } from "glass-ux/form";
import { z } from "zod";
import { Button, ConfirmDialog, DataTable, Dialog, EmptyState, ErrorState, PageHeader, type DataTableColumn } from "@/components/ui";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { usePlayer } from "@/features/player/usePlayer";
import { EntityListEntryDeleteDocument, EntityListEntryUpdateDocument, EntityPersonalListDeleteDocument, EntityPersonalListGetDocument, EntityPersonalListUpdateDocument, EntitySavedViewGetDocument, PersonalEntriesDocument, type PersonalEntryFragment } from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery } from "@/lib/graphql/live/hooks";
import { ListEditor } from "./ListEditor";
import { SmartResults } from "./SmartResults";
import { entryPresentation } from "./types";
import { useHomeShelves } from "./usePersonalLibrary";

const columns: Array<DataTableColumn<PersonalEntryFragment>> = [
  { id: "title", header: "Title", cardRole: "title", value: (entry) => entryPresentation(entry)?.title ?? "Unavailable item", cell: (entry) => {
    const item = entryPresentation(entry);
    return item ? <Link to={item.href as never} className="nav-focus flex min-w-0 items-center gap-3 rounded">
      {item.image ? <img src={item.image} alt="" loading="lazy" className="size-10 shrink-0 rounded object-cover" /> : null}<span className="min-w-0"><span className="block truncate">{item.title}</span><span className="block truncate text-body-sm text-muted">{item.subtitle}</span></span>
    </Link> : <span className="text-muted">Unavailable item</span>;
  } },
];

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
      <DataTable id={`personal:${listId}`} label={current.name} height="fill" columns={columns} rows={entries.rows} getRowId={(entry) => entry.id} isLoading={entries.loading} refetching={entries.fetching} selectable={false} dataSource={ready.current.complete ? "complete" : "server-infinite"} externalBrowserToolbar={!ready.current.complete}
        getSearchText={(entry) => { const item = entryPresentation(entry); return `${item?.title ?? ""} ${item?.subtitle ?? ""}`; }}
        emptyState={<EmptyState icon={IconBookmark} title="Nothing saved yet" description="Use Save on a title, or the track menu, to add items to this list." />}
        rowActions={[
          { key: "up", label: "Move up", hidden: (entry) => entries.rows[0]?.id === entry.id, disabledReason: () => pending ? "Saving list" : !entries.complete ? "Load the complete list to reorder" : false, onAction: (entry) => run(() => reorder(entry, -1)) },
          { key: "down", label: "Move down", hidden: (entry) => entries.rows.at(-1)?.id === entry.id, disabledReason: () => pending ? "Saving list" : !entries.complete ? "Load the complete list to reorder" : false, onAction: (entry) => run(() => reorder(entry, 1)) },
          { key: "remove", label: "Remove from list", disabledReason: () => pending ? "Saving list" : false, destructive: true, onAction: (entry) => run(async () => { const { data } = await remove({ variables: { id: entry.id } }); assertSuccess(data?.deleteListEntry, "Could not remove item"); await entries.refetch(); }) },
        ]} />
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
