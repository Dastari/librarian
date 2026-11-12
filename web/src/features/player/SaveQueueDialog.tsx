import { Field } from "glass-ux/field";
import { Input } from "glass-ux/input";
import { toast } from "glass-ux/toast";
import { useEffect, useState } from "react";

import { Button, Dialog } from "@/components/ui";
import { playItemTargets } from "@/features/personal/playlistPlayback";
import { useCreatePlaylistFromTargets } from "@/features/personal/usePlaylists";
import { errorMessage } from "@/lib/graphql/errors";

import { playerStore } from "./store";

/** Names the queue and saves its entries, in order, as a new playlist: every track, chapter, movie and episode. */
export function SaveQueueDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const createPlaylist = useCreatePlaylistFromTargets();
  const [name, setName] = useState("");
  const [pending, setPending] = useState(false);
  const [failure, setFailure] = useState<string>();
  useEffect(() => {
    if (!open) return;
    setName("Saved queue");
    setFailure(undefined);
  }, [open]);

  const items = open ? playerStore.getSnapshot().queue : [];
  // A playlist holds each thing once, so a repeat in the queue is saved once.
  const targets = playItemTargets(items);
  const repeats = items.length - targets.length;

  const save = async () => {
    setPending(true);
    setFailure(undefined);
    try {
      const result = await createPlaylist(name, targets);
      toast.success(`Saved ${result.added} ${result.added === 1 ? "item" : "items"} to "${name.trim()}"`);
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
          <Button variant="primary" isPending={pending} isDisabled={!name.trim() || targets.length === 0} onPress={() => void save()}>Save playlist</Button>
        </>
      }
    >
      <form className="flex min-w-0 flex-col gap-3" onSubmit={(event) => { event.preventDefault(); if (name.trim() && targets.length > 0 && !pending) void save(); }}>
        <Field label="Name"><Input value={name} onValueChange={setName} required disabled={pending} autoFocus /></Field>
        {repeats > 0 ? <p className="text-body-sm text-muted">{`${repeats} ${repeats === 1 ? "repeat is" : "repeats are"} saved once, because a playlist holds each item once.`}</p> : null}
        {failure ? <p role="alert" className="text-body-sm text-danger">{failure}</p> : null}
      </form>
    </Dialog>
  );
}
