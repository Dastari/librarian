import { useEffect, useState } from "react";
import { Field } from "glass-ux/field";
import { Input } from "glass-ux/input";
import { Select } from "glass-ux/select";
import { toast } from "glass-ux/toast";

import { Button, Dialog, ErrorState } from "@/components/ui";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { EntityListEntryDeleteDocument, PlaylistMembersDocument } from "@/graphql/generated/graphql";
import { useMutation } from "@apollo/client/react";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { pluralize } from "@/lib/format";

import { playlistCount, usePlaylists, usePlaylistWrites } from "./usePlaylists";
import { entryTarget, targetKey, type PlaylistTarget } from "./types";

const NEW = "__new__";

/**
 * Picks a playlist to add to, or starts one. Takes any number of targets: one title from its page
 * or many from a table's bulk bar. A single target already in the chosen playlist offers to
 * remove it; several skip what the playlist already holds and say how many that was.
 */
export function AddToPlaylistDialog({ open, onOpenChange, targets, title }: { open: boolean; onOpenChange: (open: boolean) => void; targets: readonly PlaylistTarget[]; title: string }) {
  const lists = usePlaylists();
  const { addToPlaylist, createPlaylist } = usePlaylistWrites();
  const [listId, setListId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [pending, setPending] = useState(false);
  const [failure, setFailure] = useState<string>();
  const creating = listId === NEW;
  const members = useCompleteConnection(PlaylistMembersDocument, { listId: listId ?? "" }, (data) => data.listEntries, { skip: !open || !listId || creating });
  const [remove] = useMutation(EntityListEntryDeleteDocument);
  useEffect(() => { if (!open) { setListId(null); setFailure(undefined); setName(""); } }, [open]);
  // With nothing to choose from, the only sensible thing is a new playlist.
  const choices = lists.rows.filter((list) => list.kind !== "SMART");
  useEffect(() => { if (open && lists.complete && !choices.length && listId === null) setListId(NEW); }, [open, lists.complete, choices.length, listId]);

  const single = targets.length === 1 ? targets[0]! : null;
  const existing = single && !creating ? members.rows.find((entry) => { const target = entryTarget(entry); return target && targetKey(target) === targetKey(single); }) : undefined;
  const ready = Boolean(listId) && (creating ? Boolean(name.trim()) : members.complete);

  const run = async (action: () => Promise<void>) => {
    setPending(true); setFailure(undefined);
    try { await action(); } catch (error) { setFailure(errorMessage(error)); } finally { setPending(false); }
  };
  const submit = () => run(async () => {
    if (existing) {
      const { data } = await remove({ variables: { id: existing.id } });
      assertSuccess(data?.deleteListEntry, "Could not remove the item");
      toast.success("Removed from the playlist");
    } else if (creating) {
      const made = await createPlaylist(name, targets);
      toast.success(`Added ${pluralize(made.added, "item")} to ${made.name}`);
    } else {
      const result = await addToPlaylist(listId!, targets);
      const playlist = choices.find((list) => list.id === listId)?.name ?? "the playlist";
      toast.success(result.added ? `Added ${pluralize(result.added, "item")} to ${playlist}${result.alreadyThere ? `, ${result.alreadyThere} already there` : ""}` : `Already in ${playlist}`);
    }
    onOpenChange(false);
  });

  return <Dialog isOpen={open} onOpenChange={onOpenChange} title={`Add ${title} to a playlist`} description="Playlists are only yours. They do not change download settings." isDismissable={!pending}
    footer={<><Button variant="ghost" onPress={() => onOpenChange(false)} isDisabled={pending}>Cancel</Button><Button variant="primary" isPending={pending} isDisabled={!ready} onPress={() => void submit()}>{existing ? "Remove from playlist" : creating ? "Create and add" : "Add"}</Button></>}>
    <form className="flex min-w-0 flex-col gap-4" onSubmit={(event) => { event.preventDefault(); if (ready && !pending) void submit(); }}>
      {lists.error ? <ErrorState error={lists.error} onRetry={() => void lists.refetch()} /> : null}
      <Field label="Playlist" nativeLabel={false}>
        <Select value={listId} onValueChange={setListId} disabled={pending || lists.loading} placeholder="Choose a playlist"
          options={[...choices.map((list) => ({ value: list.id, label: `${list.name} · ${pluralize(playlistCount(list), "item")}` })), { value: NEW, label: "New playlist…" }]} width="full" />
      </Field>
      {creating ? <Field label="Name" className="min-w-0"><Input value={name} onValueChange={setName} required autoFocus disabled={pending} /></Field> : null}
      {existing ? <p className="text-body-sm text-muted">This is already in the playlist.</p> : null}
      {members.error ? <ErrorState error={members.error} onRetry={() => void members.refetch()} /> : null}
      {failure ? <p role="alert" className="text-body-sm text-danger">{failure}</p> : null}
    </form>
  </Dialog>;
}
