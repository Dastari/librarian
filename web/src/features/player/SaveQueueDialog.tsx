import { useApolloClient } from "@apollo/client/react";
import { Field } from "glass-ux/field";
import { Input } from "glass-ux/input";
import { toast } from "glass-ux/toast";
import { useEffect, useState } from "react";

import { Button, Dialog } from "@/components/ui";
import { useSession } from "@/lib/auth/useSession";
import { errorMessage } from "@/lib/graphql/errors";

import { playlistAdapter, savableItems } from "./playlistAdapter";
import { playerStore } from "./store";

/** Names the queue and saves its entries, in order, as a new playlist. */
export function SaveQueueDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const client = useApolloClient();
  const { user } = useSession();
  const [name, setName] = useState("");
  const [pending, setPending] = useState(false);
  const [failure, setFailure] = useState<string>();
  useEffect(() => {
    if (!open) return;
    setName("Saved queue");
    setFailure(undefined);
  }, [open]);

  const items = open ? playerStore.getSnapshot().queue : [];
  const savable = savableItems(items).length;
  const skipped = items.length - savable;

  const save = async () => {
    if (!user) return;
    setPending(true);
    setFailure(undefined);
    try {
      const result = await playlistAdapter.createFromItems(client, { userId: user.id, name, items });
      toast.success(`Saved ${result.saved} ${result.saved === 1 ? "item" : "items"} to "${name.trim()}"${result.skipped ? `. ${result.skipped} ${result.skipped === 1 ? "item" : "items"} could not be saved.` : ""}`);
      onOpenChange(false);
    } catch (error) {
      setFailure(errorMessage(error));
    } finally {
      setPending(false);
    }
  };

  return (
    <Dialog
      isOpen={open}
      onOpenChange={onOpenChange}
      title="Save queue as playlist"
      description={`${items.length} ${items.length === 1 ? "item" : "items"}, in the order they are queued.`}
      isDismissable={!pending}
      footer={
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)} isDisabled={pending}>Cancel</Button>
          <Button variant="primary" isPending={pending} isDisabled={!name.trim() || savable === 0} onPress={() => void save()}>Save playlist</Button>
        </>
      }
    >
      <form className="flex min-w-0 flex-col gap-3" onSubmit={(event) => { event.preventDefault(); if (name.trim() && savable > 0 && !pending) void save(); }}>
        <Field label="Name"><Input value={name} onValueChange={setName} required disabled={pending} autoFocus /></Field>
        {skipped > 0 ? <p className="text-body-sm text-muted">{savable === 0 ? "Playlists cannot hold episodes or chapters yet, so nothing in this queue can be saved." : `${skipped} ${skipped === 1 ? "item" : "items"} (episodes and chapters) cannot be saved to a playlist yet and will be left out.`}</p> : null}
        {failure ? <p role="alert" className="text-body-sm text-danger">{failure}</p> : null}
      </form>
    </Dialog>
  );
}
